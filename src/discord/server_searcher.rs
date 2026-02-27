use std::net::Ipv4Addr;
use std::time::Duration;
use chrono::{DateTime, Utc};
use colored_text::Colorize;
use futures::StreamExt;
use poise::{CreateReply, ReplyHandle};
use serenity::all::{ButtonStyle, CreateInteractionResponse};
use serenity::builder::{CreateInteractionResponseMessage, EditInteractionResponse};
use serenity::collector::ComponentInteractionCollector;
use crate::discord::{actions, create_base_embed, create_error_embed, create_loading_embed, Context, Error};
use crate::{logger, scanning};
use crate::database::server;
use crate::logger::DefaultColor;
use crate::minecraft::{join, ping, query};

// Commands

/// Search a server in the database
#[poise::command(
    slash_command,
    description_localized("de", "Suche einen Server in der Datenbank")
)]
pub async fn search_server(
    ctx: Context<'_>,
    #[description = "The server IP address"] ip: Option<String>,
    #[description = "The server port"] port: Option<u16>
) -> Result<(), Error> {
    let start_time = Utc::now();

    let loading_embed = create_loading_embed("processing command");
    let reply = ctx.send(CreateReply::default().embed(loading_embed)).await?;

    match (ip, port) {
        // IP was given and maybe port
        (Some(input_ip), input_port) => {
            let (ip, port) = match input_port {
                Some(p) => (input_ip.clone(), p),
                // No port given, so try to parse the IP like this: IP[:PORT]
                None => {
                    let parts: Vec<&str> = input_ip.split(':').collect();

                    if parts.len() > 1 {
                        let p = parts[1].parse::<u16>().unwrap_or(25565);
                        (parts[0].to_string(), p)
                    } else {
                        // No port detected in the IP
                        (input_ip.clone(), 25565)
                    }
                }
            };
            // Try to resolve hostname to IP
            if let Some(resolved_ip) = scanning::resolve_address(&ip, port).await {
                match server::get_server_by_address(resolved_ip.to_string(), port).await {
                    // Found in the database
                    Ok(Some((info, history))) => {
                        actions::server::create_one_server_action(start_time, ctx, reply, info, history).await?;
                    },

                    // Server not found
                    Ok(None) => {
                        server_not_found_action(start_time, reply, ctx, ip, port, resolved_ip).await?;
                    },

                    // Database error
                    Err(e) => {
                        logger::error(
                            format!(
                                "Database error while trying to perform \"{}\" command: {}",
                                "/search_server".hex(DefaultColor::Highlight.hex()),
                                e.hex(DefaultColor::Highlight.hex())
                            )
                        ).prefix("Database").send().await;
                        let error_embed = create_error_embed("Database error", Some(start_time));
                        reply.edit(ctx, CreateReply::default().embed(error_embed)).await?;
                    }
                }
            } else {
                let error_embed = create_error_embed(&format!("Could not use IP: `{}`", input_ip), Some(start_time));
                reply.edit(ctx, CreateReply::default().embed(error_embed)).await?;
            }
        }
        // Only port was given
        (None, Some(_)) => {
            let error_embed = create_error_embed("You need to specify an IP address to use the port.", Some(start_time));
            reply.edit(ctx, CreateReply::default().embed(error_embed)).await?;
        },

        // Nothing was given
        (None, None) => {
            // TODO: Good search mechanismus
            reply.edit(ctx, CreateReply::default().content("In work!")).await?;
        }
    }

    Ok(())
}

// Action rows

async fn server_not_found_action(
    start_time: DateTime<Utc>,
    reply: ReplyHandle<'_>,
    ctx: Context<'_>,
    ip: String,
    port: u16,
    resolved_ip: Ipv4Addr
) -> Result<(), Error> {
    let make_action_row = |disabled| vec![serenity::builder::CreateActionRow::Buttons(vec![
        serenity::builder::CreateButton::new("lookup_server")
            .label("Lookup")
            .style(ButtonStyle::Success)
            .emoji('🔍')
            .disabled(disabled),
    ])];

    let ip_formatted = if resolved_ip.to_string() == ip { format!("`{}`", ip) } else { format!("`{}` (`{}`)", ip, resolved_ip.to_string()) };

    let success_embed = create_base_embed(Some(start_time))
        .title("Server not found")
        .description(
            &format!(
                "The endpoint {}, port `{}` is not indexed in the database yet.\n\nClick on the **Lookup** button to scan this address!",
                ip_formatted,
                port
            )
        )
        .color(0xFFD700);

    reply.edit(
        ctx, CreateReply::default()
            .embed(success_embed.clone())
            .components(make_action_row(false))
    ).await?;

    let mut collector = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .message_id(reply.clone().message().await?.id)
        .timeout(Duration::from_secs(120))
        .stream();

    while let Some(mci) = collector.next().await {
        match mci.data.custom_id.as_str() {
            "lookup_server" => {
                let start_time = Utc::now();

                mci.create_response(&ctx.serenity_context().http, CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_loading_embed("pinging the server (May take 6 sec)"))
                        .components(vec![])
                )).await?;

                match ping::execute_ping(resolved_ip, port, 0, Duration::from_secs(3)).await {
                    Ok(ping_result) => {
                        logger::info(format!("Ping successful for {}:{}", resolved_ip, port)).send().await;

                        let query = match query::execute_query(resolved_ip, port, Duration::from_secs(3), true).await {
                            Ok(q) => Some(q),
                            Err(e) => {
                                logger::error(format!("Query failed for {}:{}: {}", resolved_ip, port, e)).send().await;
                                None
                            }
                        };

                        let protocol = ping_result.protocol_version.unwrap_or(767);
                        let join = match join::execute_join_check(resolved_ip, port, Duration::from_secs(3), "ServerRawler", protocol).await {
                            Ok(j) => Some(j),
                            Err(e) => {
                                logger::error(format!("Join check failed for {}:{}: {}", resolved_ip, port, e)).send().await;
                                None
                            }
                        };

                        let (new_info, new_history) = crate::database::parse_server(resolved_ip, port, ping_result, query, None);

                        match server::insert_servers(&vec![(new_info, new_history)]).await {
                            Ok(_) => {
                                match server::get_server_by_address(resolved_ip.to_string(), port).await {
                                    Ok(Some((info, history))) => {
                                        if let Err(e) = actions::server::create_one_server_action(start_time, ctx, reply.clone(), info, history).await {
                                            logger::error(format!("Failed to create server action UI: {}", e)).send().await;
                                        }
                                        return Ok(());
                                    }
                                    Ok(None) => {
                                        logger::error(format!("Server inserted but not found in DB immediately after: {}:{}", resolved_ip, port)).send().await;
                                    }
                                    Err(e) => {
                                        logger::error(format!("Database error while fetching new server {}:{}: {}", resolved_ip, port, e)).send().await;
                                    }
                                }
                            }
                            Err(e) => {
                                logger::error(format!("Failed to insert server into database {}:{}: {}", resolved_ip, port, e)).send().await;
                            }
                        }
                    }

                    Err(e) => {
                        logger::error(format!("Initial Ping failed for {}:{}: {}", resolved_ip, port, e)).send().await;
                    }
                }

                mci.edit_response(&ctx.serenity_context().http, EditInteractionResponse::new()
                    .embed(create_error_embed("Lookup failed. Is the server online?", None))
                    .components(make_action_row(false))
                ).await?;
            },
            _ => {}
        }
    }

    // Disable buttons
    reply.edit(
        ctx, CreateReply::default()
            .embed(success_embed)
            .components(make_action_row(true))
    ).await?;

    Ok(())
}