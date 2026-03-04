use std::fmt::format;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use poise::ReplyHandle;
use serenity::all::{AutoArchiveDuration, ButtonStyle, ChannelType, ComponentInteraction, ComponentInteractionCollector, CreateActionRow, CreateAttachment, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateThread, EditAttachments, EditInteractionResponse, EditMessage, Message};
use crate::database::{ServerHistory, ServerInfo};
use crate::discord::{create_base_embed, create_error_embed, create_loading_embed, Context, Error};
use crate::discord::actions::paginator::base_paginator;
use crate::logger;
use crate::minecraft::{join, ping, query};

// TODO: Add error msgs

pub fn convert_img_for_discord(server_history: &ServerHistory) -> Option<CreateAttachment> {
    if let Some(base64_icon) = server_history.icon.as_ref() {
        if let Ok(decoded_bytes) = BASE64_STANDARD.decode(base64_icon.trim()) {
            return Some(CreateAttachment::bytes(decoded_bytes, "server_icon.png"));
        }
    }
    None
}

pub fn build_manage_server_action_row(disabled: bool, history: &ServerHistory) -> Vec<CreateActionRow> {
    let show_plugins = history.plugins.as_ref().map_or(0, |p| p.len()) > 0;
    let show_mods = history.mods.as_ref().map_or(0, |m| m.len()) > 0;
    let show_player_sample = history.player_sample.as_ref().map_or(0, |p| p.len()) > 0;
    let show_players = history.players.as_ref().map_or(0, |p| p.len()) > 0;

    vec![
        CreateActionRow::Buttons(
            vec![
                CreateButton::new("view_plugins")
                    .label("View Plugins")
                    .style(ButtonStyle::Secondary)
                    .emoji('🧩')
                    .disabled(disabled || !show_plugins),
                CreateButton::new("view_mods")
                    .label("View Mods")
                    .style(ButtonStyle::Secondary)
                    .emoji('➕')
                    .disabled(disabled || !show_mods)
            ]
        ),
        CreateActionRow::Buttons(
            vec![
                CreateButton::new("view_sample")
                    .label("View Sample")
                    .style(ButtonStyle::Secondary)
                    .emoji('👥')
                    .disabled(disabled || !show_player_sample),
                CreateButton::new("view_players")
                    .label("View Players")
                    .style(ButtonStyle::Secondary)
                    .emoji('👥')
                    .disabled(disabled || !show_players)
            ]
        ),
        CreateActionRow::Buttons(
            vec![
                CreateButton::new("relookup")
                    .label("Relookup")
                    .style(ButtonStyle::Primary)
                    .emoji('🔄')
                    .disabled(disabled),
                CreateButton::new("view_history")
                    .label("History")
                    .style(ButtonStyle::Primary)
                    .emoji('📖')
                    .disabled(disabled)
            ]
        )
    ]
}

pub fn build_server_embed(start_time: DateTime<Utc>, info: &ServerInfo, history: &ServerHistory) -> CreateEmbed {
    let mut embed = create_base_embed(Some(start_time))
        .title("Server Overview")
        .color(0x2ecc71);

    let mut description = format!("Details for **{}:{}**", info.server_ip, info.server_port);

    let country = info.country.as_deref().unwrap_or("Unknown");

    embed = embed.field(
        "General",
        format!(
            "- **Country:** `{}`\n- **Type:** `{}`\n- **ID:** {}",
            country,
            if info.bedrock { "Bedrock" } else { "Java" },
            info.server_id.map(|id| id.to_string()).unwrap_or("Unknown".into())
        ),
        false
    );

    let online = history.player_online.unwrap_or(0);
    let max = history.player_max.unwrap_or(0);
    let latency = history.latency.map(|l| format!("{:.2}ms", l)).unwrap_or_else(|| "N/A".to_string());
    let last_seen = format!("<t:{}:R>", info.last_seen);
    let discovered = format!("<t:{}:R>", info.discovered);

    embed = embed.field(
        "Stats",
        format!(
            "- **Online:** {}/{}\n- **Latency:** {}\n- **Discovered:** {}\n- **Last Seen:** {}",
            online,
            max,
            latency,
            discovered,
            last_seen
        ),
        true
    );

    let version = history.version_name.as_deref().unwrap_or("Unknown");
    let protocol = history.version_protocol.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string());

    embed = embed.field(
        "Version",
        format!(
            "- **Version:** `{}`\n- **Protocol:** `{}`\n- **Secure Chat:** `{}`\n- **Software:** {}",
            version,
            protocol,
            history.enforces_secure_chat.map(|b| if b { "Yes" } else { "No" }).unwrap_or("Unknown"),
            history.software.clone().map(|s| format!("Name: `{}` Version: `{}`", s.name, s.version)).unwrap_or("`Unknown`".into())
        ),
        true
    );

    if history.is_modded_server.unwrap_or(false) {
        let mod_loader = history.mod_loader.clone().unwrap_or_else(|| "Unknown".into());
        let installed_mods = history.mods.clone().map(|m| m.len().to_string()).unwrap_or("Unknown".into());

        embed = embed.field(
            "Modding",
            format!("-# Modding server detected\n- **Modloader:** `{}`\n- **Mods:** `{}`", mod_loader, installed_mods),
            false
        );
    }

    if let Some(motd) = &history.plain_description {
        if !motd.trim().is_empty() {
            embed = embed.field("Description", format!("```\n{}\n```", motd), false);
        }
    }

    if history.icon.is_some() {
        embed = embed.thumbnail("attachment://server_icon.png");
    } else {
        description.push_str("\n*No icon detected*")
    }

    embed.description(description)
}

