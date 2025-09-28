use std::collections::HashMap;

use anyhow::Result;
use teloxide::types::ChatId;
use uuid::Uuid;

use crate::{
    commands::base::Command,
    lang::Lang,
    middleware::tier::check_tier_limit,
    repos::{
        category::CategoryRepo,
        chat_binding::ChatBinding,
        expense_entry::{CreateExpenseEntryDbPayload, ExpenseEntryRepo},
        subscription::{SubscriptionRepo, UserUsageRepo},
    },
    utils::parse_price::{format_price, parse_price},
};

#[derive(Debug)]
pub struct ExpenseCommandEntry {
    pub name: String,
    pub price: f64,
    pub category_or_alias: Option<String>,
}

#[derive(Debug)]
pub struct ExpenseCommand {
    pub entries: Vec<ExpenseCommandEntry>,
    pub fail_entries: Vec<String>, // Store failed entries for reporting
}

impl ExpenseCommand {
    /*
     Expected format:
     /expense
     [name],[price],[optional category]
     or
     /expense [name],[price],[optional category]

     Examples:
     /expense
     Nasi Padang,10000,Makanan
     Warteg,15000

     or
     /expense Nasi Padang,10000,Makanan

     TODO: Improve error handling and reporting
     for example we have 10 entries, but 2 are invalid, we should return which ones are invalid
    */
    fn parse_command(input: &str) -> Result<Self> {
        let mut entries = Vec::new();
        let input = input.trim();
        let mut fail_entries = Vec::new();

        // Should start with /expense
        let input = if input.starts_with(Self::get_command()) {
            input[Self::get_command().len()..].trim()
        } else {
            input
        };

        // Split by new lines
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split by commas
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 2 {
                continue; // Invalid entry, skip
            }

            let name = parts[0].to_string();
            if name.is_empty() {
                fail_entries.push(line.to_string());
                continue; // Invalid name, skip
            }
            let Ok(price) = parse_price(parts[1]) else {
                fail_entries.push(line.to_string());
                continue; // Invalid price, skip
            };
            let category_or_alias = if parts.len() >= 3 {
                Some(parts[2].to_string())
            } else {
                None
            };

            entries.push(ExpenseCommandEntry {
                name,
                price,
                category_or_alias,
            });
        }

        if entries.is_empty() {
            return Err(anyhow::anyhow!("No valid expense entries found"));
        }

        Ok(Self {
            entries,
            fail_entries,
        })
    }

    /*
       If no success, at all, return error
       If some success, some fail, return success message with fail info

       Format
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        // TODO: Change subscription, check the
        // let subscription = SubscriptionRepo::get_by_user(tx, binding.bound_by).await?;
        // let usage_payload = UserUsageRepo::calculate_current_usage(tx, binding.bound_by).await?;
        // check_tier_limit(
        //     &subscription,
        //     "expenses_per_month",
        //     usage_payload.total_expenses,
        // )?;

        let command = Self::parse_command(raw_message)?;
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;

        // For now, assume category already exists or is optional
        let mut category_map: HashMap<String, Uuid> = HashMap::new();

        for category in categories {
            category_map.insert(category.name.to_lowercase(), category.uid);
            if let Some(alias) = &category.alias {
                category_map.insert(alias.to_lowercase(), category.uid);
            }
        }

        // TODO: Better formatting
        let mut response = String::new();
        response.push_str(&lang.get("MESSENGER__ENTRY_SUCCESS_HEADER"));

        for entry in command.entries {
            let price = entry.price;
            let product = entry.name;
            let category_uid = if let Some(cat) = entry.category_or_alias {
                if let Some(uid) = category_map.get(&cat.to_lowercase()) {
                    Some(*uid)
                } else {
                    None
                }
            } else {
                None
            };
            // Create expense entry
            let expense = ExpenseEntryRepo::create_expense_entry(
                tx,
                CreateExpenseEntryDbPayload {
                    price,
                    product,
                    group_uid: binding.group_uid,
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

        if !command.fail_entries.is_empty() {
            response.push_str("-----\n");
            response.push_str(&&lang.get_with_vars(
                "MESSENGER__ENTRY_FAIL_INVALID_FORMAT",
                HashMap::from([("line".to_string(), command.fail_entries.join("\n"))]),
            ));
        }

        Ok(response)
    }
}

impl Command for ExpenseCommand {
    fn get_command() -> &'static str {
        "/expense"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string() {
        let input = "/expense
        Nasi Padang,10000,Makanan
        Warteg,15000
        InvalidEntry
        ,20000
        Burger,-5000
        ";

        let entries = ExpenseCommand::parse_command(input).unwrap();
        assert_eq!(entries.entries.len(), 2);
        assert_eq!(entries.fail_entries.len(), 4);
        assert_eq!(entries.entries[0].name, "Nasi Padang");
        assert_eq!(entries.entries[0].price, 10000.0);
        assert_eq!(
            entries.entries[0].category_or_alias.as_deref(),
            Some("Makanan")
        );
        assert_eq!(entries.entries[1].name, "Warteg");
        assert_eq!(entries.entries[1].price, 15000.0);
        assert_eq!(entries.entries[1].category_or_alias, None);

        let input2 = "/expense Nasi Goreng,20000,Makanan";
        let entries2 = ExpenseCommand::parse_command(input2).unwrap();
        assert_eq!(entries2.entries.len(), 1);
        assert_eq!(entries2.fail_entries.len(), 0);
        assert_eq!(entries2.entries[0].name, "Nasi Goreng");
        assert_eq!(entries2.entries[0].price, 20000.0);
        assert_eq!(
            entries2.entries[0].category_or_alias.as_deref(),
            Some("Makanan")
        );
    }
}
