use std::time::Duration;
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use poise::{CreateReply, ReplyHandle};
use serenity::all::{ButtonStyle, ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter};
use crate::database::{ServerHistory, ServerInfo};
use crate::discord::actions::server::{build_manage_server_action_row, build_server_embed};
use crate::discord::{create_base_embed, Context};

fn create_action_row(page: usize, total: usize, disabled: bool) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new("first")
            .emoji('⏮')
            .style(ButtonStyle::Secondary)
            .disabled(disabled || page == 0),
        CreateButton::new("prev")
            .emoji('◀')
            .style(ButtonStyle::Secondary)
            .disabled(disabled || page == 0),
        CreateButton::new("next")
            .emoji('▶')
            .style(ButtonStyle::Secondary)
            .disabled(disabled || page == total - 1),
        CreateButton::new("last")
            .emoji('⏭')
            .style(ButtonStyle::Secondary)
            .disabled(disabled || page == total - 1),
    ])]
}

async fn base_paginator(
    ctx: Context<'_>,
    reply: ReplyHandle<'_>,
    pages: Vec<CreateEmbed>,
) -> Result<(), serenity::Error> {
    let mut current_page = 0;
    let total_pages = pages.len();

    let get_page_embed = |idx: usize, embeds: &Vec<CreateEmbed>| {
        embeds[idx].clone().footer(CreateEmbedFooter::new(format!(
            "ServerRawler {} • Page {}/{}",
            crate::get_version_raw(),
            idx + 1,
            total_pages
        )))
    };

    loop {
        let current_embed = get_page_embed(current_page, &pages);

        reply.edit(ctx, CreateReply::default()
            .embed(current_embed.clone())
            .components(create_action_row(current_page, total_pages, false))
        ).await?;

        let interaction = ComponentInteractionCollector::new(ctx.serenity_context())
            .author_id(ctx.author().id)
            .message_id(reply.message().await?.id)
            .timeout(Duration::from_secs(60))
            .next()
            .await;

        let mci = match interaction {
            Some(i) => i,
            None => break,
        };

        match mci.data.custom_id.as_str() {
            "first" => current_page = 0,
            "prev" => if current_page > 0 { current_page -= 1 },
            "next" => if current_page < total_pages - 1 { current_page += 1 },
            "last" => current_page = total_pages - 1,
            _ => {
                mci.defer(&ctx.serenity_context()).await?;
                continue;
            }
        }

        mci.create_response(
            &ctx.serenity_context(),
            serenity::all::CreateInteractionResponse::UpdateMessage(
                serenity::all::CreateInteractionResponseMessage::new()
                    .embed(get_page_embed(current_page, &pages))
                    .components(create_action_row(current_page, total_pages, false))
            )
        ).await?;
    }

    let final_embed = get_page_embed(current_page, &pages);
    reply.edit(ctx, CreateReply::default()
        .embed(final_embed)
        .components(create_action_row(current_page, total_pages, true))
    ).await?;

    Ok(())
}

// pub async fn create_paged_server_view(
//     ctx: Context<'_>,
//     reply: ReplyHandle<'_>,
//     servers: &Vec<(ServerInfo, ServerHistory)>
// ) -> Result<(), serenity::Error> {
//     if servers.is_empty() {
//         reply.edit(ctx, CreateReply::default()
//             .embed(create_base_embed(None)
//             .title("🔎 Search Results")
//             .description("No servers found matching your filters.")
//             .color(0xff0000)
//             )
//         ).await?;
//         return Ok(());
//     }
//
//     let start_time = Utc::now();
//
//     let pages: Vec<CreateEmbed> = servers
//         .iter()
//         .map(|(info, history)| build_server_embed(start_time, info, history))
//         .collect();
//     base_paginator(ctx, reply, pages).await
// }

pub async fn create_paged_server_view(
    ctx: Context<'_>,
    reply: ReplyHandle<'_>,
    servers: &Vec<(ServerInfo, ServerHistory)>
) -> Result<(), serenity::Error> {
    if servers.is_empty() {
        reply.edit(ctx, CreateReply::default()
            .embed(crate::discord::create_base_embed(None)
                .title("🔎 Search Results")
                .description("No servers found matching your filters.")
                .color(0xff0000))
        ).await?;
        return Ok(());
    }

    let mut current_page = 0;
    let total_pages = servers.len();
    let start_time = Utc::now();

    loop {
        let (info, history) = &servers[current_page];

        let mut embed = build_server_embed(start_time, info, history);
        embed = embed.footer(CreateEmbedFooter::new(format!(
            "ServerRawler {} • Page {}/{}",
            crate::get_version_raw(),
            current_page + 1,
            total_pages
        )));

        let mut components = build_manage_server_action_row(false, history);
        components.extend(create_action_row(current_page, total_pages, false));

        let mut edit_reply = CreateReply::default()
            .embed(embed)
            .components(components);

        if let Some(base64_icon) = &history.icon {
            if let Ok(decoded_bytes) = base64::prelude::BASE64_STANDARD.decode(base64_icon.trim()) {
                edit_reply = edit_reply.attachment(serenity::all::CreateAttachment::bytes(decoded_bytes, "server_icon.png"));
            }
        }

        reply.edit(ctx, edit_reply).await?;

        let interaction = ComponentInteractionCollector::new(ctx.serenity_context())
            .author_id(ctx.author().id)
            .message_id(reply.message().await?.id)
            .timeout(Duration::from_secs(60))
            .next()
            .await;

        let mci = match interaction {
            Some(i) => i,
            None => break,
        };

        match mci.data.custom_id.as_str() {
            "first" => current_page = 0,
            "prev" => if current_page > 0 { current_page -= 1 },
            "next" => if current_page < total_pages - 1 { current_page += 1 },
            "last" => current_page = total_pages - 1,

            "relookup" | "show_plugins" | "show_mods" | "show_player_sample" | "show_players" => {
                // TODO: refresh server view
                //crate::discord::actions::server::handle_server_interaction(ctx, &mci, info, history).await?;
                continue;
            }
            _ => {
                mci.defer(&ctx.serenity_context().http).await?;
                continue;
            }
        }
    }

    let (info, history) = &servers[current_page];

    let mut final_components = build_manage_server_action_row(true, history);
    final_components.extend(create_action_row(current_page, total_pages, true));

    reply.edit(ctx, CreateReply::default().components(final_components)).await?;

    Ok(())
}