use crate::embed;
use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use serenity::builder::CreateEmbed;
use serenity::model::application::command::{Command, CommandOptionType};
use serenity::model::application::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOptionValue,
};
use serenity::model::application::interaction::InteractionResponseType;
use serenity::utils::Color;

type ApplicationCommandFuture = crate::event_handler::ApplicationCommandFuture;

async fn ack_content<D: ToString>(
    ctx: &serenity::client::Context,
    interaction: &ApplicationCommandInteraction,
    content: D,
) -> anyhow::Result<()> {
    interaction
        .create_interaction_response(&ctx.http, |res| {
            res.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await
        .context("Failed to send interaction message.")
}

async fn ack_embed(
    ctx: &serenity::client::Context,
    interaction: &ApplicationCommandInteraction,
    embed: CreateEmbed,
) -> anyhow::Result<()> {
    interaction
        .create_interaction_response(&ctx.http, |res| {
            res.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.set_embed(embed))
        })
        .await
        .context("Failed to send interaction message.")
}

pub fn raw_call_command_handle(
    ctx: serenity::client::Context,
    interaction: ApplicationCommandInteraction,
) -> ApplicationCommandFuture {
    Box::pin(async move {
        let me = ctx.cache.current_user();
        let path_data = interaction
            .data
            .options
            .first()
            .unwrap()
            .resolved
            .as_ref()
            .unwrap();

        if let CommandDataOptionValue::String(path_data) = path_data {
            let response = minehut_api::rest::Call::new(path_data)
                .get::<serde_json::value::Value>()
                .await?;
            ack_content(&ctx, &interaction, format!("```json\n{response:#?}```")).await
        } else {
            embed!(failure {
                author {
                    name: (&me.name)
                    icon: (me.avatar_url().as_ref().unwrap())
                }
                description: ("**Failed to resolve path value.**")
                color: (Color::DARK_RED)
            });

            ack_embed(&ctx, &interaction, failure).await
        }
    })
}

pub fn mh_stats_command_handle(
    ctx: serenity::client::Context,
    interaction: ApplicationCommandInteraction,
) -> ApplicationCommandFuture {
    Box::pin(async move {
        let stats = minehut_api::prelude::get_simple_stats().await?;
        let me = ctx.cache.current_user();

        embed!(stats_embed {
            author {
                name: (&me.name)
                icon: (me.avatar_url().as_ref().unwrap())
            }
            description: ("**Minehut network stats result.**")
            field {
                name: ("Ram Usage")
                value: (format!("{}/{}", stats.ram_count, stats.ram_max))
                inline: false;
            }
            field {
                name: ("Player Count")
                value: (format!("{}", stats.player_count))
                inline: false;
            }
            field {
                name: ("Server Count")
                value: (format!("{}/{}", stats.server_count, stats.server_max))
                inline: false;
            }
            color: (Color::BLITZ_BLUE)
        });

        ack_embed(&ctx, &interaction, stats_embed).await
    })
}

//noinspection DuplicatedCode
pub fn server_command_handler(
    ctx: serenity::client::Context,
    interaction: ApplicationCommandInteraction,
) -> ApplicationCommandFuture {
    Box::pin(async move {
        let me = ctx.cache.current_user();
        let server_name = interaction
            .data
            .options
            .first()
            .unwrap()
            .resolved
            .as_ref()
            .unwrap();

        if let CommandDataOptionValue::String(server_name) = server_name {
            let response = minehut_api::rest::get_server_by_name(server_name).await;
            let ack = match response {
                Ok(server) => {
                    let naive = NaiveDateTime::from_timestamp(
                        (server.creation.unwrap_or(0.0) as i64) / 1_000,
                        0,
                    );
                    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
                    let date = datetime.format("%Y-%m-%d %H:%M:%S");

                    embed!(response_embed {
                        author {
                            name: ("Click here for more information")
                            icon: (me.avatar_url().as_ref().unwrap())
                            url: (format!("https://{}/server/{}?byName=true", std::env::var("MINEHUT_URL").unwrap_or_else(|_| "api.dev.minehut.com".to_string()), server_name))
                        }
                        description: (format!("**Successfully resolved server {}[{}]**", server.name.as_ref().unwrap_or(server_name), &server.id))
                        field {
                            name: ("Visibility")
                            value: (format!("{}", server.visibility.unwrap_or(false)))
                            inline: true;
                        }
                        field {
                            name: ("Suspended")
                            value: (format!("{}", server.suspended.unwrap_or(false)))
                            inline: true;
                        }
                        field {
                            name: ("Online")
                            value: (format!("{}", server.online.unwrap_or(false)))
                            inline: true;
                        }
                        field {
                            name: ("Server Plan")
                            value: (format!("{}", server.server_plan.unwrap_or_else(|| String::from("Unknown"))))
                            inline: true;
                        }
                        field {
                            name: ("Active Server Plan")
                            value: (format!("{}", server.active_server_plan.unwrap_or_else(|| String::from("Unknown"))))
                            inline: true;
                        }
                        field {
                            name: ("Server Version Type")
                            value: (format!("{}", server.server_version_type.unwrap_or_else(|| String::from("Unknown"))))
                            inline: true;
                        }
                        field {
                            name: ("Player Count")
                            value: (format!("{}", server.player_count.unwrap_or(0.0)))
                            inline: true;
                        }
                        field {
                            name: ("Max Players")
                            value: (format!("{}", server.max_players.unwrap_or(0.0)))
                            inline: true;
                        }
                        field {
                            name: ("Last Online")
                            value: (format!("{}", date))
                            inline: true;
                        }
                        field {
                            name: ("Categories")
                            value: (format!("[{}]", server.categories.unwrap_or_default().join(", ")))
                            inline: true;
                        }
                        field {
                            name: ("Inherited Categories")
                            value: (format!("[{}]", server.inherited_categories.unwrap_or_default().join(", ")))
                            inline: true;
                        }
                        field {
                            name: ("MOTD")
                            value: (format!("{}", server.motd.unwrap_or_else(|| String::from("N/A"))))
                            inline: false;
                        }
                        color: (Color::BLITZ_BLUE)
                    });
                    if server.proxy.unwrap_or(false) {
                        response_embed.field(
                            "Connected Servers",
                            format!(
                                "[{}]",
                                server.connected_servers.unwrap_or_default().join(", ")
                            ),
                            false,
                        );
                    }
                    response_embed
                }
                Err(err) => {
                    embed!(response_embed {
                        author {
                            name: ("Click here for error information")
                            icon: (me.avatar_url().as_ref().unwrap())
                            url: (format!("https://{}/server/{}?byName=true", std::env::var("MINEHUT_URL").unwrap_or_else(|_| "api.dev.minehut.com".to_string()), server_name))
                        }
                        description: (format!("**Failed to resolve server {server_name}.**"))
                        color: (Color::DARK_RED)
                    });
                    log::warn!("Potential error resolving servers: {err:?}");
                    response_embed
                }
            };

            ack_embed(&ctx, &interaction, ack).await
        } else {
            embed!(failure {
                author {
                    name: (&me.name)
                    icon: (me.avatar_url().as_ref().unwrap())
                }
                description: ("**Failed to resolve server value.**")
                color: (Color::DARK_RED)
            });

            ack_embed(&ctx, &interaction, failure).await
        }
    })
}

pub async fn configure(
    ctx: &serenity::client::Context,
    command_handles: &mut crate::event_handler::CommandHandlers,
) -> anyhow::Result<()> {
    Command::create_global_application_command(&ctx.http, |command| {
        command
            .name("raw_call")
            .description("Sends a raw call to Minehut's API.")
            .create_option(|option| {
                option
                    .name("path")
                    .description("URL Path to call, ex: https://api.minehut.com{path}")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
    })
    .await?;
    command_handles.register_handle("raw_call", raw_call_command_handle);

    Command::create_global_application_command(&ctx.http, |command| {
        command
            .name("mh_stats")
            .description("Resolves Minehut's stats from the API.")
    })
    .await?;
    command_handles.register_handle("mh_stats", mh_stats_command_handle);

    Command::create_global_application_command(&ctx.http, |command| {
        command
            .name("server")
            .description("Queries a server from Minehut's API.")
            .create_option(|option| {
                option
                    .name("server")
                    .description("A server to query.")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
    })
    .await?;
    command_handles.register_handle("server", server_command_handler);

    Ok(())
}
