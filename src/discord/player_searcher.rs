use std::time::Duration;
use chrono::Utc;
use poise::CreateReply;
use serenity::all::{ButtonStyle, ComponentInteractionCollector, CreateInteractionResponse, EditMessage};
use serenity::builder::{CreateActionRow, CreateButton};
use crate::database::player;
use crate::discord::{create_base_embed, create_error_embed, create_loading_embed, Context, Error};
use crate::discord::actions::paginator::{base_paginator, player_paged_view};

/// Search a player in the database
#[poise::command(slash_command,
    description_localized("de", "Suche einen Spieler in der Datenbank")
)]
pub async fn search_player(
    ctx: Context<'_>,
    #[description = "Username of the player"]
    #[description_localized("de", "Spielername von dem Spieler")]
    username: String
) -> Result<(), Error> {
    let start_time = Utc::now();

    let reply = ctx.send(
        CreateReply::default()
            .embed(create_loading_embed("searching player..."))
    ).await?;

    match player::get_player_by_username(&username).await {
        Ok(Some(history_entries)) => {
            let latest = &history_entries[0];

            let overview_embed = create_base_embed(Some(start_time))
                .title(format!("Player Overview: {}", latest.username))
                .thumbnail(format!("https://mc-heads.net/body/{}", latest.uuid))
                .field("Latest UUID", format!("`{}`", latest.uuid), false)
                .field("Total Entries", history_entries.len().to_string(), true)
                .field("First Seen", format!("<t:{}:R>", history_entries.last().unwrap().seen), true);

            let make_components = |disabled| vec![CreateActionRow::Buttons(vec![
                CreateButton::new("view_history")
                    .label("View Detailed History")
                    .style(ButtonStyle::Primary)
                    .emoji('📖')
                    .disabled(disabled)
            ])];

            reply.edit(ctx, CreateReply::default()
                .embed(overview_embed.clone())
                .components(make_components(false))
            ).await?;

            let mut message = reply.into_message().await?;

            let mut collector = ComponentInteractionCollector::new(ctx.serenity_context())
                .author_id(ctx.author().id)
                .message_id(message.id)
                .timeout(Duration::from_secs(60))
                .next()
                .await;

            let interaction = match collector {
                Some(i) => i,
                None => {
                    message.edit(ctx, EditMessage::new()
                        .embed(overview_embed)
                        .components(make_components(true))
                    ).await?;

                    return Ok(())
                }
            };

            interaction.create_response(
                &ctx.http(),
                CreateInteractionResponse::Acknowledge
            ).await?;

            player_paged_view(
                ctx,
                ctx.author().id,
                message,
                history_entries
            ).await?;
        },
        Ok(None) => {
            let error_embed = create_error_embed(&format!("Player `{}` not found in database.", username), Some(start_time));
            reply.edit(ctx, CreateReply::default().embed(error_embed)).await?;
        },
        Err(_) => {
            let error_embed = create_error_embed("Database error during player search.", Some(start_time));
            reply.edit(ctx, CreateReply::default().embed(error_embed)).await?;
        }
    }

    Ok(())
}