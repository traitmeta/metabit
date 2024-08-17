use anyhow::{Ok, Result};
use serde_json::json;
use teloxide::prelude::*;
use teloxide::types::Message;
use teloxide::types::{ChatId, MessageId, ThreadId};

pub struct TgBot {
    bot: Bot,
    chat_id: i64,
    topic_id: i32,
}

impl TgBot {
    pub fn new(token: &str, chat_id: i64, topic_id: i32) -> Self {
        Self {
            bot: Bot::new(token),
            chat_id,
            topic_id,
        }
    }

    pub async fn send_msg_to_topic(&self, msg: &str) -> Result<()> {
        let chat_id = ChatId(self.chat_id);
        let topic_id = ThreadId(MessageId(self.topic_id));
        self.bot
            .send_message(chat_id, msg)
            .message_thread_id(topic_id)
            .await?;
        Ok(())
    }

    pub async fn get_topic(&self) {
        teloxide::repl(
            self.bot.clone(),
            move |message: Message, bot: Bot| async move {
                eprintln!("Received message: {:?}", json!(message));
                if let Some(id) = message.thread_id {
                    println!("Received message in topic with ID: {}", id);
                    bot.send_message(
                        message.chat.id,
                        format!("我收到了你的消息. thread_id : {}", id).to_string(),
                    )
                    .message_thread_id(id)
                    .await
                    .log_on_error()
                    .await;
                } else {
                    bot.send_message(message.chat.id, "我收到了你的消息！")
                        .await
                        .log_on_error()
                        .await;
                }

                respond(())
            },
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_topic_test() {
        let bot = TgBot::new("", -1002235441155, 1);

        bot.get_topic().await;
    }
}
