use anyhow::Context;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;

pub fn ping_command_handle(
    ctx: serenity::client::Context,
    interaction: ApplicationCommandInteraction,
) -> crate::event_handler::ApplicationCommandFuture {
    Box::pin(async move {
        interaction
            .channel_id
            .send_message(&ctx.http, |message| message.content("Pong!"))
            .await
            .map(|_| ())
            .context("Failed to pong.")
    })
}

pub async fn configure(
    ctx: &serenity::client::Context,
    command_handles: &mut crate::event_handler::CommandHandlers,
) -> anyhow::Result<()> {
    Command::create_global_application_command(&ctx.http, |command| {
        command
            .name("ping")
            .description("Sends a ping to the bot and replies with a pong.")
    })
    .await?;
    command_handles.register_handle("ping", ping_command_handle);

    Ok(())
}
