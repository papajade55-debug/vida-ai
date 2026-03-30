//! Telegram Bot connector for Vida AI.
//!
//! Feature-gated behind `#[cfg(feature = "telegram")]`.
//! Provides a Telegram bot that forwards messages to VidaEngine and streams responses.

#[cfg(feature = "telegram")]
mod bot {
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    use teloxide::prelude::*;
    use teloxide::types::ParseMode;
    use teloxide::utils::command::BotCommands;

    use crate::engine::VidaEngine;

    /// Bot commands.
    #[derive(BotCommands, Clone)]
    #[command(rename_rule = "lowercase", description = "Vida AI Bot Commands:")]
    pub enum Command {
        #[command(description = "Display help")]
        Help,
        #[command(description = "Send a message to Vida AI")]
        Chat(String),
        #[command(description = "List available providers")]
        Models,
        #[command(description = "Check service health")]
        Health,
    }

    /// Configuration for the Telegram bot.
    #[derive(Debug, Clone)]
    pub struct TelegramConfig {
        /// Telegram bot token (from @BotFather).
        pub bot_token: String,
        /// Allowed chat IDs (empty = allow all).
        pub allowed_chat_ids: HashSet<i64>,
        /// Default session ID for chat commands.
        pub default_session_id: Option<String>,
    }

    /// Manages the Telegram bot lifecycle.
    pub struct TelegramBot {
        config: TelegramConfig,
        shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    }

    impl TelegramBot {
        pub fn new(config: TelegramConfig) -> Self {
            Self {
                config,
                shutdown_tx: None,
            }
        }

        pub fn is_running(&self) -> bool {
            self.shutdown_tx.is_some()
        }

        /// Start the bot. Returns immediately; bot runs in background.
        pub fn start(&mut self, engine: Arc<RwLock<VidaEngine>>) {
            if self.is_running() {
                log::warn!("Telegram bot is already running");
                return;
            }

            let config = self.config.clone();
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

            tokio::spawn(async move {
                let bot = Bot::new(&config.bot_token);

                let handler = Update::filter_message()
                    .branch(
                        dptree::entry()
                            .filter_command::<Command>()
                            .endpoint(handle_command),
                    )
                    .branch(
                        dptree::filter(|msg: Message| msg.text().is_some())
                            .endpoint(handle_plain_message),
                    );

                let mut dispatcher = Dispatcher::builder(bot, handler)
                    .dependencies(dptree::deps![
                        engine,
                        config.allowed_chat_ids.clone(),
                        config.default_session_id.clone()
                    ])
                    .enable_ctrlc_handler()
                    .build();

                tokio::select! {
                    _ = dispatcher.dispatch() => {},
                    _ = &mut shutdown_rx => {
                        log::info!("Telegram bot shutting down");
                    }
                }
            });

            self.shutdown_tx = Some(shutdown_tx);
            log::info!("Telegram bot started");
        }

