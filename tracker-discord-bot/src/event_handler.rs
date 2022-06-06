use super::commands;
use serenity::client::{Context, EventHandler};
use serenity::futures::future::BoxFuture;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::{TypeMap, TypeMapKey};
use serenity::utils::MessageBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLockWriteGuard;

pub struct CommandHandlerKey;

impl TypeMapKey for CommandHandlerKey {
    type Value = CommandHandlers;
}

pub struct Handler;

async fn configure_commands(
    ctx: &Context,
    mut type_map: RwLockWriteGuard<'_, TypeMap>,
) -> anyhow::Result<()> {
    let command_handles = type_map.get_mut::<CommandHandlerKey>().unwrap();
    commands::general::configure(ctx, command_handles).await?;
    commands::minehut::configure(ctx, command_handles).await?;
    Ok(())
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);

        crate::reporters::configure(Arc::clone(&ctx.data)).await;
        crate::collectors::configure(
            Arc::clone(&ctx.data),
            (Arc::clone(&ctx.cache), Arc::clone(&ctx.http)),
        )
        .await;

        let type_map = ctx.data.write().await;
        match configure_commands(&ctx, type_map).await {
            Ok(_) => {
                log::info!("Commands successfully registered.");
            }
            Err(err) => {
                log::error!("Failed to configure commands: {err:?}");
                return;
            }
        }

        let type_map = ctx.data.read().await;
        let config = type_map.get::<crate::ConfigurationTypeKey>().unwrap();
        let channel_id = ChannelId::from(config.tools_channel());

        let bot_ready = MessageBuilder::new()
            .push_bold_safe("MHTools Bot")
            .push(" is now online and ready to go.")
            .build();

        if let Err(why) = channel_id.say(&ctx.http, &bot_ready).await {
            log::error!("Error sending message: {:?}", why);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let type_map = ctx.data.read().await;
            let config = type_map.get::<crate::ConfigurationTypeKey>().unwrap();
            let channel_id = ChannelId::from(config.tools_channel());
            if channel_id.ne(&command.channel_id) {
                return command
                    .create_interaction_response(&ctx.http, |res| {
                        res.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.content("Invalid channel for MHTools.")
                            })
                    })
                    .await
                    .unwrap_or(());
            }
            drop(type_map);

            let command_name = String::from(&command.data.name);
            let type_map = ctx.data.read().await;
            let command_handles = type_map.get::<CommandHandlerKey>().unwrap();

            if let Some(function) = command_handles.get_handle(&command_name) {
                drop(type_map);
                match function(ctx, command).await {
                    Ok(_) => log::debug!("Command {command_name} processed successfully."),
                    Err(err) => log::error!("Command {command_name} failed. {err:?}"),
                }
            } else {
                log::warn!("Found empty handler for command interaction ({command_name})")
            }
        }
    }
}

pub type ApplicationCommandFuture = BoxFuture<'static, anyhow::Result<()>>;
pub type ApplicationCommandHandle =
    fn(Context, ApplicationCommandInteraction) -> ApplicationCommandFuture;

#[derive(Default)]
pub struct CommandHandlers {
    function_handler_map: HashMap<String, ApplicationCommandHandle>,
}

impl CommandHandlers {
    pub fn register_handle<S: Into<String>>(
        &mut self,
        name: S,
        function: ApplicationCommandHandle,
    ) {
        let name = name.into();
        log::info!("Registered command {}", &name);
        self.function_handler_map.insert(name, function);
    }

    pub fn get_handle<S: Into<String>>(&self, name: S) -> Option<ApplicationCommandHandle> {
        self.function_handler_map.get(&name.into()).copied()
    }
}
