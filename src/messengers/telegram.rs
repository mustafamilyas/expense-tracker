use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use teloxide::{prelude::*, types::Message as TgMessage};
use tracing::info;
use uuid::Uuid;

use crate::commands::report::ReportCommand;
use crate::commands::{
    category::CategoryCommand, category_edit::CategoryEditCommand, expense::ExpenseCommand,
    expense_edit::ExpenseEditCommand, help::HelpCommand, history::HistoryCommand,
};
use crate::config::Config;
use crate::lang::Lang;
use crate::middleware::tier::check_tier_limit;
use crate::reports::MonthlyReportGenerator;
use crate::repos::{
    budget::{BudgetRepo, CreateBudgetDbPayload},
    category::CategoryRepo,
    chat_bind_request::{ChatBindRequestRepo, CreateChatBindRequestDbPayload},
    chat_binding::ChatBindingRepo,
    expense_group::ExpenseGroupRepo,
    expense_group_member::GroupMemberRepo,
    subscription::{SubscriptionRepo, UserUsageRepo},
    user::UserRepo,
};
use crate::types::SubscriptionTier;

use super::Messenger;

pub struct TelegramMessenger {
    config: Config,
    bot: Bot,
    db_pool: PgPool,
    lang: Lang,
}

impl TelegramMessenger {
    pub fn new(config: &Config, db_pool: PgPool) -> Self {
        Self {
            config: config.clone(),
            bot: Bot::new(config.telegram_bot_token.clone()),
            db_pool,
            lang: Lang::from_json("id"),
        }
    }

    async fn send_message(
        &self,
        chat_id: ChatId,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.bot.send_message(chat_id, text).await?;
        Ok(())
    }

    async fn handle_message(
        &self,
        msg: TgMessage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let chat_id = msg.chat.id.to_string();
        let _user_id = msg
            .from
            .clone()
            .map(|u| u.id.to_string())
            .unwrap_or_default();

        if let Some(text) = msg.text() {
            // Check if chat is bound
            let mut tx = self.db_pool.begin().await?;
            let binding = ChatBindingRepo::list(&mut tx)
                .await?
                .into_iter()
                .find(|b| b.platform == "telegram" && b.p_uid == chat_id && b.status == "active");

            match binding {
                Some(binding) => {
                    let command = text.split_whitespace().next().unwrap_or("");
                    match command {
                        "/expense" => {
                            self.handle_expense_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/expense-edit" => {
                            self.handle_expense_edit_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/report" => {
                            self.handle_report_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/history" => {
                            self.handle_history_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/category" => {
                            self.handle_category_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/category-edit" => {
                            self.handle_category_edit_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/help" => {
                            self.handle_help_command(msg.chat.id, &binding, &mut tx)
                                .await?;
                        }
                        _ => {
                            // do nothing
                            // TODO: maybe track unknown commands later
                        }
                    }
                }
                None => {
                    // Chat not bound, handle binding request
                    if text.trim() == "/login" {
                        // Create bind request
                        let nonce = Uuid::new_v4().to_string();
                        let expires_at = Utc::now() + Duration::hours(1);

                        let request = ChatBindRequestRepo::create(
                            &mut tx,
                            CreateChatBindRequestDbPayload {
                                platform: "telegram".to_string(),
                                p_uid: chat_id.clone(),
                                nonce: nonce.clone(),
                                user_uid: None,
                                expires_at,
                            },
                        )
                        .await?;

                        let bind_url = format!("{}/{}", self.config.chat_bind_url, request.id);
                        let response = self.lang.get_with_vars(
                            "TELEGRAM__SIGN_IN_REQUEST",
                            HashMap::from([("link".to_string(), bind_url)]),
                        );

                        self.send_message(msg.chat.id, &response).await?;
                    } else {
                        let response = self.lang.get("TELEGRAM__CHAT_NOT_BOUND");
                        self.bot.send_message(msg.chat.id, response).await?;
                    }
                }
            }

            tx.commit().await?;
        }
        Ok(())
    }

