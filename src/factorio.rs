use anyhow::{Ok, Result};
use lazy_regex::regex_captures;
use rcon;
use std::{path::PathBuf, str};
use tokio::sync::mpsc;
use tracing::info;

use linemux::MuxedLines;

use crate::Event;

pub struct Factorio {
    log_file: PathBuf,
    rcon_host: String,
    rcon_pwd: String,
    sender: mpsc::Sender<Event>,
}

#[derive(PartialEq, Eq, Debug)]
enum LogRecord {
    Chat { username: String, msg: String },
    Join(String),
    Leave(String),
}

fn parse_log_line(line: &str) -> Option<LogRecord> {
    match line {
        chat_line if chat_line.contains("[CHAT]") => {
            let (_, user, msg) = regex_captures!(r"\[CHAT\] (\w+|<server>): (.+)", chat_line)?;

            if user == "<server>" {
                None
            } else {
                Some(LogRecord::Chat {
                    username: user.to_string(),
                    msg: msg.to_string(),
                })
            }
        }
        join_line if join_line.contains("[JOIN]") => {
            let (_, user) = regex_captures!(r"\[JOIN\] (\w+) joined", join_line)?;
            Some(LogRecord::Join(user.to_string()))
        }
        leave_line if leave_line.contains("[LEAVE]") => {
            let (_, user) = regex_captures!(r"\[LEAVE\] (\w+) left", leave_line)?;
            Some(LogRecord::Leave(user.to_string()))
        }
        _ => None,
    }
}

impl Factorio {
    pub fn new(
        file: impl Into<PathBuf>,
        rcon_host: impl Into<String>,
        rcon_pwd: impl Into<String>,
        sender: mpsc::Sender<Event>,
    ) -> Self {
        Self {
            log_file: file.into(),
            rcon_host: rcon_host.into(),
            rcon_pwd: rcon_pwd.into(),
            sender,
        }
    }

    async fn read_line(&self, line: &str) -> Result<()> {
        let record = match parse_log_line(line) {
            Some(record) => record,
            None => return Ok(()),
        };

        let event = match record {
            LogRecord::Chat { username, msg } => Event::MessageFromFactorio {
                msg: format!("💬{}: {}", username, msg),
                slient: false,
            },
            LogRecord::Join(user) => Event::MessageFromFactorio {
                msg: format!("😊{} joined", user),
                slient: true,
            },
            LogRecord::Leave(user) => Event::MessageFromFactorio {
                msg: format!("👋{} left", user),
                slient: true,
            },
        };

        self.sender.send(event).await?;

        Ok(())
    }

    async fn read_log(&self) -> Result<()> {
        let mut lines = MuxedLines::new()?;
        lines.add_file(&self.log_file).await?;

        while let Some(line) = lines.next_line().await? {
            self.read_line(line.line()).await?;
        }

        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting Factorio reader");

        self.read_log().await
    }

    pub async fn send_cmd(&self, cmd: &str) -> Result<String> {
        let mut conn = rcon::Builder::new()
            .enable_factorio_quirks(true)
            .connect(&self.rcon_host, &self.rcon_pwd)
            .await?;
        let resp = conn.cmd(cmd).await?;

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        assert_eq!(
            parse_log_line("[CHAT] Foo: bar"),
            Some(LogRecord::Chat {
                username: "Foo".to_string(),
                msg: "bar".to_string()
            })
        );
        assert_eq!(
            parse_log_line("[JOIN] user joined"),
            Some(LogRecord::Join("user".to_string()))
        );
        assert_eq!(
            parse_log_line("[LEAVE] user left"),
            Some(LogRecord::Leave("user".to_string()))
        );
        assert_eq!(parse_log_line("[CHAT] <server>: hi"), None);
    }
}
