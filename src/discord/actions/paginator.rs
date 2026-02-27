// // Sample basic Paginator
//
// async fn server_paginator(
//     ctx: Context<'_>,
//     reply: ReplyHandle<'_>,
//     pages: Vec<CreateEmbed>,
// ) -> Result<(), Error> {
//     let mut current_page = 0;
//     let total_pages = pages.len();
//
//     let make_components = |page: usize, total: usize, disabled: bool| {
//         vec![serenity::builder::CreateActionRow::Buttons(vec![
//             serenity::builder::CreateButton::new("first")
//                 .emoji('⏮')
//                 .style(ButtonStyle::Secondary)
//                 .disabled(disabled || page == 0),
//             serenity::builder::CreateButton::new("prev")
//                 .emoji('◀')
//                 .style(ButtonStyle::Secondary)
//                 .disabled(disabled || page == 0),
//             serenity::builder::CreateButton::new("next")
//                 .emoji('▶')
//                 .style(ButtonStyle::Secondary)
//                 .disabled(disabled || page == total - 1),
//             serenity::builder::CreateButton::new("last")
//                 .emoji('⏭')
//                 .style(ButtonStyle::Secondary)
//                 .disabled(disabled || page == total - 1),
//         ])]
//     };
//
//     let mut page = pages[current_page].clone()
//         .footer(CreateEmbedFooter::new(
//             format!(
//                 "ServerCrawler {} • Page {}/{}",
//                 crate::get_version_raw(),
//                 current_page + 1,
//                 total_pages
//             )
//         ));
//
//     reply.edit(ctx, CreateReply::default()
//         .embed(page.clone())
//         .components(make_components(current_page, total_pages, false))
//     ).await?;
//
//     let mut collector = ComponentInteractionCollector::new(ctx.serenity_context())
//         .author_id(ctx.author().id)
//         .message_id(reply.message().await?.id)
//         .timeout(Duration::from_secs(120))
//         .stream();
//
//     while let Some(mci) = collector.next().await {
//         match mci.data.custom_id.as_str() {
//             "first" => current_page = 0,
//             "prev" => if current_page > 0 { current_page -= 1 },
//             "next" => if current_page < total_pages - 1 { current_page += 1 },
//             "last" => current_page = total_pages - 1,
//             _ => continue,
//         }
//
//         page = pages[current_page].clone()
//             .footer(CreateEmbedFooter::new(
//                 format!(
//                     "ServerCrawler {} • Page {}/{}",
//                     crate::get_version_raw(),
//                     current_page + 1,
//                     total_pages
//                 )
//             ));
//
//         mci.create_response(
//             &ctx.serenity_context(),
//             serenity::builder::CreateInteractionResponse::UpdateMessage(
//                 serenity::builder::CreateInteractionResponseMessage::new()
//                     .embed(page.clone())
//                     .components(make_components(current_page, total_pages, false))
//             )
//         ).await?;
//     }
//
//     // Disable buttons
//     let _ = reply.edit(ctx, CreateReply::default()
//         .embed(page)
//         .components(make_components(current_page, total_pages, true))
//     ).await;
//
//     Ok(())
// }