use std::time::Duration;
use chrono::Utc;
use poise::CreateReply;
use serenity::all::{GetMessages, Message, MessageId};
use crate::config;
use crate::discord::{create_error_embed, create_loading_embed, create_success_embed, Context, Error};

#[poise::command(prefix_command, required_permissions = "MANAGE_MESSAGES")]
pub async fn cleanup(
    ctx: Context<'_>,
    #[description = "Deletes messages from everyone or only from the bot"]
    from_everyone: Option<bool>
) -> Result<(), Error> {
    let config = config::MainConfig::get().ok();
    let admin_role_ids = config.and_then(|c| c.discord.admin_roles.clone()).unwrap_or_default();

    if !admin_role_ids.is_empty() {
        let has_role = match ctx.author_member().await {
            Some(member) => member.roles.iter().any(|role_id| {
                admin_role_ids.contains(&role_id.get())
            }),
            None => false,
        };

        if !has_role {
            ctx.send(CreateReply::default()
                .embed(create_error_embed("You do not have the required admin roles to use this command.", None))
            ).await?;

            tokio::time::sleep(Duration::from_secs(3)).await;
            return Ok(());
        }
    }

    let channel_id = ctx.channel_id();
    let start_time = Utc::now();
    let bot_id = ctx.cache().current_user().id;
    let is_everyone = from_everyone.unwrap_or(false);

    let reply = ctx.send(
        CreateReply::default().embed(create_loading_embed("fetching messages and deleting threads..."))
    ).await?;

    if let Some(guild_id) = ctx.guild_id() {
        let threads = guild_id.get_active_threads(&ctx.http()).await?;

        for thread in threads.threads {
            if thread.parent_id == Some(channel_id) && thread.owner_id == Some(bot_id) {
                thread.delete(&ctx.http()).await?;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }

    let reply_id = reply.message().await?.id;

    let mut all_messages: Vec<Message> = Vec::new();
    let mut last_id: Option<MessageId> = None;

    loop {
        let mut getter = GetMessages::new().limit(100);
        if let Some(id) = last_id {
            getter = getter.before(id);
        }

        let fetched = channel_id.messages(&ctx.http(), getter).await?;
        if fetched.is_empty() { break; }

        last_id = fetched.last().map(|m| m.id);
        all_messages.extend(fetched);
    }

    all_messages.retain(|m| m.id != reply_id);

    if !is_everyone {
        all_messages.retain(|m| m.author.id == bot_id);
    }

    if all_messages.is_empty() {
        reply.edit(ctx, CreateReply::default().embed(
            create_error_embed("No messages found to delete!", Some(start_time))
        )).await?;

        return Ok(());
    }

    all_messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let two_weeks_ago = Utc::now() - chrono::Duration::weeks(2);
    let (young_messages, old_messages): (Vec<Message>, Vec<Message>) = all_messages
        .into_iter()
        .partition(|m| m.timestamp.unix_timestamp() > two_weeks_ago.timestamp());

    reply.edit(ctx, CreateReply::default().embed(
        create_loading_embed(&format!(
            "deleting {} young (bulk) and {} old (single) messages...",
            young_messages.len(),
            old_messages.len()
        ))
    )).await?;

    for chunk in young_messages.chunks(100) {
        channel_id.delete_messages(&ctx.http(), chunk).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    for msg in old_messages.clone() {
        if let Err(e) = msg.delete(&ctx.http()).await {
            continue;
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    reply.edit(ctx, CreateReply::default().embed(
        create_success_embed(
            &format!("Cleanup finished. Total messages processed: {}", young_messages.len() + old_messages.len()),
            Some(start_time)
        )
    )).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;
    let _ = reply.delete(ctx).await;

    Ok(())
}