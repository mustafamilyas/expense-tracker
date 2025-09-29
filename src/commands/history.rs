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

#[derive(Debug)]
pub struct HistoryCommand {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

impl HistoryCommand {
    /*
        Should be in format:
        /history (start_date) (end_date)

        Both dates are optional, if not provided, will default to last 3 days
        If only one date is provided, will use that date as start_date and end_date
        Dates should be in format YYYY-MM-DD
        The maximum range is 3 days

        Examples:
        /history
        /history 2023-01-01
        /history 2023-01-01 2023-01-31
    */
    fn parse_command(input: &str) -> Result<Self> {
        let input = input.trim();

        // Should start with /history
        let input = if input.starts_with(Self::get_command()) {
            input[Self::get_command().len()..].trim()
        } else {
            input
        };

        let parts: Vec<&str> = input.split_whitespace().collect();
        let now = Utc::now().date_naive();

        let (start_date, end_date) = match parts.len() {
            0 => {
                // Default to last 3 days
                let end_date = now;
                let start_date = end_date - Duration::days(3);
                (Some(start_date), Some(end_date))
            }
            1 => {
                // Single date provided - use as both start and end
                let date_str = parts[0];
                let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
                    anyhow::anyhow!("Invalid date format: {}. Expected YYYY-MM-DD", date_str)
                })?;
                (Some(date), Some(date))
            }
            2 => {
                // Two dates provided
                let start_str = parts[0];
                let end_str = parts[1];

                let start_date =
                    NaiveDate::parse_from_str(start_str, "%Y-%m-%d").map_err(|_| {
                        anyhow::anyhow!(
                            "Invalid start date format: {}. Expected YYYY-MM-DD",
                            start_str
                        )
                    })?;
                let end_date = NaiveDate::parse_from_str(end_str, "%Y-%m-%d").map_err(|_| {
                    anyhow::anyhow!("Invalid end date format: {}. Expected YYYY-MM-DD", end_str)
                })?;

                // Check maximum range (3 days)
                let days_diff = (end_date - start_date).num_days();
                if days_diff > 3 {
                    return Err(anyhow::anyhow!(
                        "Date range cannot exceed 3 days. Current range: {} days",
                        days_diff
                    ));
                }

                if start_date > end_date {
                    return Err(anyhow::anyhow!("Start date cannot be after end date"));
                }

                (Some(start_date), Some(end_date))
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid format. Use: /history [start_date] [end_date] (dates in YYYY-MM-DD format)"
                ));
            }
        };

        Ok(Self {
            start_date,
            end_date,
        })
    }

    /*
        Output format:

        Pengeluaran <start_date> -> <end_date>:
        [date] [uid]
        [item], Rp. [price], ([category])

        [date] [uid]
        [item], Rp. [price], ([category])

        Total: Rp. [total]

        If no expenses found, return "Tidak ada pengeluaran dalam periode ini."

        Example:
        Pengeluaran 2023-01-01 -> 2023-01-31:
        2023-01-15 123e4567-e89b-12d3-a456-426614174000
        Nasi Padang, Rp. 100000, (Makanan)

        2023-01-20 123e4567-e89b-12d3-a456-426614174001
        Warteg, Rp. 15000, (Makanan)

        2023-01-25 123e4567-e89b-12d3-a456-426614174002
        Ojek Online, Rp. 50000, (Transportasi)

        Total: Rp. 115000
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let command = Self::parse_command(raw_message)?;

        // Get the expense group to determine the date range
        let group = ExpenseGroupRepo::get(tx, binding.group_uid).await?;

        let (default_start, default_end) = Self::calculate_month_range(group.start_over_date);

        // Use provided dates or fall back to monthly range
        let start_date = command
            .start_date
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
            .unwrap_or(default_start);
        let end_date = command
            .end_date
            .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc())
            .unwrap_or(default_end);

        info!(
            "Fetching history for group {} from {} to {}",
            binding.group_uid, start_date, end_date
        );

        // Query all expenses for the group in the specified date range
        let expenses = sqlx::query(
            r#"
            SELECT e.uid, e.price::float8 AS price, e.product, e.created_at, c.name as category_name
            FROM expense_entries e
            LEFT JOIN categories c ON e.category_uid = c.uid
            WHERE e.group_uid = $1
              AND e.created_at >= $2
              AND e.created_at < $3
            ORDER BY e.created_at DESC
            "#,
        )
        .bind(binding.group_uid)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(tx.as_mut())
        .await?;

        if expenses.is_empty() {
            return Ok(lang.get("REPORT__NO_EXPENSES"));
        }

        // Calculate total
        let mut total_expenses = 0.0;
        for row in &expenses {
            total_expenses += row.get::<f64, _>("price");
        }

        // Format the response
        let start_date_str = start_date.format("%d/%m/%Y").to_string();
        let end_date_str = end_date.format("%d/%m/%Y").to_string();

        let mut response = format!("Pengeluaran {} -> {}:\n\n", start_date_str, end_date_str);

        for row in expenses {
            let uid: uuid::Uuid = row.get("uid");
            let price: f64 = row.get("price");
            let product: String = row.get("product");
            let created_at: chrono::DateTime<Utc> = row.get("created_at");
            let category_name: Option<String> = row.get("category_name");

            let category = category_name.unwrap_or_else(|| lang.get("REPORT__UNCATEGORIZED"));
            let date_str = created_at.format("%d/%m/%Y %H:%M").to_string();

            response.push_str(&format!(
                "{} {}\n{}, Rp. {}, ({})\n\n",
                date_str,
                uid,
                product,
                format_price(price),
                category
            ));
        }

        response.push_str(&format!("Total: Rp. {}", format_price(total_expenses)));

        Ok(response)
    }

    fn calculate_month_range(
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

impl Command for HistoryCommand {
    fn get_command() -> &'static str {
        "/history"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__HISTORY_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_no_dates() {
        let input = "/history";
        let command = HistoryCommand::parse_command(input).unwrap();
        assert!(command.start_date.is_some());
        assert!(command.end_date.is_some());
        // Should be 3 days apart
        let days_diff = (command.end_date.unwrap() - command.start_date.unwrap()).num_days();
        assert_eq!(days_diff, 3);
    }

    #[test]
    fn test_parse_command_single_date() {
        let input = "/history 2025-09-01";
        let command = HistoryCommand::parse_command(input).unwrap();
        assert_eq!(command.start_date.unwrap().to_string(), "2025-09-01");
        assert_eq!(command.end_date.unwrap().to_string(), "2025-09-01");
    }

    #[test]
    fn test_parse_command_two_dates() {
        let input = "/history 2025-09-01 2025-09-03";
        let command = HistoryCommand::parse_command(input).unwrap();
        assert_eq!(command.start_date.unwrap().to_string(), "2025-09-01");
        assert_eq!(command.end_date.unwrap().to_string(), "2025-09-03");
    }

    #[test]
    fn test_parse_command_invalid_date_range() {
        let input = "/history 2025-09-01 2025-09-10";
        assert!(HistoryCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_start_after_end() {
        let input = "/history 2025-09-10 2025-09-01";
        assert!(HistoryCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_invalid_date_format() {
        let input = "/history invalid-date";
        assert!(HistoryCommand::parse_command(input).is_err());
    }
}
