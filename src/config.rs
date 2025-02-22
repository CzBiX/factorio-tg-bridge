use std::{path::PathBuf, sync::LazyLock};

use clap::Parser;
use dotenv::dotenv;

#[derive(Parser)]
pub struct Config {
    #[arg(long, env)]
    pub telegram_token: String,
    #[arg(long, env)]
    pub telegram_chat_id: i64,

    #[arg(long, env, default_value = "127.0.0.1:27015")]
    pub rcon_host: String,
    #[arg(long, env)]
    pub rcon_password: String,

    #[arg(long, env)]
    pub factorio_log_file: PathBuf,
}

impl Config {
    pub(crate) fn new() -> Self {
        dotenv().ok();
        Self::parse()
    }
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::new);
