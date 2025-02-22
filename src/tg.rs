use anyhow::{Ok, Result};
use teloxide::{
    prelude::*,
    types::{MessageId, ReplyParameters},
};
use tokio::sync::mpsc;
use tracing::info;

use crate::Event;
pub struct TgBot {
    chat_id: i64,
    bot: teloxide::Bot,
    sender: mpsc::Sender<Event>,
}

impl TgBot {
    pub fn new(token: String, chat_id: i64, sender: mpsc::Sender<Event>) -> Self {
        let bot = teloxide::Bot::new(token);

        Self {
            chat_id,
            bot,
            sender,
        }
    }

    pub async fn run(&self) -> Result<()> {
        use dptree;

        info!("Starting Telegram bot");

        let chat_id = self.chat_id;
        let sender = self.sender.clone();

        let handler = Update::filter_message()
            .filter(move |msg: Message| msg.chat.id.0 == chat_id)
            .branch(Message::filter_text().endpoint(
                |sender: mpsc::Sender<Event>, msg: Message| async move {
                    let text = msg.text().unwrap();
                    let name = &msg.from.as_ref().unwrap().first_name;
                    let event = if text.starts_with('/') {
                        Event::CommandFromTg {
                            id: msg.id.0,
                            cmd: text.to_string(),
                        }
                    } else {
                        Event::MessageFromTg(format!("{}: {}", name, text.to_string()))
                    };

                    sender.send(event).await.unwrap();

                    Ok(())
                },
            ))
            .branch(Message::filter_photo().endpoint(
                |sender: mpsc::Sender<Event>, msg: Message| async move {
                    let name = &msg.from.as_ref().unwrap().first_name;
                    let event = Event::MessageFromTg(format!("{}: [IMG]", name));

                    sender.send(event).await.unwrap();

                    Ok(())
                },
            ));

        Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![sender])
            .build()
            .dispatch()
            .await;

        Ok(())
    }

    pub async fn send_message(&self, msg: &str, slient: bool) -> Result<()> {
        self.bot
            .send_message(ChatId(self.chat_id), msg)
            .disable_notification(slient)
            .send()
            .await?;

        Ok(())
    }

    pub async fn reply_message(&self, id: i32, msg: &str) -> Result<()> {
        let reply_parameters = ReplyParameters::new(MessageId(id)).allow_sending_without_reply();
        self.bot
            .send_message(ChatId(self.chat_id), msg)
            .reply_parameters(reply_parameters)
            .send()
            .await?;

        Ok(())
    }
}
