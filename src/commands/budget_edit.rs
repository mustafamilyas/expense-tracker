use std::collections::HashMap;

use anyhow::Result;
use uuid::Uuid;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{
        budget::{BudgetRepo, UpdateBudgetDbPayload},
        category::CategoryRepo,
        chat_binding::ChatBinding,
    },
};

#[derive(Debug)]
pub struct BudgetEditCommandEntry {
    pub id: Uuid,
    pub category: String,
    pub amount: f64,
}

#[derive(Debug)]
pub struct BudgetEditCommand {
    pub entries: Vec<BudgetEditCommandEntry>,
}

impl BudgetEditCommand {
    /*
        Expected format:
        /budget-edit
        [id] - UUID of the budget to edit
        [category]=[amount]

        Examples:
        /budget-edit
        123e4567-e89b-12d3-a456-426614174000
        Makanan=50000
    */
    fn parse_command(input: &str) -> Result<Vec<BudgetEditCommandEntry>> {
        let mut entries = Vec::new();
        let input = input.trim();

        // Should start with /budget-edit
        let input = if input.starts_with(Self::get_command()) {
            input[Self::get_command().len()..].trim()
        } else {
            input
        };

        let lines: Vec<&str> = input.lines().collect();

        // Process each ID-data pair
        let mut i = 0;
        while i + 1 < lines.len() {
            let id_line = lines[i].trim();
            if id_line.is_empty() {
                i += 1;
                continue; // Skip empty lines
            }
            let data_line = lines[i + 1].trim();

            // Parse UUID
            let id = Uuid::parse_str(id_line)
                .map_err(|_| anyhow::anyhow!("Invalid UUID format: {}", id_line))?;

            // Parse budget data (category=amount)
            let parts: Vec<&str> = data_line.split("=").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid budget format: {}", data_line));
            }

            let category = parts[0].to_string();
            if category.is_empty() {
                return Err(anyhow::anyhow!("Empty category name: {}", data_line));
            }

            let amount_str = parts[1];
            let amount: f64 = amount_str.parse().map_err(|_| {
                anyhow::anyhow!("Invalid amount: {}. Must be a number", amount_str)
            })?;

            entries.push(BudgetEditCommandEntry { id, category, amount });

            i += 2;
        }

        if entries.is_empty() {
            return Err(anyhow::anyhow!("No valid budget entries found"));
        }

        Ok(entries)
    }

    /*
        Output format:
        ✅ Budget berhasil diedit! Jika ingin mengedit lagi, salin dan modifikasi:

        -----
        /budget-edit

        [id]
        [category]=[amount]

    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let entries = Self::parse_command(raw_message)?;

        let mut response = String::new();
        response.push_str(&lang.get("MESSENGER__BUDGET_EDIT_SUCCESS_HEADER"));

        for entry in entries.iter() {
            let id = &entry.id;

            // Get the budget to check ownership
            let budget = BudgetRepo::get(tx, *id).await?;
            if budget.group_uid != binding.group_uid {
                return Err(anyhow::anyhow!("Budget does not belong to this group"));
            }

            // Verify the category matches (optional but good validation)
            let category = CategoryRepo::get(tx, budget.category_uid).await?;
            if category.name != entry.category {
                return Err(anyhow::anyhow!("Category name '{}' does not match the budget's category '{}'", entry.category, category.name));
            }

            // Update the budget amount
            BudgetRepo::update(
                tx,
                *id,
                UpdateBudgetDbPayload {
                    amount: Some(entry.amount),
                    period_year: None,
                    period_month: None,
                },
            )
            .await?;

            response.push_str(&lang.get_with_vars(
                "MESSENGER__BUDGET_EDIT_SUCCESS_ENTRY",
                HashMap::from([
                    ("id".to_string(), id.to_string()),
                    ("category".to_string(), entry.category.clone()),
                    ("amount".to_string(), entry.amount.to_string()),
                ]),
            ));
        }

        Ok(response)
    }
}

impl Command for BudgetEditCommand {
    fn get_command() -> &'static str {
        "/budget-edit"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__BUDGET_EDIT_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let input = "/budget-edit
123e4567-e89b-12d3-a456-426614174000
Makanan=50000

44444444-4444-4444-4444-000000000001
Transportasi=30000

";

        let entries = BudgetEditCommand::parse_command(input).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].id.to_string(),
            "123e4567-e89b-12d3-a456-426614174000"
        );
        assert_eq!(entries[0].category, "Makanan");
        assert_eq!(entries[0].amount, 50000.0);

        assert_eq!(
            entries[1].id.to_string(),
            "44444444-4444-4444-4444-000000000001"
        );
        assert_eq!(entries[1].category, "Transportasi");
        assert_eq!(entries[1].amount, 30000.0);
    }

    #[test]
    fn test_parse_command_invalid_format() {
        let input = "/budget-edit
123e4567-e89b-12d3-a456-426614174000";

        assert!(BudgetEditCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_invalid_uuid() {
        let input = "/budget-edit
invalid-uuid
Makanan=50000";

        assert!(BudgetEditCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_empty_category() {
        let input = "/budget-edit
123e4567-e89b-12d3-a456-426614174000
=50000";

        assert!(BudgetEditCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_invalid_amount() {
        let input = "/budget-edit
123e4567-e89b-12d3-a456-426614174000
Makanan=abc";

        assert!(BudgetEditCommand::parse_command(input).is_err());
    }
}