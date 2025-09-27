#[derive(Debug, Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub chat_relay_secret: String,
    pub front_end_url: String,
    pub chat_bind_url: String,
    pub telegram_bot_token: String,
    pub database_url: String,

    pub telegram_log_token: Option<String>,
    pub telegram_log_chat_id: Option<i64>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        let jwt_secret = std::env::var("JWT_SECRET").unwrap();
        let chat_relay_secret = std::env::var("CHAT_RELAY_SECRET").unwrap();
        let front_end_url = std::env::var("FRONT_END_URL").unwrap();
        let chat_bind_url = std::env::var("CHAT_BIND_URL").unwrap();
        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap();
        let database_url = std::env::var("DATABASE_URL").unwrap();

        let telegram_log_token = std::env::var("TELEGRAM_LOG_BOT_TOKEN").ok();
        let telegram_log_chat_id = std::env::var("TELEGRAM_LOG_CHAT_ID")
            .ok()
            .and_then(|id_str| id_str.parse::<i64>().ok());

        Config {
            jwt_secret,
            chat_relay_secret,
            front_end_url,
            chat_bind_url,
            telegram_bot_token,
            database_url,
            telegram_log_token,
            telegram_log_chat_id,
        }
    }
}
