use std::collections::HashMap;

use anyhow::Result;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{
        budget::{BudgetRepo, CreateBudgetDbPayload, UpdateBudgetDbPayload},
        category::CategoryRepo,
        chat_binding::ChatBinding,
    },
};

#[derive(Debug)]
pub struct BudgetCommandEntry {
    pub category: String,
    pub amount: f64,
}

#[derive(Debug)]
pub struct BudgetCommand {
    pub action: BudgetAction,
}

#[derive(Debug)]
pub enum BudgetAction {
    List,
    Create(Vec<BudgetCommandEntry>),
}

impl BudgetCommand {
    /*
        Should be in format:
        1. get list
        /budget
         or
        2. create new budget
        /budget
        [category name]=[amount]
        [category name]=[amount]
        ...

        Example:
        /budget
        Makanan=50000
        Transportasi=30000

        or
        /budget Makanan=50000

    */
    fn parse_command(input: &str) -> Result<Self> {
        let input = input.trim();

        // Remove the command prefix
        let input = if input.starts_with(Self::get_command()) {
            input[Self::get_command().len()..].trim()
        } else {
            input
        };

        if input.is_empty() {
            // Just /budget - list command
            return Ok(Self {
                action: BudgetAction::List,
            });
        }

        // Parse budget definitions
        let lines: Vec<&str> = input.lines().map(|line| line.trim()).collect();
        let mut entries = Vec::new();

        for line in lines {
            if line.is_empty() {
                continue;
            }

            // Parse format: "CategoryName=amount"
            let parts: Vec<&str> = line.split("=").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Invalid format: {}. Expected 'CategoryName=amount'",
                    line
                ));
            }

            let category = parts[0].to_string();
            if category.is_empty() {
                return Err(anyhow::anyhow!("Category name cannot be empty"));
            }

            let amount_str = parts[1];
            let amount: f64 = amount_str.parse().map_err(|_| {
                anyhow::anyhow!("Invalid amount: {}. Must be a number", amount_str)
            })?;

            entries.push(BudgetCommandEntry { category, amount });
        }

        if entries.is_empty() {
            return Err(anyhow::anyhow!("No valid budget definitions found"));
        }

        Ok(Self {
            action: BudgetAction::Create(entries),
        })
    }

    /*
        Output format:


        1. get list response:

        Budget:
        1. [category name]: [amount]
        2. [category name]: [amount]
        3. ...

        Total: X budgets

        Example:

        Budget:
        1. Makanan: 50000
        2. Transportasi: 30000
        Total: 2 budgets

        Untuk menambah budget, gunakan perintah
        /budget [nama kategori]=[amount]
        Contoh:
        /budget Makanan=50000

        2. create new budget response:
        Budget untuk [category name] sebesar [amount] berhasil ditambahkan.
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let command = Self::parse_command(raw_message)?;

        match &command.action {
            BudgetAction::List => Self::get_list(binding, tx, lang).await,
            BudgetAction::Create(entries) => {
                Self::create_budgets(entries, binding, tx, lang).await
            }
        }
    }

    async fn get_list(
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        // Fetch budgets for the group
        let budgets = BudgetRepo::list_by_group(tx, binding.group_uid).await?;

        if budgets.is_empty() {
            return Ok(lang.get("MESSENGER__BUDGET_LIST_EMPTY"));
        }

        // Fetch categories for the group
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;

        // Group categories by uid
        let mut categories_by_uid: HashMap<uuid::Uuid, String> = HashMap::new();
        for category in categories {
            categories_by_uid.insert(category.uid, category.name);
        }

        // Format the response
        let mut response = "Budget:\n".to_string();

        for (index, budget) in budgets.iter().enumerate() {
            let category_name = categories_by_uid
                .get(&budget.category_uid)
                .map(|name| name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            response.push_str(&format!(
                "{}. {}: {}\n",
                index + 1,
                category_name,
                budget.amount
            ));
        }

        response.push_str(&format!("\nTotal: {} budgets", budgets.len()));
        response.push_str(&lang.get("MESSENGER__BUDGET_LIST_FOOTER"));

        Ok(response)
    }

    async fn create_budgets(
        entries: &[BudgetCommandEntry],
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let mut results = Vec::new();

        for entry in entries {
            // Find the category
            let category = CategoryRepo::find_by_name_or_alias(tx, binding.group_uid, &entry.category).await?
                .ok_or_else(|| anyhow::anyhow!("Category '{}' not found", entry.category))?;

            // Check if budget exists
            let existing_budget = BudgetRepo::get_by_group_and_category(tx, binding.group_uid, category.uid).await?;

            let result = if let Some(budget) = existing_budget {
                // Update existing budget
                BudgetRepo::update(
                    tx,
                    budget.uid,
                    UpdateBudgetDbPayload {
                        amount: Some(entry.amount),
                        period_year: None,
                        period_month: None,
                    },
                ).await?;
                lang.get_with_vars(
                    "MESSENGER__BUDGET_UPDATED",
                    HashMap::from([
                        ("category".to_string(), category.name.clone()),
                        ("amount".to_string(), entry.amount.to_string()),
                    ]),
                )
            } else {
                // Create new budget
                BudgetRepo::create(
                    tx,
                    CreateBudgetDbPayload {
                        group_uid: binding.group_uid,
                        category_uid: category.uid,
                        amount: entry.amount,
                        period_year: None,
                        period_month: None,
                    },
                ).await?;
                lang.get_with_vars(
                    "MESSENGER__BUDGET_CREATED",
                    HashMap::from([
                        ("category".to_string(), category.name.clone()),
                        ("amount".to_string(), entry.amount.to_string()),
                    ]),
                )
            };

            results.push(result);
        }

        Ok(results.join("\n"))
    }
}

impl Command for BudgetCommand {
    fn get_command() -> &'static str {
        "/budget"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__BUDGET_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_list() {
        let input = "/budget";
        let command = BudgetCommand::parse_command(input).unwrap();
        match command.action {
            BudgetAction::List => {}
            _ => panic!("Expected List action"),
        }
    }

    #[test]
    fn test_parse_command_create_single_line() {
        let input = "/budget Makanan = 50000";
        let command = BudgetCommand::parse_command(input).unwrap();
        match &command.action {
            BudgetAction::Create(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].category, "Makanan");
                assert_eq!(entries[0].amount, 50000.0);
            }
            _ => panic!("Expected Create action"),
        }
    }

    #[test]
    fn test_parse_command_create_multiple_lines() {
        let input = "/budget\nMakanan = 50000\nTransportasi=30000";
        let command = BudgetCommand::parse_command(input).unwrap();
        match &command.action {
            BudgetAction::Create(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].category, "Makanan");
                assert_eq!(entries[0].amount, 50000.0);
                assert_eq!(entries[1].category, "Transportasi");
                assert_eq!(entries[1].amount, 30000.0);
            }
            _ => panic!("Expected Create action"),
        }
    }

    #[test]
    fn test_parse_command_invalid_format() {
        let input = "/budget invalid format";
        assert!(BudgetCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_empty_category() {
        let input = "/budget =>50000";
        assert!(BudgetCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_invalid_amount() {
        let input = "/budget Makanan=abc";
        assert!(BudgetCommand::parse_command(input).is_err());
    }
}