mod collectors;
mod commands;
mod event_handler;
mod reporters;

use crate::event_handler::{CommandHandlerKey, CommandHandlers};
use anyhow::Context;
use serenity::prelude::*;

#[derive(serde_derive::Deserialize, Debug, Copy, Clone)]
pub enum LevelFilter {
    OFF,
    ERROR,
    WARN,
    INFO,
    DEBUG,
    TRACE,
}

impl LevelFilter {
    fn to_log_level_filter(self) -> log::LevelFilter {
        match self {
            LevelFilter::OFF => log::LevelFilter::Off,
            LevelFilter::ERROR => log::LevelFilter::Error,
            LevelFilter::WARN => log::LevelFilter::Warn,
            LevelFilter::INFO => log::LevelFilter::Info,
            LevelFilter::DEBUG => log::LevelFilter::Debug,
            LevelFilter::TRACE => log::LevelFilter::Trace,
        }
    }
}

#[derive(Debug, serde_derive::Deserialize)]
struct Configuration {
    token: String,
    tools_channel: u64,
    log_level: LevelFilter,
    // for builtin_network_stats_monitor
    builtin_network_stats_monitor_channel: u64,
    builtin_network_stats_monitor_message: u64,
    // for builtin_minecraft_stats_monitor
    builtin_minecraft_stats_monitor_channel: u64,
    builtin_minecraft_stats_monitor_message: u64,
}

impl Configuration {
    pub fn tools_channel(&self) -> u64 {
        self.tools_channel
    }
}

struct ConfigurationTypeKey;

impl TypeMapKey for ConfigurationTypeKey {
    type Value = Configuration;
}

fn read_config() -> anyhow::Result<Configuration> {
    let file = std::fs::File::open("./etc/config.json")?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).context("Failed to load configuration.")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = read_config()?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] ([{}]/[{}]) {}",
                record.level(),
                chrono::Local::now().format("[%H:%M:%S]"),
                record.target(),
                message
            ))
        })
        .level((&config.log_level).to_log_level_filter())
        .chain(std::io::stdout())
        .apply()?;

    log::debug!("Read discord tracker config as: {config:#?}");

    let mut client = Client::builder(&config.token, GatewayIntents::all())
        .event_handler(event_handler::Handler)
        .await
        .expect("Error occurred creating client.");

    let mut data_write_lock = client.data.write().await;
    data_write_lock.insert::<ConfigurationTypeKey>(config);
    data_write_lock.insert::<CommandHandlerKey>(CommandHandlers::default());
    drop(data_write_lock);

    if let Err(err) = client.start().await {
        println!("Client error: {:?}", err);
    }
    Ok(())
}