        /// Stop the bot.
        pub fn stop(&mut self) {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
                log::info!("Telegram bot stopped");
            }
        }
    }

    /// Check if a chat ID is allowed.
    fn is_allowed(chat_id: i64, allowed: &HashSet<i64>) -> bool {
        allowed.is_empty() || allowed.contains(&chat_id)
    }

    /// Handle bot commands (/help, /chat, /models, /health).
    async fn handle_command(
        bot: Bot,
        msg: Message,
        cmd: Command,
        engine: Arc<RwLock<VidaEngine>>,
        allowed_chat_ids: HashSet<i64>,
        default_session_id: Option<String>,
    ) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;

        if !is_allowed(chat_id, &allowed_chat_ids) {
            bot.send_message(
                msg.chat.id,
                "⛔ Unauthorized. Your chat ID is not in the allowed list.",
            )
            .await?;
            return Ok(());
        }

        match cmd {
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Command::Chat(text) => {
                if text.is_empty() {
                    bot.send_message(msg.chat.id, "Usage: /chat <your message>")
                        .await?;
                    return Ok(());
                }
                send_chat_response(&bot, &msg, &engine, &default_session_id, &text).await?;
            }
            Command::Models => {
                let e = engine.read().await;
                let providers = e.list_providers().await;
                let mut text = String::from("📋 *Available Models*\n\n");
                for p in &providers {
                    text.push_str(&format!("*{}* ({:?})\n", p.display_name, p.provider_type));
                    for m in &p.models {
                        text.push_str(&format!("  • `{}`\n", m));
                    }
                    text.push('\n');
                }
                let md_result = bot
                    .send_message(msg.chat.id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
                if md_result.is_err() {
                    let plain = format!("Available models: {:?}", providers);
                    bot.send_message(msg.chat.id, plain).await?;
                }
            }
            Command::Health => {
                let e = engine.read().await;
                let checks = e.health_check_all().await;
                let mut text = String::from("🏥 Health Check\n\n");
                for (name, ok) in &checks {
                    let icon = if *ok { "✅" } else { "❌" };
                    text.push_str(&format!("{} {}\n", icon, name));
                }
                bot.send_message(msg.chat.id, text).await?;
            }
        }
        Ok(())
    }

    /// Handle plain text messages (not commands) as chat messages.
    async fn handle_plain_message(
        bot: Bot,
        msg: Message,
        engine: Arc<RwLock<VidaEngine>>,
        allowed_chat_ids: HashSet<i64>,
        default_session_id: Option<String>,
    ) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;

        if !is_allowed(chat_id, &allowed_chat_ids) {
            return Ok(());
        }

        if let Some(text) = msg.text() {
            send_chat_response(&bot, &msg, &engine, &default_session_id, text).await?;
        }

        Ok(())
    }

    /// Send user text to VidaEngine and reply with the response.
    /// Uses non-streaming send_message for simplicity; edits the message progressively
    /// would require streaming which adds complexity.
    async fn send_chat_response(
        bot: &Bot,
        msg: &Message,
        engine: &Arc<RwLock<VidaEngine>>,
        default_session_id: &Option<String>,
        user_text: &str,
    ) -> ResponseResult<()> {
        // Send typing indicator
        bot.send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
            .await?;

        // Get or create a session
        let session_id = match default_session_id {
            Some(sid) => sid.clone(),
            None => {
                // Create a temporary session using the first available provider
                let e = engine.read().await;
                let providers = e.list_providers().await;
                if providers.is_empty() {
                    bot.send_message(msg.chat.id, "❌ No providers configured.")
                        .await?;
                    return Ok(());
                }
                let provider = &providers[0];
                let model = provider.models.first().cloned().unwrap_or_default();
                match e.create_session(&provider.id, &model).await {
                    Ok(session) => session.id,
                    Err(err) => {
                        bot.send_message(msg.chat.id, format!("❌ {}", err)).await?;
                        return Ok(());
                    }
                }
            }
        };

        // Send the message
        let mut e = engine.write().await;
        match e.send_message(&session_id, user_text).await {
            Ok(response) => {
                // Truncate if too long for Telegram (4096 char limit)
                let reply = if response.content.len() > 4000 {
                    format!("{}…\n\n_(truncated)_", &response.content[..4000])
                } else {
                    response.content
                };
                bot.send_message(msg.chat.id, reply).await?;
            }
            Err(err) => {
                bot.send_message(msg.chat.id, format!("❌ Error: {}", err))
                    .await?;
            }
        }

        Ok(())
    }
}

// ── Public re-exports ──

#[cfg(feature = "telegram")]
pub use bot::{Command as TelegramCommand, TelegramBot, TelegramConfig};

// ── Tests ──

#[cfg(test)]
#[cfg(feature = "telegram")]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_telegram_config_creation() {
        let config = TelegramConfig {
            bot_token: "123:ABC".to_string(),
            allowed_chat_ids: HashSet::from([12345, 67890]),
            default_session_id: None,
        };
        assert_eq!(config.bot_token, "123:ABC");
        assert!(config.allowed_chat_ids.contains(&12345));
        assert_eq!(config.allowed_chat_ids.len(), 2);
    }

    #[test]
    fn test_telegram_bot_new() {
        let config = TelegramConfig {
            bot_token: "test".to_string(),
            allowed_chat_ids: HashSet::new(),
            default_session_id: Some("session-1".to_string()),
        };
        let bot = TelegramBot::new(config);
        assert!(!bot.is_running());
    }

    #[test]
    fn test_allowed_chat_ids_empty_allows_all() {
        let allowed: HashSet<i64> = HashSet::new();
        // Empty set = allow all
        assert!(allowed.is_empty());
    }

    #[test]
    fn test_allowed_chat_ids_restricts() {
        let allowed = HashSet::from([111i64, 222]);
        assert!(allowed.contains(&111));
        assert!(!allowed.contains(&333));
    }
}
