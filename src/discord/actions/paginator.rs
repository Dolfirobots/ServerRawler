use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Duration;
use chrono::Utc;
use serenity::all::{ButtonStyle, ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, EditAttachments, EditInteractionResponse, EditMessage, Message, RoleId, UserId};
use crate::config;
use crate::database::{PlayerHistory, ServerHistory, ServerInfo};
use crate::database::server::get_server_by_id;
use crate::discord::actions::server::{build_manage_server_action_row, build_server_embed, convert_img_for_discord, view_mods, view_players, view_plugins, view_sample};
use crate::discord::{actions, create_base_embed, create_error_embed, create_loading_embed, Context, Error};
use crate::minecraft::{join, ping, query};

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

pub async fn base_paginator(
    ctx: serenity::all::Context,
    author_id: UserId,
    mut message: Message,
    pages: Vec<CreateEmbed>,
) -> Result<(), Error> {
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

        message.edit(&ctx, EditMessage::default()
            .embed(current_embed.clone())
            .components(create_action_row(current_page, total_pages, false))
        ).await?;

        let collector = ComponentInteractionCollector::new(&ctx)
            .author_id(author_id)
            .message_id(message.id)
            .timeout(Duration::from_secs(35))
            .next()
            .await;

        let interaction = match collector {
            Some(i) => i,
            None => break,
        };

        match interaction.data.custom_id.as_str() {
            "first" => current_page = 0,
            "prev" => if current_page > 0 { current_page -= 1 },
            "next" => if current_page < total_pages - 1 { current_page += 1 },
            "last" => current_page = total_pages - 1,
            _ => {
                interaction.defer(&ctx).await?;
                continue;
            }
        }

        interaction.create_response(
            &ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(get_page_embed(current_page, &pages))
                    .components(create_action_row(current_page, total_pages, false))
            )
        ).await?;
    }

    let final_embed = get_page_embed(current_page, &pages);

    message.edit(
        ctx, EditMessage::default()
            .embed(final_embed)
            .components(create_action_row(current_page, total_pages, true))
    ).await?;

    Ok(())
}

pub async fn player_paged_view(
    ctx: Context<'_>,
    author_id: UserId,
    mut message: Message,
    player_history: Vec<PlayerHistory>,
) -> Result<(), Error> {
    let total_pages = player_history.len();
    if total_pages == 0 { return Ok(()); }

    let embeds: Vec<CreateEmbed> = player_history.iter().map(|history| {
        create_base_embed(None)
            .title(format!("History Entry: {}", history.username))
            .field("Server ID", format!("`{}`", history.server_id), true)
            .field("Seen at", format!("<t:{}:F>", history.seen), true)
            .thumbnail(format!("https://mc-heads.net/avatar/{}", history.uuid))
    }).collect();

    let make_components = |disabled: bool, current: usize, total: usize| {
        let mut rows = vec![
            CreateActionRow::Buttons(vec![
                CreateButton::new("view_server")
                    .label("View Server")
                    .emoji('⏺')
                    .style(ButtonStyle::Primary)
                    .disabled(disabled)
            ])
        ];
        rows.extend(create_action_row(current, total, disabled));
        rows
    };

    let mut current_page = 0;

    let get_page_embed = |idx: usize, base_embeds: &[CreateEmbed]| {
        base_embeds[idx].clone().footer(CreateEmbedFooter::new(format!(
            "ServerRawler {} • Page {}/{}",
            crate::get_version_raw(),
            idx + 1,
            total_pages
        )))
    };

    loop {
        let current_embed = get_page_embed(current_page, &embeds);

        message.edit(&ctx.http(), EditMessage::default()
            .embed(current_embed)
            .components(make_components(false, current_page, total_pages))
        ).await?;

        let collector = ComponentInteractionCollector::new(ctx)
            .author_id(author_id)
            .message_id(message.id)
            .timeout(Duration::from_secs(35))
            .await;

        let interaction = match collector {
            Some(i) => i,
            None => break,
        };

        match interaction.data.custom_id.as_str() {
            "first" => current_page = 0,
            "prev" => if current_page > 0 { current_page -= 1 },
            "next" => if current_page < total_pages - 1 { current_page += 1 },
            "last" => current_page = total_pages - 1,
            "view_server" => {
                let start_time = Utc::now();
                let server_id = player_history[current_page].server_id;

                interaction.create_response(&ctx.http(), CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(create_loading_embed("fetching server from database"))
                )).await?;


                match get_server_by_id(server_id).await {
                    Ok(Some((info, history))) => {
                        actions::server::create_one_server_action(
                            start_time,
                            ctx,
                            &mut message,
                            info,
                            history
                        ).await?;
                    },
                    Ok(None) => {
                        message.edit(&ctx.http(), EditMessage::default()
                            .embed(create_error_embed("Server not found in database.", None))
                            .components(vec![CreateActionRow::Buttons(vec![
                                CreateButton::new("back_to_history").label("Back").style(ButtonStyle::Danger)
                            ])])
                        ).await?;

                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    },
                    Err(e) => {
                        message.edit(&ctx.http(), EditMessage::default()
                            .embed(create_error_embed("Database error", None))
                        ).await?;

                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                }
            }
            _ => continue,
        }

        interaction.create_response(
            &ctx.http(),
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(get_page_embed(current_page, &embeds))
                    .components(make_components(false, current_page, total_pages))
            )
        ).await?;
    }

    let final_embed = get_page_embed(current_page, &embeds);
    let _ = message.edit(
        &ctx.http(),
        EditMessage::default()
            .embed(final_embed)
            .components(make_components(true, current_page, total_pages))
    ).await;

    Ok(())
}

