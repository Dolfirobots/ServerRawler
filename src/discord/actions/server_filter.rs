use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use poise::{CreateReply, ReplyHandle};
use serenity::all::{ButtonStyle, ComponentInteraction, ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse, EditMessage, Message};
use serenity::all::CreateInteractionResponse::UpdateMessage;
use crate::database::server::search_servers;
use crate::discord::{create_base_embed, create_error_embed, create_loading_embed, create_success_embed, Context};
use crate::discord::actions::paginator::create_paged_server_view;

#[derive(Debug, Clone, PartialEq)]
pub enum StringFilter {
    Contains(String),
    Equals(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberFilter {
    Less(i32),
    Greater(i32),
    Equal(i32),
    Range(i32, i32),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchFilters {
    pub description: Option<StringFilter>,
    pub version_name: Option<StringFilter>,
    pub software_name: Option<StringFilter>,
    pub kick_message: Option<StringFilter>,

    pub players_online: Option<NumberFilter>,
    pub players_max: Option<NumberFilter>,
    pub version_protocol: Option<i32>,

    pub enforces_secure_chat: Option<bool>,
    pub is_modded: Option<bool>,
    pub cracked: Option<bool>,
    pub whitelist: Option<bool>,

    pub plugin_name: Option<String>,
    pub mod_id: Option<String>,
}

impl SearchFilters {
    fn format_string_filter(filter: &Option<StringFilter>) -> String {
        match filter {
            Some(StringFilter::Contains(s)) => format!("*CONTAINS* `{}`", s),
            Some(StringFilter::Equals(s)) => format!("*EQUALS* `{}`", s),
            None => "Any".to_string(),
        }
    }

    fn format_number_filter(filter: &Option<NumberFilter>) -> String {
        match filter {
            Some(NumberFilter::Less(n)) => format!("< `{}`", n),
            Some(NumberFilter::Greater(n)) => format!("> `{}`", n),
            Some(NumberFilter::Equal(n)) => format!("= `{}`", n),
            Some(NumberFilter::Range(a, b)) => format!("`{}` - `{}`", a, b),
            None => "Any".to_string(),
        }
    }

    fn format_bool_filter(filter: Option<bool>) -> String {
        match filter {
            Some(true) => "✅ Yes".to_string(),
            Some(false) => "❌ No".to_string(),
            None => "Any".to_string(),
        }
    }

    pub fn build_homepage(&self, start_time: DateTime<Utc>) -> CreateEmbed {
        let mut description: String = "Configure your search filters below\nClick the categories in the menu to customize your filter.".into();
        let mut embed = create_base_embed(Some(start_time))
            .title("🔎 Advanced Server Search")
            .color(0x3498db);

        let general_info = format!(
            "📝 **Description:** {}\n📶 **Version:** {}\n📖 **Software:** {}",
            Self::format_string_filter(&self.description),
            Self::format_string_filter(&self.version_name),
            Self::format_string_filter(&self.software_name)
        );
        embed = embed.field("General", general_info, false);

        let player_info = format!(
            "👥 **Online:** {}\n👥 **Max:** {}",
            Self::format_number_filter(&self.players_online),
            Self::format_number_filter(&self.players_max),
        );
        embed = embed.field("Players", player_info, true);

        let technical_info = format!(
            "➕ **Modded:** {}\n🔓 **Cracked:** {}\n🛡️ **Whitelist:** {}",
            Self::format_bool_filter(self.is_modded),
            Self::format_bool_filter(self.cracked),
            Self::format_bool_filter(self.whitelist)
        );
        embed = embed.field("Settings", technical_info, true);

        if self.description.is_none() && self.players_online.is_none() && self.is_modded.is_none() {
            description.push_str("\n\n-# Tip: Click on the dropdown below to configure any filter!");
        }

        embed.description(description)
    }

    pub fn is_empty(&self) -> bool {
        self == &SearchFilters::default()
    }
}

pub async fn open_filter_ui(ctx: Context<'_>, reply: ReplyHandle<'_>) -> Result<(), serenity::Error> {
    let mut filters = SearchFilters::default();

    let make_home_action_row = |disabled, filter_is_none|
        vec![
            CreateActionRow::SelectMenu(CreateSelectMenu::new("server_filter", CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Description", "edit_description").emoji('📝'),

                    CreateSelectMenuOption::new("Max Players", "edit_max_players").emoji('👥'),
                    CreateSelectMenuOption::new("Online Players", "edit_online_players").emoji('👥'),
                    CreateSelectMenuOption::new("Player Sample", "edit_player_sample").emoji('👥'),

                    CreateSelectMenuOption::new("Version", "edit_version").emoji('📶'),
                    CreateSelectMenuOption::new("Enforce Secure Chat", "edit_enforce_secure_chat").emoji('💬'),

                    CreateSelectMenuOption::new("Modded Server", "edit_modding").emoji('🧩'),

                    CreateSelectMenuOption::new("Plugins", "edit_plugins").emoji('🧩'),
                    CreateSelectMenuOption::new("Software", "edit_software").emoji('📖'),

                    CreateSelectMenuOption::new("Kick Message", "edit_kick_message").emoji('📝'),
                    CreateSelectMenuOption::new("Cracked", "edit_cracked").emoji('🔓'),
                    CreateSelectMenuOption::new("Whitelist", "edit_whitelist").emoji('🔓')
                ]
            }).disabled(disabled)),
            CreateActionRow::Buttons(vec![
                CreateButton::new("search")
                    .label("Search")
                    .style(ButtonStyle::Success)
                    .emoji('🔍')
                    .disabled(disabled || filter_is_none),
                CreateButton::new("reset")
                    .label("Reset")
                    .style(ButtonStyle::Danger)
                    .disabled(disabled || filter_is_none)
            ])
        ];

    loop {
        let start_time = Utc::now();

        reply.edit(ctx, CreateReply::default()
            .embed(filters.build_homepage(start_time))
            .components(make_home_action_row(false, filters.is_empty()))
        ).await?;

        let collector = ComponentInteractionCollector::new(ctx.serenity_context())
            .author_id(ctx.author().id)
            .message_id(reply.message().await?.id)
            .timeout(Duration::from_secs(120))
            .next()
            .await;

        let mci = match collector {
            Some(i) => i,
            None => break,
        };

        match mci.data.custom_id.as_str() {
            "reset" => {
                filters = SearchFilters::default();
                mci.create_response(
                    &ctx.serenity_context().http, UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(create_success_embed("Reset all filter rules!", Some(start_time)))
                            .components(vec![])
                    )
                ).await?;

                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            "search" => {
                mci.create_response(&ctx.serenity_context().http, UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_loading_embed("searching..."))
                        .components(vec![])
                )).await?;

                match search_servers(filters.clone(), 200).await {
                    // Found server
                    Ok(Some(servers)) => {
                        mci.edit_response(
                            &ctx.serenity_context().http,
                            EditInteractionResponse::new()
                                .embed(create_success_embed(&format!("Found {} servers", servers.len()), Some(start_time)))
                        ).await?;

                        create_paged_server_view(ctx, reply, &servers).await?;
                    },
                    // No server found
                    Ok(None) => {
                        let dur = format!("<t:{}:R>", (Utc::now() + chrono::Duration::seconds(5)).timestamp());
                        let embed = create_base_embed(None)
                            .title("🔎 Search Results")
                            .description(
                                format!("No servers found matching your filters.\n\n-# Returning {} to the filter home menu", dur)
                            )
                            .color(0xff0000);

                        mci.edit_response(
                            &ctx.serenity_context().http,
                            EditInteractionResponse::new()
                                .embed(embed)
                        ).await?;

                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    },
                    // Database error
                    Err(err) => {
                        mci.edit_response(
                            &ctx.serenity_context().http,
                            EditInteractionResponse::new().embed(create_error_embed("Database error", Some(start_time)))
                        ).await?;
                    }
                };

                return Ok(());
            }
            "server_filter" => {
                if let ComponentInteractionDataKind::StringSelect { values } = &mci.data.kind {
                    mci.create_response(&ctx.serenity_context().http, UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(create_loading_embed("processing the input..."))
                            .components(vec![])
                    )).await?;

                    let selected = &values[0];
                    let message = &mut reply.message().await?.into_owned();
                    handle_filter_selection(ctx, message, selected, &mut filters).await?;
                }
            }
            _ => {
                mci.defer(&ctx.serenity_context().http).await?;
            }
        }
    }

    reply.edit(ctx, CreateReply::default()
        .embed(filters.build_homepage(Utc::now()))
        .components(make_home_action_row(true, true))
    ).await?;
    Ok(())
}

