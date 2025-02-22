mod config;
mod factorio;
mod tg;

use crate::config::CONFIG;
use anyhow::Result;
use tokio::{self, sync::mpsc};
use tracing::{info, warn};

#[derive(Debug)]
enum Event {
    MessageFromFactorio{
      msg: String,
      slient: bool,
    },
    MessageFromTg(String),
    CommandFromTg { id: i32, cmd: String },
}

async fn bus(factorio: &factorio::Factorio, tg: &tg::TgBot, mut rx: mpsc::Receiver<Event>) {
    while let Some(event) = rx.recv().await {
        info!("Event: {:?}", event);

        match event {
            Event::MessageFromFactorio { msg, slient } => {
                if let Err(err) = tg.send_message(msg.as_str(), slient).await {
                    warn!("Failed to send message to Telegram: {}", err);
                }
            }
            Event::MessageFromTg(msg) => {
                if let Err(err) = factorio.send_cmd(msg.as_str()).await {
                    warn!("Failed to send command to Factorio: {}", err);
                }
            }
            Event::CommandFromTg { id, cmd } => match factorio.send_cmd(&cmd).await {
                Ok(reply) => {
                    if let Err(err) = tg.reply_message(id, &reply).await {
                        warn!("Failed to send reply to Telegram: {}", err);
                    }
                }
                Err(err) => {
                    warn!("Failed to send command to Factorio: {}", err);
                }
            },
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("log file: {}", CONFIG.factorio_log_file.display());

    let (tx, rx) = mpsc::channel::<Event>(16);

    let reader = factorio::Factorio::new(
        &CONFIG.factorio_log_file,
        &CONFIG.rcon_host,
        &CONFIG.rcon_password,
        tx.clone(),
    );
    let tg = tg::TgBot::new(
        CONFIG.telegram_token.clone(),
        CONFIG.telegram_chat_id,
        tx.clone(),
    );

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl-C received, shutting down");
        }
        result = reader.run() => {
            result?;
        }
        result = tg.run() => {
            result?;
        }
        _ = bus(&reader, &tg, rx) => {

        }
    }

    Ok(())
}
