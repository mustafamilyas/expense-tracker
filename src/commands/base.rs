#[async_trait::async_trait(?Send)]
pub trait Command {
    fn get_command() -> &'static str;

    fn get_instruction_text_key() -> &'static str {
        "MESSENGER__INSTRUCTION_UNKNOWN_COMMAND"
    }
}
