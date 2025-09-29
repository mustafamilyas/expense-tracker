use std::collections::HashMap;

use anyhow::Result;
use uuid::Uuid;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{
        category::{CategoryRepo, UpdateCategoryDbPayload},
        category_alias::{CategoryAliasRepo, CreateCategoryAliasDbPayload},
        chat_binding::ChatBinding,
    },
};

#[derive(Debug)]
pub struct CategoryEditCommandEntry {
    pub id: Uuid,
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug)]
pub struct CategoryEditCommand {
    pub entries: Vec<CategoryEditCommandEntry>,
}

impl CategoryEditCommand {
    /*
        Expected format:
        /category-edit
        [id] - UUID of the category to edit
        [name]=[alias1, alias2, ...]

        Examples:
        /category-edit
        123e4567-e89b-12d3-a456-426614174000
        Makanan=makan, food
    */
    fn parse_command(input: &str) -> Result<Vec<CategoryEditCommandEntry>> {
        let mut entries = Vec::new();
        let input = input.trim();

        // Should start with /category-edit
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

            // Parse category data (name=aliases)
            let parts: Vec<&str> = data_line.split("=").map(|s| s.trim()).collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid category format: {}", data_line));
            }

            let name = parts[0].to_string();
            if name.is_empty() {
                return Err(anyhow::anyhow!("Empty category name: {}", data_line));
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

            entries.push(CategoryEditCommandEntry { id, name, aliases });

            i += 2;
        }

        if entries.is_empty() {
            return Err(anyhow::anyhow!("No valid category entries found"));
        }

        Ok(entries)
    }

    /*
        Output format:
        ✅ Kategori berhasil diedit! Jika ingin mengedit lagi, salin dan modifikasi:

        -----
        /category-edit

        [id]
        [name]=[aliases]

    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let entries = Self::parse_command(raw_message)?;

        let mut response = String::new();
        response.push_str(&lang.get("MESSENGER__CATEGORY_EDIT_SUCCESS_HEADER"));

        for entry in entries.iter() {
            let id = &entry.id;

            // Get the category to check ownership
            let category = CategoryRepo::get(tx, *id).await?;
            if category.group_uid != binding.group_uid {
                return Err(anyhow::anyhow!("Category does not belong to this group"));
            }

            // Update the category name
            CategoryRepo::update(
                tx,
                *id,
                UpdateCategoryDbPayload {
                    name: Some(entry.name.clone()),
                    description: None,
                },
            )
            .await?;

            // Delete existing aliases for this category
            let existing_aliases = CategoryAliasRepo::list_by_category(tx, *id).await?;
            for alias in existing_aliases {
                CategoryAliasRepo::delete(tx, alias.alias_uid).await?;
            }

            // Create new aliases
            for alias in &entry.aliases {
                CategoryAliasRepo::create(
                    tx,
                    CreateCategoryAliasDbPayload {
                        group_uid: binding.group_uid,
                        alias: alias.clone(),
                        category_uid: *id,
                    },
                )
                .await?;
            }

            let aliases_str = if entry.aliases.is_empty() {
                "".to_string()
            } else {
                entry.aliases.join(", ")
            };

            response.push_str(&lang.get_with_vars(
                "MESSENGER__CATEGORY_EDIT_SUCCESS_ENTRY",
                HashMap::from([
                    ("id".to_string(), id.to_string()),
                    ("name".to_string(), entry.name.clone()),
                    ("aliases".to_string(), aliases_str),
                ]),
            ));
        }

        Ok(response)
    }
}

impl Command for CategoryEditCommand {
    fn get_command() -> &'static str {
        "/category-edit"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__CATEGORY_EDIT_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let input = "/category-edit
123e4567-e89b-12d3-a456-426614174000
Makanan=makan, food

44444444-4444-4444-4444-000000000001
Transportasi=transport

";

        let entries = CategoryEditCommand::parse_command(input).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].id.to_string(),
            "123e4567-e89b-12d3-a456-426614174000"
        );
        assert_eq!(entries[0].name, "Makanan");
        assert_eq!(entries[0].aliases, vec!["makan", "food"]);

        assert_eq!(
            entries[1].id.to_string(),
            "44444444-4444-4444-4444-000000000001"
        );
        assert_eq!(entries[1].name, "Transportasi");
        assert_eq!(entries[1].aliases, vec!["transport"]);
    }

    #[test]
    fn test_parse_command_invalid_format() {
        let input = "/category-edit
123e4567-e89b-12d3-a456-426614174000";

        assert!(CategoryEditCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_invalid_uuid() {
        let input = "/category-edit
invalid-uuid
Makanan=makan";

        assert!(CategoryEditCommand::parse_command(input).is_err());
    }

    #[test]
    fn test_parse_command_empty_name() {
        let input = "/category-edit
123e4567-e89b-12d3-a456-426614174000
=makan";

        assert!(CategoryEditCommand::parse_command(input).is_err());
    }
}
