use std::time::Duration;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use poise::{CreateReply, ReplyHandle};
use serenity::all::{ButtonStyle, ComponentInteractionCollector, ComponentInteractionDataKind, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, EditInteractionResponse, EditMessage, Message};
use serenity::all::CreateInteractionResponse::UpdateMessage;
use crate::database::server::search_servers;
use crate::discord::{create_base_embed, create_error_embed, create_loading_embed, create_success_embed, open_string_input_modal, Context, Error};
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

        if self.is_empty() {
            description.push_str("\n\n-# Tip: Click on the dropdown below to configure any filter!");
        }

        embed.description(description)
    }

    pub fn is_empty(&self) -> bool {
        self == &SearchFilters::default()
    }
}

pub async fn open_filter_ui(ctx: Context<'_>, reply: ReplyHandle<'_>) -> Result<(), Error> {
    let mut filters = SearchFilters::default();

    let make_home_action_row = |disabled, filter_is_none|
        vec![
            CreateActionRow::SelectMenu(CreateSelectMenu::new("server_filter", CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Description", "edit_description").emoji('📝'),

                    CreateSelectMenuOption::new("Max Players", "edit_max_players").emoji('👥'),
                    CreateSelectMenuOption::new("Online Players", "edit_online_players").emoji('👥'),
                    //CreateSelectMenuOption::new("Player Sample", "edit_player_sample").emoji('👥'),

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
                    .emoji('🗑')
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
            .timeout(Duration::from_secs(60))
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

                        create_paged_server_view(ctx, reply.message().await?.into_owned(), servers).await?;
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

// Edit filter handlers

fn get_default_filter_buttons(disabled: bool, is_filter_none: bool) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new("filter_back_to_home")
            .label("Back")
            .emoji('⬅')
            .style(ButtonStyle::Secondary)
            .disabled(disabled),
        CreateButton::new("filter_unset")
            .label("Unset")
            .emoji('🗑')
            .style(ButtonStyle::Danger)
            .disabled(disabled || is_filter_none)
    ])
}

pub async fn string_edit_filter(
    name: &str,
    description: &str,
    placeholder: &str,

    filter: &mut Option<StringFilter>,

    msg: &mut Message,
    ctx: Context<'_>
) -> Result<(), Error> {
    let get_style = |matches: bool| if matches { ButtonStyle::Success } else { ButtonStyle::Primary };

    let make_components = |disabled| vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new("filter_string_set_contains")
                .label("Contains")
                .style(get_style(matches!(filter, Some(StringFilter::Contains(_)))))
                .disabled(disabled),
            CreateButton::new("filter_string_set_equals")
                .label("Exact Match")
                .style(get_style(matches!(filter, Some(StringFilter::Equals(_)))))
                .disabled(disabled),
        ]),
        get_default_filter_buttons(disabled, filter.is_none())
    ];

    let embed = create_base_embed(None)
        .title(format!("Filter: {}", name))
        .description(
            format!(
                "**Current:** {}\n\n{}",
                SearchFilters::format_string_filter(filter),
                description
            )
        )
        .field("Contains", "Search for a keyword anywhere.", true)
        .field("Exact Match", "Must be exactly what you type.", true)
        .color(0x5865F2);

    msg.edit(
        &ctx.serenity_context().http,
        EditMessage::new()
            .embed(embed)
            .components(make_components(false))
    ).await?;

    let collector = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .message_id(msg.id)
        .timeout(Duration::from_secs(30))
        .next()
        .await;

    let interaction = match collector {
        None => { return Ok(()) }
        Some(i) => { i }
    };
    let start_time = Utc::now();

    match interaction.data.custom_id.as_str() {
        "filter_back_to_home" => {
            interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Acknowledge).await?;
            return Ok(())
        }
        "filter_string_set_contains" | "filter_string_set_equals" => {
            let is_contains = interaction.data.custom_id == "filter_string_set_contains";
            msg.edit(
                &ctx.serenity_context().http,
                EditMessage::new()
                    .embed(create_loading_embed(
                        &format!(
                            "waiting for your input (UI expires {})",
                            format!(
                                "<t:{}:R>",
                                (Utc::now() + chrono::Duration::seconds(25)).timestamp()
                            )
                        )
                    ))
                    .components(make_components(true))
            ).await?;

            if let Ok(Some(input)) = open_string_input_modal(
                ctx,
                &interaction,
                &format!("Filter: {}", name),
                "Enter a string (This UI will expire in 25s)",
                placeholder
            ).await {
                if is_contains {
                    *filter = Some(StringFilter::Contains(input));
                } else {
                    *filter = Some(StringFilter::Equals(input));
                }
                return Ok(());
            }
        },
        "filter_unset" => {
            *filter = None;

            interaction.create_response(
                &ctx.serenity_context().http,
                UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_success_embed(&format!("Unset {} filter", name), None))
                        .components(vec![])
                )
            ).await?;

            tokio::time::sleep(Duration::from_secs(1)).await;
            return Ok(())
        }
        _ => {}
    }
    Ok(())
}

