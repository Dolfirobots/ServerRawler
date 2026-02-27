use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use poise::ReplyHandle;
use serenity::all::{ButtonStyle, ComponentInteractionCollector, CreateActionRow, CreateAttachment, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, EditAttachments, EditInteractionResponse, EditMessage};
use crate::database::{ServerHistory, ServerInfo};
use crate::discord::{create_base_embed, create_error_embed, create_loading_embed, Context};
use crate::logger;
use crate::minecraft::{join, ping, query};

// TODO: Add error msgs
fn build_manage_server_action_row(disabled: bool, history: &ServerHistory) -> Vec<CreateActionRow> {
    let show_plugins = history.plugins.as_ref().map_or(0, |p| p.len()) > 0;
    let show_mods = history.mods.as_ref().map_or(0, |m| m.len()) > 0;
    let show_player_sample = history.player_sample.as_ref().map_or(0, |p| p.len()) > 0;
    let show_players = history.players.as_ref().map_or(0, |p| p.len()) > 0;

    vec![
        CreateActionRow::Buttons(
            vec![
                CreateButton::new("show_plugins")
                    .label("View Plugins")
                    .style(ButtonStyle::Secondary)
                    .emoji('🧩')
                    .disabled(disabled || !show_plugins),
                CreateButton::new("show_mods")
                    .label("View Mods")
                    .style(ButtonStyle::Secondary)
                    .emoji('➕')
                    .disabled(disabled || !show_mods),
                CreateButton::new("show_player_sample")
                    .label("View Sample")
                    .style(ButtonStyle::Secondary)
                    .emoji('👥')
                    .disabled(disabled || !show_player_sample),
                CreateButton::new("show_players")
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
                CreateButton::new("show_history")
                    .label("History")
                    .style(ButtonStyle::Primary)
                    .emoji('📖')
                    .disabled(disabled)
            ]
        )
    ]
}

fn build_server_embed(start_time: DateTime<Utc>, info: &ServerInfo, history: &ServerHistory) -> CreateEmbed {
    let mut embed = create_base_embed(Some(start_time))
        .title("Server Overview")
        .color(0x2ecc71);

    let mut description = format!("Details for **{}:{}**", info.server_ip, info.server_port);

    let country = info.country.as_deref().unwrap_or("Unknown");
    let last_seen = format!("<t:{}:R>", info.last_seen);
    let discovered = format!("<t:{}:R>", info.discovered);

    embed = embed.field(
        "General",
        format!(
            "- **Country:** `{}`\n- **Type:** `{}`\n- **Discovered:** {}\n- **Last Seen:** {}\n- **ID:** {}",
            country,
            if info.bedrock { "Bedrock" } else { "Java" },
            discovered,
            last_seen,
            info.server_id.map(|id| id.to_string()).unwrap_or("Unknown".into())
        ),
        false
    );

    let online = history.player_online.unwrap_or(0);
    let max = history.player_max.unwrap_or(0);
    let latency = history.latency.map(|l| format!("{:.2}ms", l)).unwrap_or_else(|| "N/A".to_string());

    embed = embed.field(
        "Stats",
        format!("- **Online:** {}/{}\n- **Latency:** {}", online, max, latency),
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
    reply: ReplyHandle<'_>,
    server_info: ServerInfo,
    server_history: ServerHistory
) -> Result<(), serenity::Error> {

    let mut response = EditMessage::new().embed(build_server_embed(start_time, &server_info, &server_history));

    if let Some(base64_icon) = &server_history.icon {
        if let Ok(decoded_bytes) = BASE64_STANDARD.decode(base64_icon.trim()) {
            let attachment = CreateAttachment::bytes(decoded_bytes, "server_icon.png");
            response = response.attachments(EditAttachments::new().add(attachment.clone()));
        }
    }

    // Using the message.edit() because reply.edit() doesn't works correctly with attachments
    let mut message = reply.message().await?.into_owned();

    message.edit(
        &ctx.serenity_context().http,
        response.clone().components(build_manage_server_action_row(false, &server_history))
    ).await?;

    let mut collector = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .message_id(reply.message().await?.id)
        .timeout(Duration::from_secs(240))
        .stream();

    while let Some(mci) = collector.next().await {
        match mci.data.custom_id.as_str() {
            "relookup" => {
                let start_time = Utc::now();
                mci.create_response(&ctx.serenity_context().http, CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_loading_embed("pinging the server (May take 6 sec)"))
                        .components(vec![])
                )).await?;

                if let Ok(ip) = Ipv4Addr::from_str(server_info.server_ip.as_str()) {
                    if let Ok(ping_result) = ping::execute_ping(ip, server_info.server_port, 0, Duration::from_secs(3)).await {
                        let query = query::execute_query(ip, server_info.server_port, Duration::from_secs(3), true).await.ok();
                        let join = join::execute_join_check(ip, server_info.server_port, Duration::from_secs(3), "ServerRawler", ping_result.protocol_version.or(server_history.version_protocol).unwrap_or(767)).await.ok();
                        let (mut new_info, new_history) = crate::database::parse_server(ip, server_info.server_port, ping_result, query, None);

                        if crate::database::server::insert_servers(
                            &vec![
                                (new_info.clone(), new_history.clone())
                            ]
                        ).await.is_ok() {
                            new_info.discovered = server_info.discovered;
                            new_info.server_id = server_info.server_id;

                            let mut inter_response = EditInteractionResponse::new()
                                .embed(build_server_embed(start_time, &new_info, &new_history))
                                .components(build_manage_server_action_row(false, &new_history));

                            if let Some(base64_icon) = &new_history.icon {
                                if let Ok(decoded_bytes) = BASE64_STANDARD.decode(base64_icon.trim()) {
                                    let attachment = CreateAttachment::bytes(decoded_bytes, "server_icon.png");
                                    inter_response = inter_response.attachments(EditAttachments::new().add(attachment.clone()));
                                    response = response.attachments(EditAttachments::new().add(attachment))
                                }
                            }
                            response = response.embed(build_server_embed(start_time, &server_info, &server_history));

                            mci.edit_response(&ctx.serenity_context().http, inter_response).await?;
                            continue;
                        }
                    }
                }

                mci.edit_response(&ctx.serenity_context().http, EditInteractionResponse::new()
                    .embed(create_error_embed("Failed to ping server", None))
                    .components(build_manage_server_action_row(false, &server_history))
                ).await?;
                return Ok(())
            },
            "show_history" => {
                mci.create_response(&ctx.serenity_context().http, CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_loading_embed("coding this feature"))
                        .components(vec![])
                )).await?;
                // TODO: Give user the complete history of the server
                //  with paginator
            },
            _ => {}
        }
    }

    message.edit(
        &ctx.serenity_context().http,
        response.components(build_manage_server_action_row(true, &server_history))
    ).await?;

    Ok(())
}