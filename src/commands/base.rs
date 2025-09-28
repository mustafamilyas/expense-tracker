#[async_trait::async_trait(?Send)]
pub trait Command {
    fn get_command() -> &'static str;
}