pub async fn integer_edit_filter(
    name: &str,
    description: &str,
    placeholder: &str,
    filter: &mut Option<NumberFilter>,
    msg: &mut Message,
    ctx: Context<'_>
) -> Result<(), Error> {
    let get_style = |matches: bool| if matches { ButtonStyle::Success } else { ButtonStyle::Primary };

    let make_components = |disabled| vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new("filter_set_integer_greater")
                .label("Greater than")
                .emoji('📈')
                .style(get_style(matches!(filter, Some(NumberFilter::Greater(_)))))
                .disabled(disabled),
            CreateButton::new("filter_set_integer_less")
                .label("Less than")
                .emoji('📉')
                .style(get_style(matches!(filter, Some(NumberFilter::Less(_)))))
                .disabled(disabled),
            CreateButton::new("filter_set_integer_equal")
                .label("Exact Match")
                .emoji('🎯')
                .style(get_style(matches!(filter, Some(NumberFilter::Equal(_)))))
                .disabled(disabled),
            CreateButton::new("filter_set_integer_range")
                .label("Range")
                .emoji('↔')
                .style(get_style(matches!(filter, Some(NumberFilter::Range(_, _)))))
                .disabled(disabled),
        ]),
        get_default_filter_buttons(disabled, filter.is_none())
    ];

    let embed = create_base_embed(None)
        .title(format!("Filter: {}", name))
        .description(format!(
            "**Current:** {}\n\n{}",
            SearchFilters::format_number_filter(filter),
            description
        ))
        .field("Modes", "- **Greater/Less**: Servers with more or fewer players.\n- **Exact**: Match the number precisely.\n- **Range**: Between two numbers (e.g. `10-50`)", false)
        .color(0x5865F2);

    msg.edit(
        &ctx.serenity_context().http,
        EditMessage::new().embed(embed).components(make_components(false))
    ).await?;

    let collector = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .message_id(msg.id)
        .timeout(Duration::from_secs(30))
        .next()
        .await;

    let interaction = match collector {
        None => return Ok(()),
        Some(i) => i,
    };

    match interaction.data.custom_id.as_str() {
        "filter_back_to_home" => {
            interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Acknowledge).await?;
        }
        "filter_set_integer_greater" | "filter_set_integer_less" | "filter_set_integer_equal" | "filter_set_integer_range" => {
            msg.edit(
                &ctx.serenity_context().http,
                EditMessage::new()
                    .embed(create_loading_embed(
                        &format!(
                            "waiting for your integer input (UI expires {})",
                            format!(
                                "<t:{}:R>",
                                (Utc::now() + chrono::Duration::seconds(25)).timestamp()
                            )
                        )
                    ))
                    .components(make_components(true))
            ).await?;

            let mode = interaction.data.custom_id.clone();
            let is_range = mode == "filter_set_integer_range";

            let modal_placeholder = if is_range { "e.g. 10-50" } else { placeholder };
            let modal_desc = if is_range { "Enter range (min-max):" } else { "Enter a number:" };

            if let Ok(Some(input)) = open_string_input_modal(
                ctx,
                &interaction,
                &format!("Filter: {}", name),
                modal_desc,
                modal_placeholder
            ).await {
                let input = input.trim();

                if is_range {
                    let parts: Vec<&str> = input.split(|c| c == '-' || c == ' ').filter(|s| !s.is_empty()).collect();
                    if parts.len() == 2 {
                        if let (Ok(min), Ok(max)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                            *filter = Some(NumberFilter::Range(min, max));
                        }
                    }
                } else if let Ok(val) = input.parse::<i32>() {
                    *filter = match mode.as_str() {
                        "filter_set_integer_greater" => Some(NumberFilter::Greater(val)),
                        "filter_set_integer_less" => Some(NumberFilter::Less(val)),
                        _ => Some(NumberFilter::Equal(val)),
                    };
                }
            }
        },
        "filter_unset" => {
            *filter = None;

            interaction.create_response(
                &ctx.serenity_context().http,
                UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_success_embed(&format!("Unset {} filter", name), None))
                        .components(vec![])
                )
            ).await?;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        _ => {}
    }
    Ok(())
}

