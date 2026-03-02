pub mod server_searcher;
pub mod util_commands;
pub mod stats_command;
pub mod actions;
mod player_searcher;

use poise::serenity_prelude as serenity;
use std::env;
use std::process::exit;
use std::time::Duration;
use chrono::{DateTime, Utc};
use colored_text::Colorize;
use serenity::all::{ActionRowComponent, ComponentInteraction, CreateActionRow, CreateEmbed, CreateEmbedFooter, CreateInputText, CreateInteractionResponse, CreateModal, InputTextStyle, ModalInteraction, ModalInteractionData};
use serenity::collector::ModalInteractionCollector;
use crate::{logger, manager};
use crate::config::MainConfig;
use crate::logger::DefaultColor;

#[derive(Debug)]
pub struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn start_bot() {
    let token = match MainConfig::get() {
        Ok(config) => {
            if let Some(ref t) = config.discord.token {
                t.clone()
            } else {
                match env::var("DISCORD_TOKEN") {
                    Ok(t) => t,
                    Err(_) => {
                        logger::critical("Discord token not found in config or environment!".to_string())
                            .prefix("Discord").send().await;
                        return;
                    }
                }
            }
        },
        Err(e) => {
            logger::critical(format!("Config not initialized: {}", e.to_string()))
                .prefix("Discord").send().await;
            exit(0);
        }
    };

    manager::TaskManager::spawn("discord_bot", |token_cancel| async move {
        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![
                    server_searcher::search_server(),
                    player_searcher::search_player(),
                ],
                on_error: |error| Box::pin(async move {
                    match error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            logger::error(format!(
                                "Command '{}' failed: {}",
                                ctx.command().name.hex(DefaultColor::Highlight.hex()),
                                error.hex(DefaultColor::Highlight.hex())
                            )).prefix("Discord").send().await;
                        },
                        poise::FrameworkError::Setup { error, .. } => {
                            logger::error(format!("Setup failed: {}", error.hex(DefaultColor::Highlight.hex())))
                                .prefix("Discord").send().await;
                        },
                        other => {
                            logger::error(format!("Framework error: {}", format!("{:?}", other).hex(DefaultColor::Highlight.hex())))
                                .prefix("Discord").send().await;
                        }
                    }
                }),
                ..Default::default()
            })
            .setup(|ctx, ready, framework| {
                Box::pin(async move {
                    let bot_name = &ready.user.name;
                    let bot_id = ready.user.id;
                    let command_count = framework.options().commands.len();

                    logger::success(format!("Logged in as \"{}\" ({})", bot_name.hex(DefaultColor::Highlight.hex()), bot_id.hex(DefaultColor::Highlight.hex())))
                        .prefix("Discord")
                        .send()
                        .await;

                    logger::info(format!("Loading {} commands...", command_count.hex(DefaultColor::Highlight.hex())))
                        .prefix("Discord")
                        .send()
                        .await;

                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(Data {})
                })
            })
            .build();

        let mut client = serenity::ClientBuilder::new(token, serenity::GatewayIntents::non_privileged())
            .framework(framework)
            .await
            .expect("Failed to create client");

        let shard_manager = client.shard_manager.clone();

        tokio::spawn(async move {
            token_cancel.cancelled().await;

            logger::warning("Graceful shutdown requested. Stopping Shard Manager...".to_string())
                .prefix("Discord").send().await;

            tokio::time::sleep(Duration::from_millis(500)).await;
            shard_manager.shutdown_all().await;
        });

        if let Err(e) = client.start().await {
            logger::error(format!("Client error during execution: {:?}", e))
                .prefix("Discord").send().await;
        }

        logger::success("Bot has been shut down gracefully.".into())
            .prefix("Discord").send().await;
    }).await;
}

// Generic methods
pub fn create_base_embed(start_time: Option<DateTime<Utc>>) -> CreateEmbed {
    let footer_text = match start_time {
        Some(t) => {
            let duration = Utc::now().signed_duration_since(t).num_milliseconds();
            format!("ServerRawler {} • {}ms", crate::get_version_raw(), duration)
        },
        None => format!("ServerRawler {}", crate::get_version_raw()),
    };

    CreateEmbed::new()
        .color(u32::from_str_radix(DefaultColor::Highlight.hex().trim_start_matches('#'), 16).unwrap_or(0xff4500))
        .footer(CreateEmbedFooter::new(footer_text))
        .timestamp(serenity::Timestamp::now())
}

pub fn create_loading_embed(action: &str) -> CreateEmbed {
    CreateEmbed::default()
        .title("⏳ Loading...")
        .description(format!("Please wait while we are **{}**", action))
        .color(0xeec900)
        .footer(CreateEmbedFooter::new(format!(
            "ServerRawler {} • Processing",
            crate::get_version_raw()
        )))
        .timestamp(serenity::Timestamp::now())
}

pub fn create_error_embed(error: &str, start_time: Option<DateTime<Utc>>) -> CreateEmbed {
    create_base_embed(start_time)
        .title("❌ Error")
        .description(&format!("There was an error: {}", error))
        .color(0xff0000)
}

pub fn create_success_embed(message: &str, start_time: Option<DateTime<Utc>>) -> CreateEmbed {
    create_base_embed(start_time)
        .title("✅ Success")
        .description(&format!("Successfully done: {}", message))
        .color(0x00ff00)
}

async fn open_string_input_modal(
    ctx: Context<'_>,
    mci: &ComponentInteraction,
    title: &str,
    label: &str,
    placeholder: &str,
) -> Result<Option<String>, serenity::Error> {
    let modal_id = format!("modal_{}", mci.id);
    let custom_id = "input_value";

    mci.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Modal(
        CreateModal::new(&modal_id, title)
            .components(vec![CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Short, label, custom_id)
                    .placeholder(placeholder)
                    .required(true)
            )])
    )).await?;

    let response = ModalInteractionCollector::new(ctx.serenity_context())
        .filter(move |m| m.data.custom_id == modal_id)
        .timeout(Duration::from_secs(25))
        .await;

    if let Some(m) = response {
        m.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Acknowledge).await?;

        let value = m.data.components.iter()
            .flat_map(|row| row.components.iter())
            .find_map(|component| {
                if let ActionRowComponent::InputText(t) = component {
                    if t.custom_id == custom_id {
                        return t.value.clone();
                    }
                }
                None
            });

        return Ok(value);
    }

    Ok(None)
}