pub async fn create_paged_server_view(
    ctx: Context<'_>,
    mut message: Message,
    mut servers: Vec<(ServerInfo, ServerHistory)>,
) -> Result<(), Error> {
    let mut current_page = 0;
    let total_pages = servers.len();

    let get_page_embed = |idx: usize, info: &ServerInfo, history: &ServerHistory| {
        build_server_embed(Utc::now(), info, history)
            .footer(CreateEmbedFooter::new(format!(
                "ServerRawler {} • Page {}/{}",
                crate::get_version_raw(),
                idx + 1,
                total_pages
            )))
    };

    let (info, history) = &servers[current_page];
    let current_embed = get_page_embed(current_page, info, history);

    let mut components = build_manage_server_action_row(false, history);
    components.extend(create_action_row(current_page, total_pages, false));

    let mut response = EditMessage::default()
        .embed(current_embed.clone())
        .components(components)
        .attachments(EditAttachments::new());

    if let Some(attachment) = convert_img_for_discord(history) {
        response = response.attachments(EditAttachments::new().add(attachment));
    }

    message.edit(ctx, response).await?;

    loop {
        // This is waiting of an interaction, or if the timeout runs out
        let interaction = ComponentInteractionCollector::new(ctx.serenity_context())
            .author_id(ctx.author().id)
            .message_id(message.id)
            .timeout(Duration::from_secs(60))
            .next()
            .await;

        let interaction = match interaction {
            Some(i) => i,
            None => break, // Timeout
        };

        let (info, history) = &servers[current_page];

        match interaction.data.custom_id.as_str() {
            "first" => {
                current_page = 0;
                
                interaction.create_response(
                    &ctx.serenity_context().http,
                    CreateInteractionResponse::Acknowledge
                ).await?;
            },
            "prev" => {
                if current_page > 0 { current_page -= 1 }

                interaction.create_response(
                    &ctx.serenity_context().http,
                    CreateInteractionResponse::Acknowledge
                ).await?;
            },
            "next" => {
                if current_page < total_pages - 1 { current_page += 1 }

                interaction.create_response(
                    &ctx.serenity_context().http,
                    CreateInteractionResponse::Acknowledge
                ).await?;
            },
            "last" => {
                current_page = total_pages - 1;

                interaction.create_response(
                    &ctx.serenity_context().http,
                    CreateInteractionResponse::Acknowledge
                ).await?;
            },

            "view_plugins" => {
                view_plugins(
                    ctx,
                    &interaction,
                    &info,
                    &history
                ).await?;

                continue
            }

            "view_mods" => {
                view_mods(
                    ctx,
                    &interaction,
                    &info,
                    &history
                ).await?;

                continue
            }
            
            "view_sample" => {
                view_sample(
                    ctx,
                    &interaction,
                    &info,
                    &history
                ).await?;

                continue
            }

            "view_players" => {
                view_players(
                    ctx,
                    &interaction,
                    &info,
                    &history
                ).await?;

                continue
            }

            "relookup" => {
                interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_loading_embed("Pinging the server (may take 6s)..."))
                        .components(vec![])
                )).await?;

                if let Ok(ip) = Ipv4Addr::from_str(info.server_ip.as_str()) {
                    if let Ok(ping_result) = ping::execute_ping(ip, info.server_port, 0, Duration::from_secs(3)).await {
                        let query = query::execute_query(ip, info.server_port, Duration::from_secs(3), true).await.ok();
                        let protocol = ping_result.protocol_version.or(history.version_protocol).unwrap_or(767);
                        let join = join::execute_join_check(ip, info.server_port, Duration::from_secs(3), "ServerRawler", protocol).await.ok();

                        let (mut new_info, new_history) = crate::database::parse_server(ip, info.server_port, ping_result, query, join);

                        if crate::database::server::insert_servers(&vec![(new_info.clone(), new_history.clone())]).await.is_ok() {
                            new_info.discovered = info.discovered;
                            new_info.server_id = info.server_id;

                            servers[current_page] = (new_info, new_history);
                        }
                    }
                } else {
                    interaction.edit_response(
                        &ctx.serenity_context().http,
                        EditInteractionResponse::new()
                            .embed(create_error_embed("Failed to ping server", None))
                            .components(vec![])
                    ).await?;

                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue
                }

            }

            "history" => {
                interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(create_loading_embed("coding this feature"))
                        .components(vec![])
                )).await?;
                // TODO: Give user the complete history of the server
                //  with paginator
            }

            "view_join" => {
                let start_time = Utc::now();
                let cfg = config::MainConfig::get().ok();
                let required_role_id = cfg.and_then(|c| c.discord.join_verify_role).map(RoleId::new);

                let has_permission = if let Some(role_id) = required_role_id {
                    interaction.member.as_ref().map_or(false, |member| {
                        member.roles.contains(&role_id)
                    })
                } else {
                    false
                };

                if !has_permission {
                    interaction.create_response(
                        &ctx.http(),
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(
                                    create_error_embed("", Some(start_time))
                                        .description("**No Permission:** You need to be verified to see the join information's")
                                )
                                .ephemeral(true)
                        )
                    ).await?;
                    return Ok(());
                }

                let mut response = create_base_embed(Some(start_time))
                    .title("Join Information's")
                    .description(
                        format!(
                            "-# {}:{}\n- **Cracked:** {}\n- **Whitelist:** {}",
                            info.server_ip,
                            info.server_port,
                            history.cracked.map(|b| if b { "Yes" } else { "No" }).unwrap_or("Unknown"),
                            history.whitelist.map(|b| if b { "Enabled" } else { "Disabled" }).unwrap_or("Unknown")
                        )
                    );

                if let Some(kick_msg) = &history.kick_message {
                    response = response.field("Kick Message", format!("```{}```", kick_msg), true);
                }

                interaction.create_response(
                    &ctx.http(),
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(response)
                            .ephemeral(true)
                    )
                ).await?;
            }

            _ => continue
        }

        let (info, history) = &servers[current_page];
        let current_embed = get_page_embed(current_page, info, history);

        let mut components = build_manage_server_action_row(false, history);
        components.extend(create_action_row(current_page, total_pages, false));

        let mut response = EditInteractionResponse::new()
            .embed(current_embed)
            .components(components);

        if let Some(attachment) = convert_img_for_discord(history) {
            response = response.attachments(
                EditAttachments::new().add(attachment)
            );
        } else {
            response = response.clear_attachments();
        }

        interaction.edit_response(
            &ctx.serenity_context(),
            response
        ).await?;
    }

    let (info, history) = &servers[current_page];
    let current_embed = get_page_embed(current_page, info, history);

    let mut components = build_manage_server_action_row(true, history);
    components.extend(create_action_row(current_page, total_pages, true));

    let _ = message.edit(
        ctx, EditMessage::default()
            .embed(current_embed.clone())
            .components(components)
    ).await;

    Ok(())
}