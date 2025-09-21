use teloxide::{prelude::*, types::ChatId};
use tracing_subscriber::Layer;

pub struct TelegramLogger {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramLogger {
    pub fn new(token: String, chat_id: i64) -> Self {
        Self {
            bot: Bot::new(token),
            chat_id: ChatId(chat_id),
        }
    }
}

impl<S> Layer<S> for TelegramLogger
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let level = event.metadata().level();
        if *level == tracing::Level::WARN || *level == tracing::Level::ERROR {
            let mut message = format!("{}: ", level);

            let mut visitor = StringVisitor::new();
            event.record(&mut visitor);
            message.push_str(&visitor.0);

            let bot = self.bot.clone();
            let chat_id = self.chat_id;

            tokio::spawn(async move {
                if let Err(e) = bot.send_message(chat_id, &message).await {
                    eprintln!("Failed to send log to Telegram: {:?}", e);
                }
            });
        }
    }
}

struct StringVisitor(String);

impl StringVisitor {
    fn new() -> Self {
        Self(String::new())
    }
}

impl tracing::field::Visit for StringVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        } else {
            if !self.0.is_empty() {
                self.0.push_str(", ");
            }
            self.0.push_str(&format!("{}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        } else {
            if !self.0.is_empty() {
                self.0.push_str(", ");
            }
            self.0.push_str(&format!("{}={}", field.name(), value));
        }
    }
}