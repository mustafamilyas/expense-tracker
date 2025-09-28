use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use teloxide::types::ParseMode;
use teloxide::{prelude::*, types::Message as TgMessage};
use tracing::info;
use uuid::Uuid;

use crate::commands::report::ReportCommand;
use crate::commands::{
    base::Command, expense::ExpenseCommand, expense_edit::ExpenseEditCommand,
    history::HistoryCommand,
};
use crate::config::Config;
use crate::lang::Lang;
use crate::middleware::tier::check_tier_limit;
use crate::reports::MonthlyReportGenerator;
use crate::repos::{
    budget::{BudgetRepo, CreateBudgetDbPayload},
    category::{CategoryRepo, CreateCategoryDbPayload},
    chat_bind_request::{ChatBindRequestRepo, CreateChatBindRequestDbPayload},
    chat_binding::ChatBindingRepo,
    expense_entry::{CreateExpenseEntryDbPayload, ExpenseEntryRepo},
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
            info!("Received message in chat {}: {}", chat_id, text);

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
                            self.handle_category_command(msg.chat.id, &binding, &mut tx)
                                .await?;
                        }
                        "/category-add" => {
                            self.handle_category_add_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/category-edit" => {
                            self.handle_category_edit_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/category-alias" => {
                            self.handle_category_alias_command(
                                msg.chat.id,
                                text,
                                &binding,
                                &mut tx,
                            )
                            .await?;
                        }
                        "/command" => {
                            self.handle_command_list_command(msg.chat.id).await?;
                        }
                        "/budget" => {
                            self.handle_budget_command(msg.chat.id, &binding, &mut tx)
                                .await?;
                        }
                        "/budget-add" => {
                            self.handle_budget_add_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/budget-edit" => {
                            self.handle_budget_edit_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/budget-remove" => {
                            self.handle_budget_remove_command(msg.chat.id, text, &binding, &mut tx)
                                .await?;
                        }
                        "/generate-report" => {
                            self.handle_generate_report_command(msg.chat.id, &binding, &mut tx)
                                .await?;
                        }
                        "/subscription" => {
                            self.handle_subscription_command(msg.chat.id, &binding, &mut tx)
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
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all categories for the group
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;

        if categories.is_empty() {
            let response = "No categories found for this group.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let mut response = "Categories:\n\n".to_string();

        for category in categories {
            response.push_str(&format!("📁 {}\n", category.name));

            // Check if category has an alias
            if let Some(alias) = &category.alias {
                response.push_str("   Alias: ");
                response.push_str(alias);
                response.push_str("\n");
            }
            response.push_str("\n");
        }

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_category_add_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse the command: /category-add [category_name]
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 2 {
            let response = "Usage: /category-add [category_name]";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let category_name = parts[1..].join(" ").trim().to_string();
        if category_name.is_empty() {
            let response = "Category name cannot be empty.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Check if category already exists
        let existing_categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        if existing_categories
            .iter()
            .any(|c| c.name.to_lowercase() == category_name.to_lowercase())
        {
            let response = format!("Category '{}' already exists.", category_name);
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Get user's subscription for tier checking
        let subscription = SubscriptionRepo::get_by_user(tx, binding.bound_by).await?;
        let current_categories = existing_categories.len() as i32;
        check_tier_limit(&subscription, "categories_per_group", current_categories)?;

        // Create new category
        let new_category = CategoryRepo::create(
            tx,
            CreateCategoryDbPayload {
                group_uid: binding.group_uid,
                name: category_name.clone(),
                description: None,
                alias: None,
            },
        )
        .await?;

        // Check if near limit and add upgrade warning
        let limits = subscription.get_tier().limits();
        let mut response = format!("✅ Category '{}' added successfully!", new_category.name);

        if limits.is_near_limit(current_categories + 1, limits.max_categories_per_group) {
            let percentage = ((current_categories + 1) * 100) / limits.max_categories_per_group;
            let suggested_tier = SubscriptionTier::Personal;

            response.push_str(&format!(
                "\n\n⚠️ You're at {}% of your category limit ({}/{}).\n\
                💡 Upgrade to {} for ${:.2}/month for more categories!",
                percentage,
                current_categories + 1,
                limits.max_categories_per_group,
                suggested_tier.display_name(),
                suggested_tier.price()
            ));
        }

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_category_edit_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse the command: /category-edit [old_name] [new_name]
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 3 {
            let response = "Usage: /category-edit [current_name] [new_name]";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let current_name = parts[1].to_string();
        let new_name = parts[2..].join(" ").trim().to_string();

        if new_name.is_empty() {
            let response = "New category name cannot be empty.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Find the category to update
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        let category_to_update = categories
            .into_iter()
            .find(|c| c.name.to_lowercase() == current_name.to_lowercase());

        match category_to_update {
            Some(category) => {
                // Update the category
                let updated_category = CategoryRepo::update(
                    tx,
                    category.uid,
                    crate::repos::category::UpdateCategoryDbPayload {
                        name: Some(new_name.clone()),
                        description: None,
                        alias: None,
                    },
                )
                .await?;

                let response = format!(
                    "✅ Category '{}' updated to '{}'!",
                    category.name, updated_category.name
                );
                self.bot.send_message(chat_id, response).await?;
            }
            None => {
                let response = format!("Category '{}' not found.", current_name);
                self.bot.send_message(chat_id, response).await?;
            }
        }

        Ok(())
    }

    async fn handle_category_alias_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse the command: /category-alias [alias] [category_name]
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 3 {
            let response = "Usage: /category-alias [alias] [category_name]";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let alias = parts[1].to_string();
        let category_name = parts[2..].join(" ").trim().to_string();

        if alias.is_empty() || category_name.is_empty() {
            let response = "Alias and category name cannot be empty.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Find the category
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        let category = categories
            .into_iter()
            .find(|c| c.name.to_lowercase() == category_name.to_lowercase());

        match category {
            Some(cat) => {
                // Check if category already has this alias
                if cat.alias.as_ref().map(|a| a.to_lowercase()) == Some(alias.to_lowercase()) {
                    let response = self.lang.get_with_vars(
                        "TELEGRAM__CATEGORY_ALIAS_EXISTS",
                        HashMap::from([
                            ("alias".to_string(), alias.clone()),
                            ("category".to_string(), cat.name.clone()),
                        ]),
                    );
                    self.bot.send_message(chat_id, response).await?;
                    return Ok(());
                }

                // Update category with new alias
                CategoryRepo::update(
                    tx,
                    cat.uid,
                    crate::repos::category::UpdateCategoryDbPayload {
                        name: None,
                        description: None,
                        alias: Some(alias.clone()),
                    },
                )
                .await?;

                let response = self.lang.get_with_vars(
                    "TELEGRAM__CATEGORY_ALIAS_ADDED",
                    HashMap::from([
                        ("alias".to_string(), alias),
                        ("category".to_string(), cat.name),
                    ]),
                );
                self.bot.send_message(chat_id, response).await?;
            }
            None => {
                let response = self.lang.get_with_vars(
                    "TELEGRAM__CATEGORY_NOT_FOUND",
                    HashMap::from([("category".to_string(), category_name.to_string())]),
                );
                self.bot.send_message(chat_id, response).await?;
            }
        }

        Ok(())
    }

    async fn handle_command_list_command(
        &self,
        chat_id: ChatId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let commands = vec![
            "/expense - Add a new expense",
            "/expense-edit - Edit existing expenses",
            "/report - View monthly expense summary",
            "/history - View detailed expense history",
            "/budget - View budget overview",
            "/budget-add - Add a new budget",
            "/budget-edit - Edit budget amount",
            "/budget-remove - Remove a budget",
            "/category - List all categories and aliases",
            "/category-add - Add a new category",
            "/category-edit - Edit a category name",
            "/category-alias - Add an alias for a category",
            "/generate-report - Generate monthly PDF report",
            "/subscription - View subscription status and limits",
            "/command - Show this command list",
            "/login - Bind this chat to your expense group",
        ];

        let response = format!("Available Commands:\n\n{}", commands.join("\n"));
        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_budget_command(
        &self,
        chat_id: ChatId,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all budgets for the group
        let budgets = BudgetRepo::list_by_group(tx, binding.group_uid).await?;

        if budgets.is_empty() {
            let response = "No budgets configured for this group.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Get group members to calculate date range
        let group_members = GroupMemberRepo::list(tx)
            .await?
            .into_iter()
            .filter(|m| m.group_uid == binding.group_uid)
            .collect::<Vec<_>>();

        if group_members.is_empty() {
            let response = "No group members found.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Use first user's start_over_date for date range
        let first_user = UserRepo::get(tx, group_members[0].user_uid).await?;
        let (start_date, end_date) = self.calculate_month_range(first_user.start_over_date);

        let mut response = "Budget Overview:\n\n".to_string();

        for budget in budgets {
            // Get category name
            let category = CategoryRepo::get(tx, budget.category_uid).await?;
            let category_name = category.name;

            // Calculate spending for this category in current period
            let spending = sqlx::query(
                r#"
                SELECT COALESCE(SUM(e.price), 0) as total_spent
                FROM expense_entries e
                WHERE e.group_uid = $1
                  AND e.category_uid = $2
                  AND e.created_at >= $3
                  AND e.created_at < $4
                "#,
            )
            .bind(binding.group_uid)
            .bind(budget.category_uid)
            .bind(start_date)
            .bind(end_date)
            .fetch_one(tx.as_mut())
            .await?;

            let spent: f64 = spending.get("total_spent");
            let remaining = budget.amount - spent;
            let percentage = if budget.amount > 0.0 {
                (spent / budget.amount) * 100.0
            } else {
                0.0
            };

            let status = if remaining < 0.0 {
                "❌ Over budget"
            } else if percentage >= 80.0 {
                "⚠️ Near limit"
            } else {
                "✅ On track"
            };

            response.push_str(&format!(
                "📊 {}\nBudget: Rp. {:.0}\nSpent: Rp. {:.0}\nRemaining: Rp. {:.0}\nUsage: {:.1}%\n{}\n\n",
                category_name,
                budget.amount,
                spent,
                remaining,
                percentage,
                status
            ));
        }

        self.bot.send_message(chat_id, response).await?;
        Ok(())
    }

    async fn handle_budget_add_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse the command: /budget-add [category_name] [amount]
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 3 {
            let response = "Usage: /budget-add [category_name] [amount]";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let category_name = parts[1].to_string();
        let amount_str = parts[2];

        // Parse amount
        let amount: f64 = match amount_str.parse() {
            Ok(a) => a,
            Err(_) => {
                let response = "Invalid amount format. Please use a number.";
                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        if amount <= 0.0 {
            let response = "Budget amount must be greater than 0.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Find category
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        let category = categories
            .into_iter()
            .find(|c| c.name.to_lowercase() == category_name.to_lowercase());

        match category {
            Some(cat) => {
                // Check if budget already exists for this category
                let existing_budgets = BudgetRepo::list_by_group(tx, binding.group_uid).await?;
                if existing_budgets.iter().any(|b| b.category_uid == cat.uid) {
                    let response = format!(
                        "Budget already exists for category '{}'. Use /budget-edit to modify it.",
                        cat.name
                    );
                    self.bot.send_message(chat_id, response).await?;
                    return Ok(());
                }

                // Get user's subscription for tier checking
                let subscription = SubscriptionRepo::get_by_user(tx, binding.bound_by).await?;
                let current_budgets = existing_budgets.len() as i32;
                check_tier_limit(&subscription, "budgets_per_group", current_budgets)?;

                // Create budget
                let budget = BudgetRepo::create(
                    tx,
                    CreateBudgetDbPayload {
                        group_uid: binding.group_uid,
                        category_uid: cat.uid,
                        amount,
                        period_year: None, // Monthly budget by default
                        period_month: None,
                    },
                )
                .await?;

                // Check if near limit and add upgrade warning
                let limits = subscription.get_tier().limits();
                let mut response = format!(
                    "✅ Budget of Rp. {:.0} added for category '{}'!",
                    budget.amount, cat.name
                );

                if limits.is_near_limit(current_budgets + 1, limits.max_budgets_per_group) {
                    let percentage = ((current_budgets + 1) * 100) / limits.max_budgets_per_group;
                    let suggested_tier = SubscriptionTier::Personal;

                    response.push_str(&format!(
                        "\n\n⚠️ You're at {}% of your budget limit ({}/{}).\n\
                        💡 Upgrade to {} for ${:.2}/month for more budgets!",
                        percentage,
                        current_budgets + 1,
                        limits.max_budgets_per_group,
                        suggested_tier.display_name(),
                        suggested_tier.price()
                    ));
                }

                self.bot.send_message(chat_id, response).await?;
            }
            None => {
                let response = format!(
                    "Category '{}' not found. Use /category to see available categories.",
                    category_name
                );
                self.bot.send_message(chat_id, response).await?;
            }
        }

        Ok(())
    }

    async fn handle_budget_edit_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse the command: /budget-edit [category_name] [new_amount]
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 3 {
            let response = "Usage: /budget-edit [category_name] [new_amount]";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let category_name = parts[1].to_string();
        let amount_str = parts[2];

        // Parse amount
        let new_amount: f64 = match amount_str.parse() {
            Ok(a) => a,
            Err(_) => {
                let response = "Invalid amount format. Please use a number.";
                self.bot.send_message(chat_id, response).await?;
                return Ok(());
            }
        };

        if new_amount <= 0.0 {
            let response = "Budget amount must be greater than 0.";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        // Find category
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        let category = categories
            .into_iter()
            .find(|c| c.name.to_lowercase() == category_name.to_lowercase());

        match category {
            Some(cat) => {
                // Find existing budget
                let budgets = BudgetRepo::list_by_group(tx, binding.group_uid).await?;
                let budget = budgets.into_iter().find(|b| b.category_uid == cat.uid);

                match budget {
                    Some(b) => {
                        // Update budget
                        let updated_budget = BudgetRepo::update(
                            tx,
                            b.uid,
                            crate::repos::budget::UpdateBudgetDbPayload {
                                amount: Some(new_amount),
                                period_year: None,
                                period_month: None,
                            },
                        )
                        .await?;

                        let response = format!(
                            "✅ Budget for '{}' updated from Rp. {:.0} to Rp. {:.0}!",
                            cat.name, b.amount, updated_budget.amount
                        );
                        self.bot.send_message(chat_id, response).await?;
                    }
                    None => {
                        let response = format!(
                            "No budget found for category '{}'. Use /budget-add to create one.",
                            cat.name
                        );
                        self.bot.send_message(chat_id, response).await?;
                    }
                }
            }
            None => {
                let response = format!(
                    "Category '{}' not found. Use /category to see available categories.",
                    category_name
                );
                self.bot.send_message(chat_id, response).await?;
            }
        }

        Ok(())
    }

    async fn handle_budget_remove_command(
        &self,
        chat_id: ChatId,
        text: &str,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Parse the command: /budget-remove [category_name]
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 2 {
            let response = "Usage: /budget-remove [category_name]";
            self.bot.send_message(chat_id, response).await?;
            return Ok(());
        }

        let category_name = parts[1].to_string();

        // Find category
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        let category = categories
            .into_iter()
            .find(|c| c.name.to_lowercase() == category_name.to_lowercase());

        match category {
            Some(cat) => {
                // Find existing budget
                let budgets = BudgetRepo::list_by_group(tx, binding.group_uid).await?;
                let budget = budgets.into_iter().find(|b| b.category_uid == cat.uid);

                match budget {
                    Some(b) => {
                        // Delete budget
                        BudgetRepo::delete(tx, b.uid).await?;
                        let response =
                            format!("✅ Budget for '{}' removed successfully!", cat.name);
                        self.bot.send_message(chat_id, response).await?;
                    }
                    None => {
                        let response = format!("No budget found for category '{}'.", cat.name);
                        self.bot.send_message(chat_id, response).await?;
                    }
                }
            }
            None => {
                let response = format!(
                    "Category '{}' not found. Use /category to see available categories.",
                    category_name
                );
                self.bot.send_message(chat_id, response).await?;
            }
        }

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

            // Generate report
            let report_generator = MonthlyReportGenerator::new(self.db_pool.clone());
            match report_generator
                .generate_monthly_report(binding.group_uid, user.uid, user.start_over_date)
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

    async fn handle_subscription_command(
        &self,
        chat_id: ChatId,
        binding: &crate::repos::chat_binding::ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get user's subscription
        let subscription = SubscriptionRepo::get_by_user(tx, binding.bound_by).await?;
        let limits = subscription.get_tier().limits();

        // Get current usage
        let usage = UserUsageRepo::calculate_current_usage(tx, binding.bound_by).await?;

        let status = if subscription.status == "active" {
            "✅ Active"
        } else {
            "❌ Inactive"
        };

        let response = format!(
            "📊 Subscription Status\n\n\
            Current Tier: {}\n\
            Status: {}\n\
            Price: ${:.2}/month\n\n\
            📈 Current Usage:\n\
            • Groups: {}/{}\n\
            • Total Members: {}\n\
            • Expenses This Month: {}/{}\n\n\
            🎯 Limits:\n\
            • Max Categories per Group: {}\n\
            • Max Budgets per Group: {}\n\
            • Data Retention: {} days\n\
            • Advanced Reports: {}\n\
            • Data Export: {}\n\
            • Priority Support: {}\n\n\
            💡 Upgrade Options:\n\
            • Personal: $4.99/month\n\
            • Family: $9.99/month\n\
            • Team: $19.99/month\n\
            • Enterprise: $49.99/month",
            subscription.get_tier().display_name(),
            status,
            subscription.get_tier().price(),
            usage.groups_count,
            if limits.max_groups == -1 {
                "∞".to_string()
            } else {
                limits.max_groups.to_string()
            },
            usage.total_members,
            usage.total_expenses,
            if limits.max_expenses_per_month == -1 {
                "∞".to_string()
            } else {
                limits.max_expenses_per_month.to_string()
            },
            if limits.max_categories_per_group == -1 {
                "∞".to_string()
            } else {
                limits.max_categories_per_group.to_string()
            },
            if limits.max_budgets_per_group == -1 {
                "∞".to_string()
            } else {
                limits.max_budgets_per_group.to_string()
            },
            limits.data_retention_days,
            if limits.advanced_reports {
                "✅"
            } else {
                "❌"
            },
            if limits.export_data { "✅" } else { "❌" },
            if limits.priority_support {
                "✅"
            } else {
                "❌"
            }
        );

        self.bot.send_message(chat_id, response).await?;
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
