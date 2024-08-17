use clap::Parser;
use serde::Deserialize;
use std::fs;

#[derive(Parser)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub bitcoin: BitcoinConfig,
    pub tgbot: TgBot,
}

#[derive(Deserialize, Debug)]
pub struct BitcoinConfig {
    pub zmq: String,
    pub zmq_port: u16,
}

#[derive(Deserialize, Debug)]
pub struct TgBot {
    pub token: String,
    pub chat_id: i64,
    pub sold_topic_id: i32,
    pub sniper_topic_id: i32,
    pub tx_topic_id: i32,
}

pub fn read_config() -> Config {
    let args = Cli::parse();
    load_config(&args.config)
}

fn load_config(path: &str) -> Config {
    let config_content = fs::read_to_string(path).expect("Failed to read config file");
    let config: Config = toml::from_str(&config_content).expect("Failed to parse config file");

    config
}

#[cfg(test)]
mod tests {
    use super::load_config;

    #[test]
    fn load_config_test() {
        let cfg_path = "example_config.toml";
        let cfg = load_config(cfg_path);

        println!("Server address: {}", cfg.bitcoin.zmq);
        println!("Server port: {}", cfg.bitcoin.zmq_port);
        println!("Database user: {}", cfg.tgbot.chat_id);
        println!("Database name: {}", cfg.tgbot.tx_topic_id);
    }
}
