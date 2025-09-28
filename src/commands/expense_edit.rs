use std::collections::HashMap;

use anyhow::Result;
use uuid::Uuid;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{
        category::CategoryRepo,
        chat_binding::ChatBinding,
        expense_entry::{ExpenseEntryRepo, UpdateExpenseEntryDbPayload},
    },
    utils::parse_price::{format_price, parse_price},
};

#[derive(Debug)]
pub struct ExpenseEditCommandEntry {
    pub id: Uuid,
    pub name: String,
    pub price: f64,
    pub category_or_alias: Option<String>,
}

#[derive(Debug)]
pub struct ExpenseEditCommand {
    pub entries: Vec<ExpenseEditCommandEntry>,
}

impl ExpenseEditCommand {
    /*
     Expected format:
     /expense-edit
     [id] - UUID of the expense entry to edit
     [name],[price],[optional category]

     Examples:
     /expense-edit
     123e4567-e89b-12d3-a456-426614174000
     Nasi Padang,10000,Makanan

     123e4567-e89b-12d3-a456-426614174001
     Warteg,15000
    */
    fn parse_command(input: &str) -> Result<Vec<ExpenseEditCommandEntry>> {
        let mut entries = Vec::new();
        let input = input.trim();

        // Should start with /expense-edit
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
            println!("Parsing ID line: {}", id_line);
            let data_line = lines[i + 1].trim();

            // Parse UUID
            let id = Uuid::parse_str(id_line)
                .map_err(|_| anyhow::anyhow!("Invalid UUID format: {}", id_line))?;

            // Parse expense data (name,price,category)
            let parts: Vec<&str> = data_line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 2 {
                return Err(anyhow::anyhow!("Invalid expense format: {}", data_line));
            }

            let name = parts[0].to_string();
            if name.is_empty() {
                return Err(anyhow::anyhow!("Empty expense name: {}", data_line));
            }

            let price = parse_price(parts[1])
                .map_err(|_| anyhow::anyhow!("Invalid price format: {}", parts[1]))?;

            let category_or_alias = if parts.len() >= 3 && !parts[2].is_empty() {
                Some(parts[2].to_string())
            } else {
                None
            };

            entries.push(ExpenseEditCommandEntry {
                id,
                name,
                price,
                category_or_alias,
            });

            i += 2;
        }

        if entries.is_empty() {
            return Err(anyhow::anyhow!("No valid expense entries found"));
        }

        Ok(entries)
    }

    /*
       Output format should be the same as expense command, but with "edit" worded
       Example:
       ✅ Pengeluaran berhasil diedit! Jika ingin mengedit, salin dan modifikasi (can be found on lang/id.json)
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let entries = Self::parse_command(raw_message)?;

        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;
        let mut category_map: HashMap<String, Uuid> = HashMap::new();

        for category in categories {
            category_map.insert(category.name.to_lowercase(), category.uid);
            if let Some(alias) = &category.alias {
                category_map.insert(alias.to_lowercase(), category.uid);
            }
        }

        let mut response = String::new();
        response.push_str(&lang.get("MESSENGER__ENTRY_EDIT_SUCCESS_HEADER"));

        for entry in entries.iter() {
            let id = &entry.id;
            let category_uid = if let Some(cat) = &entry.category_or_alias {
                category_map.get(&cat.to_lowercase()).copied()
            } else {
                None
            };

            // Update the expense entry
            let expense = ExpenseEntryRepo::update(
                tx,
                *id,
                UpdateExpenseEntryDbPayload {
                    price: Some(entry.price),
                    product: Some(entry.name.clone()),
                    category_uid,
                },
            )
            .await?;

            response.push_str(
                &lang.get_with_vars(
                    "MESSENGER__ENTRY_SUCCESS_EDIT_ENTRY",
                    HashMap::from([
                        ("id".to_string(), expense.uid.to_string()),
                        ("item".to_string(), expense.product),
                        (
                            "price".to_string(),
                            format!("Rp. {}", format_price(expense.price)),
                        ),
                        (
                            "category".to_string(),
                            category_uid
                                .map(|uid| uid.to_string())
                                .unwrap_or_else(|| "Uncategorized".to_string()),
                        ),
                    ]),
                ),
            );
        }

        Ok(response)
    }
}

impl Command for ExpenseEditCommand {
    fn get_command() -> &'static str {
        "/expense-edit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let input = "/expense-edit
44444444-4444-4444-4444-000000000002
Nasi Padang,10000,Makanan


44444444-4444-4444-4444-000000000003
Warteg,15000
44444444-4444-4444-4444-000000000004
Bakso,20000,Food

";

        let entries = ExpenseEditCommand::parse_command(input).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[0].id.to_string(),
            "44444444-4444-4444-4444-000000000002"
        );
        assert_eq!(entries[0].name, "Nasi Padang");
        assert_eq!(entries[0].price, 10000.0);
        assert_eq!(entries[0].category_or_alias.as_deref(), Some("Makanan"));

        assert_eq!(
            entries[1].id.to_string(),
            "44444444-4444-4444-4444-000000000003"
        );
        assert_eq!(entries[1].name, "Warteg");
        assert_eq!(entries[1].price, 15000.0);
        assert_eq!(entries[1].category_or_alias, None);

        assert_eq!(
            entries[2].id.to_string(),
            "44444444-4444-4444-4444-000000000004"
        );
        assert_eq!(entries[2].name, "Bakso");
        assert_eq!(entries[2].price, 20000.0);
        assert_eq!(entries[2].category_or_alias.as_deref(), Some("Food"));
    }

    #[test]
    fn test_parse_command_invalid_format() {
        let input = "/expense-edit
123e4567-e89b-12d3-a456-426614174000";

        assert!(ExpenseEditCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_invalid_uuid() {
        let input = "/expense-edit
invalid-uuid
Nasi Padang,10000,Makanan";

        assert!(ExpenseEditCommand::parse_command(input).is_err());
    }
}