pub async fn handle_filter_selection(
    ctx: Context<'_>,
    msg: &mut Message,
    selected: &str,
    filters: &mut SearchFilters
) -> Result<(), serenity::Error> {
    match selected {
        "edit_description" => {
            let current = &filters.description;
            let get_style = |matches: bool| if matches { ButtonStyle::Success } else { ButtonStyle::Primary };

            let embed = create_base_embed(None)
                .title("📝 Filter: Description")
                .color(0x5865F2)
                .description(format!(
                    "**Current Filter:** {}\n\nWith what pattern do you want to search a server by description?\n-# The color codes are removed in the search process",
                    SearchFilters::format_string_filter(current)
                ))
                .field("Contains", "Search for a keyword anywhere in the Description.", true)
                .field("Exact Match", "The Description must be exactly what you type.", true);

            let components = |disabled| vec![
                CreateActionRow::Buttons(vec![
                    CreateButton::new("filter_back_to_main")
                        .label("Back")
                        .emoji('⬅')
                        .style(ButtonStyle::Secondary)
                        .disabled(disabled),
                    CreateButton::new("filter_set_description_contains")
                        .label("Contains")
                        .emoji('🔎')
                        .style(get_style(matches!(current, Some(StringFilter::Contains(_)))))
                        .disabled(disabled),
                    CreateButton::new("filter_set_description_equals")
                        .label("Exact Match")
                        .emoji('🎯')
                        .style(get_style(matches!(current, Some(StringFilter::Equals(_)))))
                        .disabled(disabled),
                    CreateButton::new("filter_clear")
                        .label("Clear")
                        .emoji('🗑')
                        .style(ButtonStyle::Danger)
                        .disabled(disabled || current.is_none()),
                ])
            ];

            msg.edit(ctx, EditMessage::default().embed(embed).components(components(false))).await?;

            let mut collector = ComponentInteractionCollector::new(ctx.serenity_context())
                .author_id(ctx.author().id)
                .message_id(msg.id)
                .timeout(Duration::from_secs(30))
                .stream();

            while let Some(mci) = collector.next().await {
                match mci.data.custom_id.as_str() {
                    "filter_back_to_main" => {
                        mci.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Acknowledge).await?;
                        return Ok(())
                    },
                    "filter_set_description_contains" | "filter_set_description_equals" => {
                        let is_contains = mci.data.custom_id == "filter_set_description_contains";
                        let dur = format!("<t:{}:R>", (Utc::now() + chrono::Duration::seconds(25)).timestamp());

                        msg.edit(
                            &ctx.serenity_context().http,
                            EditMessage::new()
                                .embed(create_loading_embed(
                                    &format!(
                                        "waiting for your input (UI expires {})",
                                        dur
                                    )
                                ))
                                .components(components(true))
                        ).await?;

                        if let Ok(Some(input)) = crate::discord::open_string_input_modal(
                            ctx,
                            &mci,
                            "Filter: Description",
                            "Enter search string (This will expire in 25s)",
                            "e.g. Survival, Semi-Vanilla..."
                        ).await {
                            if is_contains {
                                filters.description = Some(StringFilter::Contains(input));
                            } else {
                                filters.description = Some(StringFilter::Equals(input));
                            }
                            return Ok(());
                        }
                    },
                    "filter_clear" => {
                        let start_time = Utc::now();
                        filters.description = None;

                        mci.create_response(
                            &ctx.serenity_context().http, UpdateMessage(
                                CreateInteractionResponseMessage::new().embed(create_success_embed("Reset description filter rule", Some(start_time)))
                            )
                        ).await?;

                        tokio::time::sleep(Duration::from_secs(1)).await;
                        return Ok(())
                    },
                    _ => {}
                }
            }
        },

        "edit_max_players" => {

        },
        "edit_online_players" => {

        },
        "edit_player_sample" => {

        },

        "edit_version" => {},
        "edit_enforce_secure_chat" => {},

        "edit_modding" => {},

        "edit_plugins" => {},
        "edit_software" => {},

        "edit_kick_message" => {},
        "edit_cracked" => {},
        "edit_whitelist" => {},
        _ => {}
    }

    Ok(())
}