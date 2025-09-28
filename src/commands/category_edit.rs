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
pub struct CategoryCommand;

impl CategoryCommand {
    /*
        Should be in format:
        /category
    */
    fn parse_command(input: &str) -> Result<Self> {
        let input = input.trim();

        if input != Self::get_command() {
            return Err(anyhow::anyhow!("Invalid format: expected only /history"));
        }

        Ok(Self {})
    }

    /*
        Output format:

        Kategori:
        1. <category name>(id: <category_id>):
            => <alias1, alias2, ...>
        2. <category name>(id: <category_id>):
            => <alias1, alias2, ...>
        3. ...


        Total: X categories

        Example:

        Kategori:
        1. Makanan(id: 1):
            => (makan, food)
        2. Transportasi(id: 2):
            => (transport, travel)
        3. Hiburan(id: 3):
            => (fun, entertainment)
        Total: 3 categories
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        Ok("Category command not implemented yet".to_string())
    }
}

impl Command for CategoryCommand {
    fn get_command() -> &'static str {
        "/category"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