    async fn handle_expense_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match ExpenseCommand::run(text, binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error handling expense command: {}", e);
                let mut response = e.to_string();

                response.push_str("\n-----\n");
                response.push_str(&self.lang.get("MESSENGER__ENTRY_HELP"));

                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_report_command(
        &self,
        chat_id: ChatId,
        raw_message: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match ReportCommand::run(raw_message, binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error generating report: {}", e);
                let response = e.to_string();
                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_history_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match HistoryCommand::run(text, binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error handling history command: {}", e);
                let mut response = e.to_string();

                response.push_str("\n-----\n");
                response.push_str("Format:\n/history\n/history YYYY-MM-DD\n/history YYYY-MM-DD YYYY-MM-DD\n\nContoh:\n/history\n/history 2025-09-01\n/history 2025-09-01 2025-09-03");

                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        // Truncate if too long for Telegram
        let final_response = if response.len() > 4000 {
            let mut truncated = response.chars().take(3950).collect::<String>();
            truncated.push_str("...\n\n(Message truncated due to length)");
            truncated
        } else {
            response
        };

        self.bot.send_message(chat_id, final_response).await?;
        Ok(())
    }

    async fn handle_category_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match CategoryCommand::run(text, binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error handling category command: {}", e);
                let mut response = e.to_string();
                response.push_str("\n-----\n");
                response.push_str("Format:\n/category\n\nMenampilkan semua kategori dan alias yang tersedia untuk grup ini.");

                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        // Truncate if too long for Telegram
        let final_response = if response.len() > 4000 {
            let mut truncated = response.chars().take(3950).collect::<String>();
            truncated.push_str("...\n\n(Message truncated due to length)");
            truncated
        } else {
            response
        };

        self.bot.send_message(chat_id, final_response).await?;
        Ok(())
    }

    async fn handle_category_edit_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match CategoryEditCommand::run(text, binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error handling category edit command: {}", e);
                let mut response = e.to_string();
                response.push_str("\n-----\n");
                response.push_str("Format:\n/category-edit\n[id]\n[name]=[alias1, alias2, ...]\n\nContoh:\n/category-edit\n123e4567-e89b-12d3-a456-426614174000\nMakanan=makan, food");

                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_help_command(
        &self,
        chat_id: ChatId,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match HelpCommand::run("/help", binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error handling help command: {}", e);
                format!("Error: {}", e)
            }
        };

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_generate_report_command(
        &self,
        chat_id: ChatId,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get the user who bound this chat
        let group_members = GroupMemberRepo::list(tx).await?;
        let user_member = group_members
            .into_iter()
            .find(|gm| gm.group_uid == binding.group_uid);

        if let Some(member) = user_member {
            let user = UserRepo::get(tx, member.user_uid).await?;
            let group = ExpenseGroupRepo::get(tx, binding.group_uid).await?;

            // Generate report
            let report_generator = MonthlyReportGenerator::new(self.db_pool.clone());
            match report_generator
                .generate_monthly_report(binding.group_uid, user.uid, group.start_over_date)
                .await
            {
                Ok(pdf_bytes) => {
                    let response = format!(
                        "📊 Monthly report generated successfully!\nReport size: {} bytes\n\nNote: PDF file sending is not yet implemented in this demo.",
                        pdf_bytes.len()
                    );
                    self.bot.send_message(chat_id, response).await?;
                }
                Err(e) => {
                    let response = format!("❌ Failed to generate report: {:?}", e);
                    self.bot.send_message(chat_id, response).await?;
                }
            }
        } else {
            let response = "No user found for this chat binding.";
            self.bot.send_message(chat_id, response).await?;
        }

        Ok(())
    }

    async fn handle_expense_edit_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let response = match ExpenseEditCommand::run(text, binding, tx, &self.lang).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error handling expense edit command: {}", e);
                let mut response = e.to_string();

                response.push_str("\n-----\n");
                response.push_str("Format:\n/expense-edit\n[id]\n[nama],[harga],[kategori]\n\nContoh:\n/expense-edit\n123e4567-e89b-12d3-a456-426614174000\nNasi Padang,10000,Makanan");

                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    fn calculate_month_range(
        &self,
        start_over_date: i16,
    ) -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
        let now = Utc::now();
        let current_year = now.year();
        let current_month = now.month();

        // Calculate the start date based on start_over_date
        let start_day = start_over_date as u32;
        let mut start_date = if current_month == 1 {
            // January - go back to previous year
            NaiveDate::from_ymd_opt(current_year - 1, 12, start_day)
        } else {
            NaiveDate::from_ymd_opt(current_year, current_month - 1, start_day)
        }
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap());

        // If the calculated start date is in the future, use the previous month's start date
        if start_date > now.date_naive() {
            start_date = if current_month == 1 {
                NaiveDate::from_ymd_opt(current_year - 1, 11, start_day)
            } else if current_month == 2 {
                NaiveDate::from_ymd_opt(current_year - 1, 12, start_day)
            } else {
                NaiveDate::from_ymd_opt(current_year, current_month - 2, start_day)
            }
            .unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(current_year, current_month - 1, 1).unwrap()
            });
        }

        let end_date = start_date + Duration::days(30); // Approximate month length

        (
            start_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            end_date.and_hms_opt(23, 59, 59).unwrap().and_utc(),
        )
    }
}

#[async_trait]
impl Messenger for TelegramMessenger {
    async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let chat_id: i64 = chat_id.parse()?;
        self.bot.send_message(ChatId(chat_id), text).await?;
        Ok(())
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot = self.bot.clone();
        let db_pool = self.db_pool.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let handler = Update::filter_message().endpoint(move |bot: Bot, msg: TgMessage| {
                let db_pool = db_pool.clone();
                let config = config.clone();
                async move {
                    let messenger = TelegramMessenger::new(&config, db_pool);
                    if let Err(e) = messenger.handle_message(msg).await {
                        tracing::error!("Error handling message: {:?}", e);
                    }
                    respond(())
                }
            });

            Dispatcher::builder(bot, handler)
                .enable_ctrlc_handler()
                .build()
                .dispatch()
                .await;
        });

        Ok(())
    }

    fn platform(&self) -> &str {
        "telegram"
    }
}
