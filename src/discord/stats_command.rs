use chrono::Utc;
use poise::{command, CreateReply};
use crate::database::server::get_database_counts;
use crate::discord::{create_base_embed, create_error_embed, create_loading_embed, Context, Error};
use crate::logger;

/// Shows stats about the bot and database
#[command(slash_command)]
pub async fn stats(
    ctx: Context<'_>
) -> Result<(), Error> {
    let start_time = Utc::now();

    let reply = ctx.send(
        CreateReply::default()
            .embed(create_loading_embed("fetching stats"))
    ).await?;

    match get_database_counts().await {
        Ok((server_count, history_count, player_count)) => {
            let response = create_base_embed(Some(start_time))
                .title("📶 Stats")
                .description("Some stats about ServerRawler")
                .field("💻 Indexed servers", format!("--> `{}`", server_count), true)
                .field("📅 Server data entries", format!("--> `{}`", history_count), false)
                .field("👥 Player join entries", format!("--> `{}`", player_count), false);

            reply.edit(
                ctx,
                CreateReply::default()
                    .embed(response)
            ).await?;
        }
        Err(e) => {
            reply.edit(
                ctx,
                CreateReply::default()
                    .embed(create_error_embed("Database error", Some(start_time)))
            ).await?;
            logger::error(format!("Database stats error: {}", e)).prefix("Discord").send().await;
        }
    }

    Ok(())
}