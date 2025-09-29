use std::collections::HashMap;

use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use sqlx::Row;
use tracing::info;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{
        chat_binding::ChatBinding, expense_group::ExpenseGroupRepo,
        expense_group_member::GroupMemberRepo, user::UserRepo,
    },
    utils::parse_price::format_price,
};

#[derive(Debug, PartialEq)]
pub struct ReportCommand;

impl ReportCommand {
    /*
        Should be in format:
        /report
    */
    fn parse_command(input: &str) -> Result<Self> {
        let input = input.trim();

        if input != Self::get_command() {
            return Err(anyhow::anyhow!("Invalid format: expected only /report"));
        }

        Ok(Self {})
    }

    /*
        Output format:
        Pengeluaran <start_date> -> <end_date>:

        Kategori:
        1. Makanan: Rp. 100.000
        2. Transportasi: Rp. 50.000
        3. Tidak Berkategori: Rp. 25.000

        Total: Rp. 175.000
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let _command = Self::parse_command(raw_message)?;

        // Get expenses for the current month based on each user's start_over_date
        let mut category_totals: HashMap<String, f64> = HashMap::new();
        let mut total_expenses = 0.0;
        let mut earliest_start = Utc::now();
        let mut latest_end = Utc::now() - Duration::days(365); // Far in the past

        let group = ExpenseGroupRepo::get(tx, binding.group_uid).await?;
        let (start_date, end_date) = Self::calculate_month_range(group.start_over_date);
        info!(
            "Calculating report for group {} from {} to {}",
            group.name, start_date, end_date
        );

        // Track the overall date range
        if start_date < earliest_start {
            earliest_start = start_date;
        }
        if end_date > latest_end {
            latest_end = end_date;
        }

        // Query expenses for this user in the current month
        let expenses = sqlx::query(
            r#"
            SELECT e.price::float8 AS price, c.name as category_name
            FROM expense_entries e
            LEFT JOIN categories c ON e.category_uid = c.uid
            WHERE e.group_uid = $1
              AND e.created_at >= $2
              AND e.created_at < $3
            "#,
        )
        .bind(binding.group_uid)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(tx.as_mut())
        .await?;

        for row in expenses {
            let price: f64 = row.get("price");
            let category_name: Option<String> = row.get("category_name");
            let category_name = category_name.unwrap_or_else(|| lang.get("REPORT__UNCATEGORIZED"));
            *category_totals.entry(category_name).or_insert(0.0) += price;
            total_expenses += price;
        }

        if total_expenses == 0.0 {
            return Ok(lang.get("REPORT__NO_EXPENSES"));
        }

        // Format the response
        let mut response = lang.get_with_vars(
            "REPORT__HEADER",
            HashMap::from([
                (
                    "start_date".to_string(),
                    earliest_start.format("%d/%m/%Y").to_string(),
                ),
                (
                    "end_date".to_string(),
                    latest_end.format("%d/%m/%Y").to_string(),
                ),
            ]),
        );

        response.push_str(&lang.get("REPORT__CATEGORY_HEADER"));

        let mut sorted_categories: Vec<_> = category_totals.iter().collect();
        sorted_categories.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap()); // Sort by amount descending

        for (index, (category, amount)) in sorted_categories.iter().enumerate() {
            response.push_str(&lang.get_with_vars(
                "REPORT__CATEGORY_ITEM",
                HashMap::from([
                    ("index".to_string(), (index + 1).to_string()),
                    ("category".to_string(), (*category).clone()),
                    ("amount".to_string(), format_price(**amount)),
                ]),
            ));
        }

        response.push_str(&lang.get_with_vars(
            "REPORT__TOTAL",
            HashMap::from([("total".to_string(), format_price(total_expenses))]),
        ));

        Ok(response)
    }

    /*
     * Calculate the start and end date for the monthly report based on the user's start_over_date
     * For example, if today is 15th June and start_over_date is 10,
     * the range is 10th June to 10th July.
     * If today is 5th June and start_over_date is 10,
     * the range is 10th May to 10th June.
     */
    fn calculate_month_range(
        start_over_date: i16,
    ) -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
        let now = Utc::now();
        let current_year = now.year();
        let current_month = now.month();
        let current_start_over_date =
            NaiveDate::from_ymd_opt(current_year, current_month, start_over_date as u32)
                .unwrap_or_else(|| {
                    NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap()
                });

        let start_date = if current_start_over_date > now.date_naive() {
            // If the start_over_date hasn't occurred yet this month, use last month's date
            if current_month == 1 {
                NaiveDate::from_ymd_opt(current_year - 1, 12, start_over_date as u32)
            } else {
                NaiveDate::from_ymd_opt(current_year, current_month - 1, start_over_date as u32)
            }
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(current_year, current_month - 1, 1).unwrap())
        } else {
            current_start_over_date
        };

        let end_date = if start_date.month() == 12 {
            NaiveDate::from_ymd_opt(start_date.year() + 1, 1, start_over_date as u32)
        } else {
            NaiveDate::from_ymd_opt(
                start_date.year(),
                start_date.month() + 1,
                start_over_date as u32,
            )
        }
        .unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(start_date.year(), start_date.month() + 1, 1).unwrap()
        });

        (
            start_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            end_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
        )
    }
}

impl Command for ReportCommand {
    fn get_command() -> &'static str {
        "/report"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__REPORT_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