pub async fn boolean_edit_filter(
    name: &str,
    description: &str,
    label_true: &str,
    label_false: &str,
    filter: &mut Option<bool>,
    msg: &mut Message,
    ctx: Context<'_>
) -> Result<(), Error> {
    let current = *filter;
    let get_style = |val: bool| if current == Some(val) { ButtonStyle::Success } else { ButtonStyle::Primary };

    let make_components = |disabled| vec![
        CreateActionRow::Buttons(vec![
            CreateButton::new("filter_set_bool_true")
                .label(label_true)
                .style(get_style(true))
                .disabled(disabled),
            CreateButton::new("filter_set_bool_false")
                .label(label_false)
                .style(get_style(false))
                .disabled(disabled),
        ]),
        get_default_filter_buttons(disabled, filter.is_none())
    ];

    let embed = create_base_embed(None)
        .title(format!("Filter: {}", name))
        .description(format!(
            "**Current:** {}\n\n{}",
            SearchFilters::format_bool_filter(current),
            description
        ))
        .color(0x5865F2);

    msg.edit(
        &ctx.serenity_context().http,
        EditMessage::new().embed(embed).components(make_components(false))
    ).await?;

    let collector = ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .message_id(msg.id)
        .timeout(Duration::from_secs(30))
        .next()
        .await;

    let interaction = match collector {
        None => return Ok(()),
        Some(i) => i,
    };

    match interaction.data.custom_id.as_str() {
        "filter_back_to_home" => {
            interaction.create_response(&ctx.serenity_context().http, CreateInteractionResponse::Acknowledge).await?;
        }
        "filter_set_bool_true" | "filter_set_bool_false" => {
            let val = interaction.data.custom_id == "filter_set_bool_true";
            *filter = Some(val);

            interaction.create_response(&ctx.serenity_context().http, UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(create_success_embed(
                        &format!("{} set to: {}", name, if val { label_true } else { label_false }),
                        Some(Utc::now())
                    ))
                    .components(vec![])
            )).await?;
            tokio::time::sleep(Duration::from_secs(1)).await;
        },
        "filter_unset" => {
            *filter = None;

            interaction.create_response(
                &ctx.serenity_context().http,
                UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(create_success_embed(&format!("Unset {} filter", name), None))
                        .components(vec![])
                )
            ).await?;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_filter_selection(
    ctx: Context<'_>,
    msg: &mut Message,
    selected: &str,
    filter: &mut SearchFilters
) -> Result<(), Error> {
    match selected {
        "edit_description" => {
            string_edit_filter(
                "Description",
                "With what pattern do you want to search a server by description?\n-# The color codes are removed in the search process",
                "e.g. A Minecraft Server",
                &mut filter.description,
                msg,
                ctx
            ).await?;
        }

        "edit_max_players" => {
            integer_edit_filter(
                "Max Players",
                "Filter by maximum player slots.",
                "e.g. 100",
                &mut filter.players_max,
                msg,
                ctx
            ).await?;
        }
        "edit_online_players" => {
            integer_edit_filter(
                "Online Players",
                "Filter by online players.",
                "e.g. 5",
                &mut filter.players_online,
                msg,
                ctx
            ).await?;
        }

        "edit_version" => {
            string_edit_filter(
                "Version",
                "What version should the server run?",
                "e.g. Paper 1.21.5",
                &mut filter.version_name,
                msg,
                ctx
            ).await?;
        }

        "edit_enforce_secure_chat" => {
            boolean_edit_filter(
                "Enforce Secure Chat",
                "Should the server enforce a secure (signed) chat?",
                "Yes",
                "No",
                &mut filter.enforces_secure_chat,
                msg,
                ctx
            ).await?;
        }

        "edit_modding" => {
            boolean_edit_filter(
                "Modding",
                "Search for modded or vanilla servers.",
                "Modded",
                "Vanilla",
                &mut filter.is_modded,
                msg,
                ctx
            ).await?;
        }

        "edit_cracked" => {
            boolean_edit_filter(
                "Cracked",
                "Allow players without premium accounts?",
                "Cracked",
                "Premium",
                &mut filter.cracked,
                msg,
                ctx
            ).await?;
        }

        "edit_whitelist" => {
            boolean_edit_filter(
                "Whitelist",
                "Should the server have an active whitelist?",
                "Whitelisted",
                "Open",
                &mut filter.whitelist,
                msg,
                ctx
            ).await?;
        }

        "edit_software" => {
            string_edit_filter(
                "Software",
                "How do you want to search for the server software/implementation?",
                "e.g. Paper, Spigot, Velocity...",
                &mut filter.software_name,
                msg,
                ctx
            ).await?;
        }

        "edit_kick_message" => {
            string_edit_filter(
                "Kick Message",
                "Search for servers based on the message shown when being kicked or rejected.",
                "e.g. Whitelisted, Maintenance...",
                &mut filter.kick_message,
                msg,
                ctx
            ).await?;
        }

        "edit_plugins" => {
            let current = &filter.plugin_name;
            let embed = create_base_embed(None)
                .title("🧩 Filter: Plugins")
                .color(0x5865F2)
                .description(format!(
                    "**Current Filter:** {}\n\nEnter the name of a plugin to search for (e.g., Essentials, WorldEdit).",
                    current.as_ref().map(|s| format!("`{}`", s)).unwrap_or_else(|| "Any".to_string())
                ));

            let components = |disabled| vec![
                CreateActionRow::Buttons(vec![
                    CreateButton::new("filter_back_to_main")
                        .label("Back")
                        .emoji('⬅')
                        .style(ButtonStyle::Secondary)
                        .disabled(disabled),
                    CreateButton::new("filter_set_plugin")
                        .label("Set Plugin")
                        .emoji('🔎')
                        .style(if current.is_some() { ButtonStyle::Success } else { ButtonStyle::Primary })
                        .disabled(disabled),
                    CreateButton::new("filter_unset")
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
                    "filter_set_plugin" => {
                        let dur = format!("<t:{}:R>", (Utc::now() + chrono::Duration::seconds(25)).timestamp());

                        msg.edit(
                            &ctx.serenity_context().http,
                            EditMessage::new()
                                .embed(create_loading_embed(&format!("Waiting for plugin name (Expires {})", dur)))
                                .components(components(true))
                        ).await?;

                        if let Ok(Some(input)) = open_string_input_modal(
                            ctx,
                            &mci,
                            "Filter: Plugins",
                            "Enter plugin name:",
                            "e.g. Essentials, LuckPerms, ViaVersion..."
                        ).await {
                            filter.plugin_name = Some(input);
                            return Ok(());
                        }
                    },
                    "filter_unset" => {
                        filter.plugin_name = None;
                        mci.create_response(
                            &ctx.serenity_context().http, UpdateMessage(
                                CreateInteractionResponseMessage::new()
                                    .embed(create_success_embed("Cleared plugin filter", None))
                                    .components(vec![])
                            )
                        ).await?;
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        return Ok(())
                    },
                    _ => {}
                }
            }
        }

        _ => {}
    }

    Ok(())
}