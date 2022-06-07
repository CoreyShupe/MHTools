use crate::{embed, TypeMap};
use serenity::cache::Cache;
use serenity::http::Http;
use serenity::model::id::{ChannelId, MessageId};
use serenity::utils::Color;
use std::ops::Div;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

const POLL_PERIOD_SIZE: u8 = 100u8;

struct NetworkStatsSheet {
    player_count: usize,
    server_count: usize,
    latency: u128,
    successful_calls: u8,
}

//noinspection ALL
pub async fn setup(type_map: Arc<RwLock<TypeMap>>, cache_and_http: (Arc<Cache>, Arc<Http>)) {
    let read_lock = type_map.read().await;

    let configuration = read_lock.get::<crate::ConfigurationTypeKey>().unwrap();
    let channel = ChannelId::from(configuration.builtin_minecraft_stats_monitor_channel);
    let message = MessageId::from(configuration.builtin_minecraft_stats_monitor_message);
    let receiver = read_lock
        .get::<crate::reporters::MinecraftStatsReporterKey>()
        .unwrap()
        .clone();
    drop(read_lock);

    let mut poll_index = 0u8;
    let mut past_network_sheet: Option<NetworkStatsSheet> = None;
    let mut network_info = Vec::with_capacity(POLL_PERIOD_SIZE as usize);
    let mut last_poll_time = Instant::now();

    log::info!(target: "MinecraftStats/Collector", "Network stats collector looping");
    while let Ok(response) = receiver.recv_async().await {
        network_info.push(response);
        poll_index += 1;
        log::info!(target: "MinecraftStats/Collector", "Received a network stats event.");

        if poll_index >= POLL_PERIOD_SIZE {
            log::info!(target: "MinecraftStats/Collector", "Emitting network stats");
            let sheet = network_info
                .iter()
                .fold((0, 0, 0, 0), |accum, item| match item {
                    None => accum,
                    Some(stats) => (
                        accum.0 + stats.players,
                        accum.1 + stats.servers,
                        accum.2 + stats.latency,
                        accum.3 + 1,
                    ),
                });
            let true_poll_size = (POLL_PERIOD_SIZE - (POLL_PERIOD_SIZE - sheet.3)) as usize;
            let sheet = NetworkStatsSheet {
                player_count: sheet.0 / true_poll_size,
                server_count: sheet.1 / true_poll_size,
                latency: sheet.2 / (true_poll_size as u128),
                successful_calls: sheet.3,
            };

            let me = cache_and_http.0.current_user();

            let header = format!(
                r#"**Minehut Network Minecraft Monitor**

                _Minecraft Query_: `mh-prd.minehut.com:25565`

                _Latency_: `{}ms`

                _Time since last update_: `{} seconds`

                _Time since last embed update_: <t:{}:R>

                _Successful Operations_: `({}/{POLL_PERIOD_SIZE})`
                "#,
                sheet.latency,
                last_poll_time.elapsed().as_secs(),
                crate::minecraft_bot::get_system_time_as_millis() / 1000,
                sheet.successful_calls
            );

            let embed = match past_network_sheet {
                None => {
                    embed!(embed {
                        author {
                            name: (&me.name)
                            icon: (me.avatar_url().as_ref().unwrap())
                        }
                        description: (header)
                        field {
                            name: ("Player Count")
                            value: (format!("{}", sheet.player_count))
                            inline: true;
                        }
                        field {
                            name: ("Server Count")
                            value: (format!("{}", sheet.server_count))
                            inline: true;
                        }
                        color: (Color::BLITZ_BLUE)
                    });
                    embed
                }
                Some(past_network_sheet) => {
                    let player_count_deviation = (f64::div(
                        sheet.player_count as f64,
                        past_network_sheet.player_count as f64,
                    ) * 100.0)
                        - 100.0;
                    let server_count_deviation = (f64::div(
                        sheet.server_count as f64,
                        past_network_sheet.server_count as f64,
                    ) * 100.0)
                        - 100.0;
                    embed!(embed {
                        author {
                            name: (&me.name)
                            icon: (me.avatar_url().as_ref().unwrap())
                        }
                        description: (header)
                        field {
                            name: ("Player Count (AVG)")
                            value: (format!("{}", sheet.player_count))
                            inline: true;
                        }
                        field {
                            name: ("Last Player Count (AVG)")
                            value: (format!("{}", past_network_sheet.player_count))
                            inline: true;
                        }
                        field {
                            name: ("Player Count Deviation")
                            value: (format!("{}{:.2}%", if player_count_deviation > 0.0 {
                                "+"
                            } else {
                                ""
                            }, player_count_deviation))
                            inline: true;
                        }
                        field {
                            name: ("Server Count (AVG)")
                            value: (format!("{}", sheet.server_count))
                            inline: true;
                        }
                        field {
                            name: ("Last Server Count (AVG)")
                            value: (format!("{}", past_network_sheet.server_count))
                            inline: true;
                        }
                        field {
                            name: ("Server Count Deviation")
                            value: (format!("{}{:.2}%", if server_count_deviation > 0.0 {
                                "+"
                            } else {
                                ""
                            }, server_count_deviation))
                            inline: true;
                        }
                        color: (Color::BLITZ_BLUE)
                    });
                    embed
                }
            };

            if let Err(err) = channel
                .edit_message(&cache_and_http.1, message, |message| {
                    message.content("").set_embed(embed)
                })
                .await
            {
                log::error!(target: "MinecraftStats/Collector", "Error editing network stats monitor message: {err:?}");
            }

            past_network_sheet = Some(sheet);
            poll_index = 0;
            network_info.clear();
            last_poll_time = Instant::now()
        }
    }
    log::error!(target: "MinecraftStats/Collector", "Some error occurred during processing of flume messenger.");
}
