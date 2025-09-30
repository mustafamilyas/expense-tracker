use std::collections::HashMap;

use anyhow::Result;

use crate::{
    commands::base::Command,
    lang::Lang,
    repos::{chat_binding::ChatBinding, expense_group::ExpenseGroupRepo, user::UserRepo},
};

#[derive(Debug)]
pub struct HelpCommand;

impl HelpCommand {
    /*
        Should be in format:
        /help
    */
    fn parse_command(input: &str) -> Result<Self> {
        let input = input.trim();

        if input != Self::get_command() {
            return Err(anyhow::anyhow!("Invalid format: expected only /help"));
        }

        Ok(Self {})
    }

    /*
        Output format:

        Hello, <name>! Chat ini terhubung dengan akun <email> (<group_name>).
        Berikut adalah daftar perintah yang tersedia:
        1. /expense [nama],[harga],[kategori] - Menambahkan entri pengeluaran.
        2. /expense-edit [id] [nama],[harga],[kategori] - Mengedit entri pengeluaran.
        3. /category [nama kategori]=[alias1, alias2, ...] - Menampilkan atau menambahkan kategori.
        4. /category-edit [id] [nama kategori]=[alias1, alias2, ...] - Mengedit kategori.
        5. /history (start_date) (end_date) - Menampilkan riwayat pengeluaran.
        6. /report - Menampilkan laporan pengeluaran bulanan.
        7. /help - Menampilkan daftar perintah yang tersedia.
        Gunakan perintah di atas untuk mengelola pengeluaran Anda dengan mudah!

        Untuk bantuan lebih lanjut, hubungi admin @mustafamilyas
    */

    pub async fn run(
        raw_message: &str,
        binding: &ChatBinding,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        lang: &Lang,
    ) -> Result<String> {
        let _command = Self::parse_command(raw_message)?;

        // Get user info
        let user = UserRepo::get(tx, binding.bound_by).await?;
        // Get group info
        let group = ExpenseGroupRepo::get(tx, binding.group_uid).await?;

        let mut response = format!(
            "{}",
            lang.get_with_vars(
                "MESSENGER__HELP_INTRO",
                HashMap::from([
                    ("name".to_string(), user.email.clone()),
                    ("group".to_string(), group.name.clone())
                ])
            )
        );

        response
            .push_str(format!("{}\n\n", lang.get("MESSENGER__HELP_COMMAND_LIST_HEADER")).as_str());

        // List all commands with their instructions
        let commands = vec![
            "MESSENGER__EXPENSE_SHORT_INSTRUCTION",
            "MESSENGER__EXPENSE_EDIT_SHORT_INSTRUCTION",
            "MESSENGER__BUDGET_SHORT_INSTRUCTION",
            "MESSENGER__BUDGET_EDIT_SHORT_INSTRUCTION",
            "MESSENGER__CATEGORY_SHORT_INSTRUCTION",
            "MESSENGER__CATEGORY_EDIT_SHORT_INSTRUCTION",
            "MESSENGER__HISTORY_SHORT_INSTRUCTION",
            "MESSENGER__REPORT_SHORT_INSTRUCTION",
            "MESSENGER__HELP_SHORT_INSTRUCTION",
        ];

        for (index, key) in commands.iter().enumerate() {
            response.push_str(&format!("{}. {}\n", index + 1, lang.get(key)));
        }
        response.push('\n');

        response.push_str(format!("{}\n\n", lang.get("MESSENGER__HELP_CLOSING")).as_str());
        response.push_str(format!("{}", lang.get("MESSENGER__HELP_CTA")).as_str());

        Ok(response)
    }
}

impl Command for HelpCommand {
    fn get_command() -> &'static str {
        "/help"
    }

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__HELP_SHORT_INSTRUCTION"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_valid() {
        let input = "/help";
        let command = HelpCommand::parse_command(input).unwrap();
        // Just check it doesn't panic
    }

    #[test]
    fn test_parse_command_invalid() {
        let input = "/help extra";
        assert!(HelpCommand::parse_command(input).is_err());
    }
}