pub async fn create_one_server_action(
    start_time: DateTime<Utc>,
    ctx: Context<'_>,
    message: &mut Message,
    server_info: ServerInfo,
    server_history: ServerHistory
) -> Result<(), Error> {
    let mut response = EditMessage::new().embed(build_server_embed(start_time, &server_info, &server_history));

    if let Some(attachment) = convert_img_for_discord(&server_history) {
        response = response.attachments(EditAttachments::new().add(attachment));
    }

    message.edit(
        &ctx.serenity_context().http,
        response.clone().components(build_manage_server_action_row(false, &server_history))
    ).await?;

    let mut collector = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .message_id(message.id)
        .timeout(Duration::from_secs(240))
        .stream();

    while let Some(mci) = collector.next().await {
        handle_server_actions(ctx, &mci, &server_info, &server_history).await?;
    }

    message.edit(
        &ctx.serenity_context().http,
        response.components(build_manage_server_action_row(true, &server_history))
    ).await?;

    Ok(())
}

pub async fn handle_server_actions(
    ctx: Context<'_>,
    interaction: &ComponentInteraction,
    server_info: &ServerInfo,
    server_history: &ServerHistory,
) -> Result<(), Error> {
    match interaction.data.custom_id.as_str() {
        "relookup" => {
            let start_time = Utc::now();
            interaction.create_response(&ctx.http(), CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(create_loading_embed("Pinging the server (may take 6s)..."))
                    .components(vec![])
            )).await?;

            if let Ok(ip) = Ipv4Addr::from_str(server_info.server_ip.as_str()) {
                if let Ok(ping_result) = ping::execute_ping(ip, server_info.server_port, 0, Duration::from_secs(3)).await {
                    let query = query::execute_query(ip, server_info.server_port, Duration::from_secs(3), true).await.ok();
                    let protocol = ping_result.protocol_version.or(server_history.version_protocol).unwrap_or(767);
                    let join = join::execute_join_check(ip, server_info.server_port, Duration::from_secs(3), "ServerRawler", protocol).await.ok();

                    let (mut new_info, new_history) = crate::database::parse_server(ip, server_info.server_port, ping_result, query, join);

                    if crate::database::server::insert_servers(&vec![(new_info.clone(), new_history.clone())]).await.is_ok() {
                        new_info.discovered = server_info.discovered;
                        new_info.server_id = server_info.server_id;

                        let mut response = EditInteractionResponse::new()
                            .embed(build_server_embed(start_time, &new_info, &new_history))
                            .components(build_manage_server_action_row(false, &new_history));

                        if let Some(attachment) = convert_img_for_discord(&server_history) {
                            response = response.attachments(EditAttachments::new().add(attachment));
                        }

                        interaction.edit_response(
                            &ctx.serenity_context().http,
                            response
                        ).await?;

                        return Ok(());
                    }
                }
            }

            interaction.edit_response(
                &ctx.serenity_context().http,
                EditInteractionResponse::new()
                    .embed(create_error_embed("Failed to ping server", None))
                    .components(build_manage_server_action_row(false, server_history))
            ).await?;
        }

        "view_history" => {
            interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(create_loading_embed("coding this feature"))
                    .components(vec![])
            )).await?;
            // TODO: Give user the complete history of the server
            //  with paginator
        }

        "view_plugins" => {
            view_plugins(
                ctx,
                &interaction,
                &server_info,
                &server_history
            ).await?;
        }

        "view_mods" => {
            view_mods(
                ctx,
                &interaction,
                &server_info,
                &server_history
            ).await?;
        }
        "view_sample" => {
            view_sample(
                ctx,
                &interaction,
                &server_info,
                &server_history
            ).await?;
        }

        "view_players" => {
            view_players(
                ctx,
                &interaction,
                &server_info,
                &server_history
            ).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn create_view_thread(
    ctx: Context<'_>,
    interaction: &ComponentInteraction,
    title: &str
) -> Result<Message, Error> {
    interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Acknowledge).await?;

    let channel_id = interaction.channel_id;
    let message = &interaction.message;

    let thread_id = if let Some(thread) = message.thread.as_ref() {
        thread.id
    } else {
        let new_thread = channel_id.create_thread_from_message(
            &ctx.serenity_context().http,
            message.id,
            CreateThread::new("Detailed information's")
                .kind(ChannelType::PublicThread)
                .auto_archive_duration(AutoArchiveDuration::OneHour)
        ).await?;
        new_thread.id
    };

    let target_message = thread_id.send_message(
        &ctx.serenity_context().http,
        CreateMessage::new()
            .embed(create_loading_embed(&format!("loading {}...", title)))
    ).await?;

    Ok(target_message)
}
pub async fn view_plugins(
    ctx: Context<'_>,
    interaction: &ComponentInteraction,
    info: &ServerInfo,
    history: &ServerHistory,
) -> Result<(), Error> {
    let mut pages = Vec::new();
    if let Some(plugins) = &history.plugins {
        let lines: Vec<String> = plugins.iter()
            .map(|p| format!("- **{}** (`{}`)", p.name, p.version))
            .collect();

        for chunk in lines.chunks(15) {
            pages.push(create_base_embed(None)
                .title("🧩 Server Plugins", )
                .description(format!(
                    "-# {}:{}\n{}",
                    info.server_ip,
                    info.server_port,
                    chunk.join("\n")
                ))
                .color(0x3498db));
        }
    }

    if pages.is_empty() {
        interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("No plugins found.").ephemeral(true)
        )).await?;
        return Ok(());
    }

    let message = create_view_thread(ctx, interaction, "Plugins View").await?;

    let shard_ctx = ctx.serenity_context().clone();
    let author_id = ctx.author().id;

    tokio::spawn(async move {
        if let Err(e) = base_paginator(shard_ctx, author_id, message, pages).await {
            logger::error(format!("Paginator error: {:?}", e));
        }
    });

    Ok(())
}

