use std::collections::HashMap;

use anyhow::Result;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{
        category::{CategoryRepo, CreateCategoryDbPayload},
        category_alias::{CategoryAliasRepo, CreateCategoryAliasDbPayload},
        chat_binding::ChatBinding,
    },
};

#[derive(Debug)]
pub struct CategoryCommandEntry {
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug)]
pub struct CategoryCommand {
    pub action: CategoryAction,
}

#[derive(Debug)]
pub enum CategoryAction {
    List,
    Create(Vec<CategoryCommandEntry>),
}

impl CategoryCommand {
    /*
        Should be in format:
        1. get list
        /category
         or
        2. create new category
        /category
        [category name]=[alias1, alias2, ...]
        [category name]=[alias1, alias2, ...]
        ...

        Example:
        /category
        Makanan=makan, food
        Transportasi=transport, travel
        Hiburan=fun, entertainment

        or
        /category Makanan=makan, food


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
            // Just /category - list command
            return Ok(Self {
                action: CategoryAction::List,
            });
        }

        // Parse category definitions
        let lines: Vec<&str> = input.lines().map(|line| line.trim()).collect();
        let mut entries = Vec::new();

        for line in lines {
            if line.is_empty() {
                continue;
            }

            // Parse format: "CategoryName=alias1, alias2, alias3"
            let parts: Vec<&str> = line.split("=").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Invalid format: {}. Expected 'CategoryName=alias1, alias2, ...'",
                    line
                ));
            }

            let name = parts[0].to_string();
            if name.is_empty() {
                return Err(anyhow::anyhow!("Category name cannot be empty"));
            }

            let aliases_str = parts[1];
            let aliases: Vec<String> = if aliases_str.is_empty() {
                Vec::new()
            } else {
                aliases_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };

            entries.push(CategoryCommandEntry { name, aliases });
        }

        if entries.is_empty() {
            return Err(anyhow::anyhow!("No valid category definitions found"));
        }

        Ok(Self {
            action: CategoryAction::Create(entries),
        })
    }

    /*
        Output format:


        1. get list response:

        Kategori:
        1. [category name]: ([alias1, alias2, ...])
        2. [category name]: ([alias1, alias2, ...])
        3. ...


        Total: X categories

        Example:

        Kategori:
        1. Makanan: (makan, food)
        2. Transportasi: (transport, travel)
        3. Hiburan: (fun, entertainment)
        Total: 3 categories

        Untuk menambah kategori, gunakan perintah
        /category [nama kategori]=>[alias1, alias2, ...]
        Contoh:
        /category Makanan=>makan, food

        2. create new category response:
        Kategori [category name] dengan alias ([alias1, alias2, ...]) berhasil ditambahkan.
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let command = Self::parse_command(raw_message)?;

        match &command.action {
            CategoryAction::List => Self::get_list(binding, tx, lang).await,
            CategoryAction::Create(entries) => {
                Self::create_categories(entries, binding, tx, lang).await
            }
        }
    }

    async fn get_list(
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        // Fetch categories for the group
        let categories = CategoryRepo::list_by_group(tx, binding.group_uid).await?;

        if categories.is_empty() {
            return Ok(lang.get("MESSENGER__CATEGORY_LIST_EMPTY"));
        }

        // Fetch category aliases for the group
        let aliases = CategoryAliasRepo::list_by_group(tx, binding.group_uid).await?;

        // Group aliases by category_uid
        let mut aliases_by_category: HashMap<uuid::Uuid, Vec<String>> = HashMap::new();
        for alias in aliases {
            aliases_by_category
                .entry(alias.category_uid)
                .or_insert_with(Vec::new)
                .push(alias.alias);
        }

        // Format the response
        let mut response = "Kategori:\n".to_string();

        for (index, category) in categories.iter().enumerate() {
            let category_aliases = aliases_by_category
                .get(&category.uid)
                .map(|aliases| aliases.join(", "))
                .unwrap_or_else(|| "".to_string());

            let aliases_str = if category_aliases.is_empty() {
                "".to_string()
            } else {
                format!(" ({})", category_aliases)
            };

            response.push_str(&format!(
                "{}. {}{}\n",
                index + 1,
                category.name,
                aliases_str
            ));
        }

        response.push_str(&format!("\nTotal: {} categories", categories.len()));
        response.push_str(&lang.get("MESSENGER__CATEGORY_LIST_FOOTER"));

        Ok(response)
    }

    async fn create_categories(
        entries: &[CategoryCommandEntry],
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let mut results = Vec::new();

        for entry in entries {
            // Create the category
            let category = CategoryRepo::create(
                tx,
                CreateCategoryDbPayload {
                    group_uid: binding.group_uid,
                    name: entry.name.clone(),
                    description: None,
                },
            )
            .await?;

            // Create aliases
            for alias in &entry.aliases {
                CategoryAliasRepo::create(
                    tx,
                    CreateCategoryAliasDbPayload {
                        group_uid: binding.group_uid,
                        alias: alias.clone(),
                        category_uid: category.uid,
                    },
                )
                .await?;
            }

            let aliases_str = if entry.aliases.is_empty() {
                "".to_string()
            } else {
                entry.aliases.join(", ")
            };

            results.push(lang.get_with_vars(
                "MESSENGER__CATEGORY_CREATED",
                HashMap::from([
                    ("name".to_string(), entry.name.clone()),
                    ("aliases".to_string(), aliases_str),
                ]),
            ));
        }

        Ok(results.join("\n"))
    }
}

impl Command for CategoryCommand {
    fn get_command() -> &'static str {
        "/category"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__CATEGORY_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_list() {
        let input = "/category";
        let command = CategoryCommand::parse_command(input).unwrap();
        match command.action {
            CategoryAction::List => {}
            _ => panic!("Expected List action"),
        }
    }

    #[test]
    fn test_parse_command_create_single_line() {
        let input = "/category Makanan = makan, food";
        let command = CategoryCommand::parse_command(input).unwrap();
        match &command.action {
            CategoryAction::Create(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].name, "Makanan");
                assert_eq!(entries[0].aliases, vec!["makan", "food"]);
            }
            _ => panic!("Expected Create action"),
        }
    }

    #[test]
    fn test_parse_command_create_multiple_lines() {
        let input = "/category\nMakanan = makan, food\nTransportasi=transport";
        let command = CategoryCommand::parse_command(input).unwrap();
        match &command.action {
            CategoryAction::Create(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].name, "Makanan");
                assert_eq!(entries[0].aliases, vec!["makan", "food"]);
                assert_eq!(entries[1].name, "Transportasi");
                assert_eq!(entries[1].aliases, vec!["transport"]);
            }
            _ => panic!("Expected Create action"),
        }
    }

    #[test]
    fn test_parse_command_invalid_format() {
        let input = "/category invalid format";
        assert!(CategoryCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_empty_name() {
        let input = "/category =>alias";
        assert!(CategoryCommand::parse_command(input).is_err());
    }
}
