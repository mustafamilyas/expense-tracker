use chrono::{Utc, Timelike, Datelike};
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use sqlx::PgPool;

use crate::repos::{
    user::UserRepo,
    expense_group::ExpenseGroupRepo,
    expense_group_member::GroupMemberRepo,
    chat_binding::ChatBindingRepo,
    subscription::UserUsageRepo,
};
use crate::messengers::MessengerManager;
use super::monthly_report::MonthlyReportGenerator;

pub struct ReportScheduler {
    db_pool: PgPool,
    messenger_manager: Arc<MessengerManager>,
    report_generator: MonthlyReportGenerator,
}

impl ReportScheduler {
    pub fn new(
        db_pool: PgPool,
        messenger_manager: Arc<MessengerManager>,
    ) -> Self {
        let report_generator = MonthlyReportGenerator::new(db_pool.clone());
        Self {
            db_pool,
            messenger_manager,
            report_generator,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let sched = JobScheduler::new().await?;

        // Schedule job to run every hour to check for reports to send
        let db_pool = self.db_pool.clone();
        let messenger_manager = self.messenger_manager.clone();
        let report_generator = self.report_generator.clone();

        let report_job = Job::new_async("0 * * * * *", move |_, _| {
            let db_pool = db_pool.clone();
            let messenger_manager = messenger_manager.clone();
            let report_generator = report_generator.clone();

            Box::pin(async move {
                if let Err(e) = Self::check_and_send_reports(
                    db_pool,
                    messenger_manager,
                    report_generator,
                ).await {
                    tracing::error!("Error sending monthly reports: {:?}", e);
                }
            })
        })?;

        // Schedule job to run daily at 2 AM to update usage statistics
        let db_pool_usage = self.db_pool.clone();
        let usage_job = Job::new_async("0 2 * * * *", move |_, _| {
            let db_pool = db_pool_usage.clone();

            Box::pin(async move {
                if let Err(e) = Self::update_usage_statistics(db_pool).await {
                    tracing::error!("Error updating usage statistics: {:?}", e);
                }
            })
        })?;

        sched.add(report_job).await?;
        sched.add(usage_job).await?;
        sched.start().await?;

        tracing::info!("Report scheduler and usage tracker started");
        Ok(())
    }

    async fn check_and_send_reports(
        db_pool: PgPool,
        messenger_manager: Arc<MessengerManager>,
        report_generator: MonthlyReportGenerator,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = db_pool.begin().await?;

        // Get all users
        let groups = ExpenseGroupRepo::list(&mut tx).await?;

        for group in groups {
            // Check if it's time to send the monthly report for this group
            if Self::should_send_report(group.start_over_date) {
                // Get group members
                let group_members = GroupMemberRepo::list(&mut tx).await?;
                let current_group_members: Vec<_> = group_members
                    .iter()
                    .filter(|gm| gm.group_uid == group.uid)
                    .collect();

                for group_member in current_group_members {
                    // Check if group has active chat binding
                    let chat_bindings = ChatBindingRepo::list(&mut tx).await?;
                    let active_binding = chat_bindings
                        .iter()
                        .find(|cb| cb.group_uid == group_member.group_uid && cb.status == "active");

                    if let Some(binding) = active_binding {
                        // Generate and send report
                        match report_generator.generate_monthly_report(
                            group_member.group_uid,
                            group_member.user_uid,
                            group.start_over_date,
                        ).await {
                            Ok(_pdf_bytes) => {
                                let _filename = format!(
                                    "monthly_report_{}_{}.pdf",
                                    group_member.user_uid,
                                    Utc::now().format("%Y_%m")
                                );

                                let message = format!(
                                    "📊 Your monthly expense report for {} is ready!",
                                    Utc::now().format("%B %Y")
                                );

                                // Send PDF via Telegram
                                if let Err(e) = messenger_manager.send_message(
                                    &binding.platform,
                                    &binding.p_uid,
                                    &message,
                                ).await {
                                    tracing::error!("Failed to send monthly report message: {:?}", e);
                                }

                                // Note: In a real implementation, you'd need to modify the messenger
                                // to support sending files/documents. For now, we'll just send the message.
                            }
                            Err(e) => {
                                tracing::error!("Failed to generate monthly report for user {}: {:?}", group_member.user_uid, e);
                            }
                        }
                    }
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn update_usage_statistics(
        db_pool: PgPool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = db_pool.begin().await?;

        // Get all users
        let users = UserRepo::list(&mut tx).await?;
        let user_count = users.len();

        for user in users {
            // Calculate current usage for this user
            match UserUsageRepo::calculate_current_usage(&mut tx, user.uid).await {
                Ok(usage_payload) => {
                    // Create or update usage record
                    if let Err(e) = UserUsageRepo::create_or_update(&mut tx, usage_payload).await {
                        tracing::error!("Failed to update usage for user {}: {:?}", user.uid, e);
                    } else {
                        tracing::debug!("Updated usage statistics for user {}", user.uid);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to calculate usage for user {}: {:?}", user.uid, e);
                }
            }
        }

        tx.commit().await?;
        tracing::info!("Usage statistics updated for {} users", user_count);
        Ok(())
    }

    fn should_send_report(start_over_date: i16) -> bool {
        let now = Utc::now();
        let current_day = now.day() as i16;
        let current_hour = now.hour();

        // Send report on the start_over_date at 9 AM
        current_day == start_over_date && current_hour == 9
    }
}