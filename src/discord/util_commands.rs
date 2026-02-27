use crate::discord::{Context, Error};
use crate::logger;

/// Test command (Deactivated)
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    logger::debug(format!("Ping command triggered by user: {}", ctx.author().name))
        .prefix("Discord")
        .send()
        .await;

    ctx.say("Pong! 🏓").await?;
    Ok(())
}