pub async fn view_mods(
    ctx: Context<'_>,
    interaction: &ComponentInteraction,
    info: &ServerInfo,
    history: &ServerHistory,
) -> Result<(), Error> {
    let mut pages = Vec::new();
    if let Some(mods) = &history.mods {
        let lines: Vec<String> = mods.iter()
            .map(|m| format!("- **{}** (`{}`)", m.name, m.version))
            .collect();

        for chunk in lines.chunks(15) {
            pages.push(create_base_embed(None)
                .title("➕ Server Mods")
                .description(format!(
                    "-# {}:{}\n{}",
                    info.server_ip,
                    info.server_port,
                    chunk.join("\n")
                ))
                .color(0xe67e22));
        }
    }

    if pages.is_empty() {
        interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("No mods found.").ephemeral(true)
        )).await?;
        return Ok(());
    }

    let message = create_view_thread(ctx, interaction, "Mods View").await?;

    let shard_ctx = ctx.serenity_context().clone();
    let author_id = ctx.author().id;

    tokio::spawn(async move {
        if let Err(e) = base_paginator(shard_ctx, author_id, message, pages).await {
            logger::error(format!("Paginator error: {:?}", e));
        }
    });

    Ok(())
}

pub async fn view_sample(
    ctx: Context<'_>,
    interaction: &ComponentInteraction,
    info: &ServerInfo,
    history: &ServerHistory,
) -> Result<(), Error> {
    let mut pages = Vec::new();
    if let Some(sample) = &history.player_sample {
        let lines: Vec<String> = sample.iter()
            .map(|p| format!("- `{}` \n(`{}`)", p.name.as_deref().unwrap_or("Unknown"), p.uuid.as_deref().unwrap_or("N/A")))
            .collect();

        for chunk in lines.chunks(15) {
            pages.push(create_base_embed(None)
                .title("👥 Player Sample (Ping)")
                .description(format!(
                    "-# {}:{}\n{}",
                    info.server_ip,
                    info.server_port,
                    chunk.join("\n")
                ))
            );
        }
    }

    if pages.is_empty() {
        interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("No player sample found.").ephemeral(true)
        )).await?;
        return Ok(());
    }

    let message = create_view_thread(ctx, interaction, "Player Sample View").await?;

    let shard_ctx = ctx.serenity_context().clone();
    let author_id = ctx.author().id;

    tokio::spawn(async move {
        if let Err(e) = base_paginator(shard_ctx, author_id, message, pages).await {
            logger::error(format!("Paginator error: {:?}", e));
        }
    });

    Ok(())
}

pub async fn view_players(
    ctx: Context<'_>,
    interaction: &ComponentInteraction,
    info: &ServerInfo,
    history: &ServerHistory,
) -> Result<(), Error> {
    let mut pages = Vec::new();
    if let Some(players) = &history.players {
        let lines: Vec<String> = players.iter()
            .map(|p| format!("- `{}`", p.name.as_deref().unwrap_or("Unknown")))
            .collect();

        for chunk in lines.chunks(20) {
            pages.push(create_base_embed(None)
                .title("👥 Online Players (Query)")
                .description(format!(
                    "-# {}:{}\n{}",
                    info.server_ip,
                    info.server_port,
                    chunk.join("\n")
                ))
            );
        }
    }

    if pages.is_empty() {
        interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("No query player list available.").ephemeral(true)
        )).await?;
        return Ok(());
    }

    let message = create_view_thread(ctx, interaction, "Player List View").await?;

    let shard_ctx = ctx.serenity_context().clone();
    let author_id = ctx.author().id;

    tokio::spawn(async move {
        if let Err(e) = base_paginator(shard_ctx, author_id, message, pages).await {
            logger::error(format!("Paginator error: {:?}", e));
        }
    });

    Ok(())
}