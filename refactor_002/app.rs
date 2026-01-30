use std::time::Duration;
use crossterm::event;
use crossterm::event::KeyEventKind;
use ratatui::layout::{ Alignment, Constraint, Direction, Layout };
use ratatui::prelude::{ Color, Line, Span, Style, Stylize };
use ratatui::widgets::{ Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState };
use crate::{is_online, get_version_raw, render_banner};
use crate::logger;
use crate::manager;
use crate::updater;

#[derive(PartialEq)]
enum Focus {
    Console,
    Tasks,
}

pub enum AppEvent {
    InternetStatus(bool),
    GithubData(Option<updater::GithubRelease>, Option<usize>),
    TaskUpdate(Vec<(String, manager::TaskState)>),
    AddLog(Line<'static>),
}

pub struct App {
    should_quit: bool,

    do_update_check: bool,
    latest_version: Option<updater::GithubRelease>,
    behind: usize,
    is_online: Option<bool>,

    focus: Focus,

    tasks_state: ListState,
    tasks: Vec<(String, manager::TaskState)>,
    tasks_autoscroll: bool,

    log_level: logger::LogLevel,
    console_state: ListState,
    logs: Vec<Line<'static>>,
    console_autoscroll: bool,
}

impl App {
    pub fn new(no_check: bool, log_level: logger::LogLevel) -> Self {
        Self {
            should_quit: false,

            do_update_check: !no_check,
            latest_version: None,
            behind: 0,
            is_online: None,

            focus: Focus::Console,

            tasks_state: Default::default(),
            tasks: vec![],
            tasks_autoscroll: true,

            log_level,
            console_state: Default::default(),
            logs: vec![],
            console_autoscroll: true,
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> crate::Result<()> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<AppEvent>(100);

        logger::init(event_tx.clone(), self.log_level.clone());

        let net_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                let online = is_online().await;

                if net_tx.send(AppEvent::InternetStatus(online)).await.is_err() {
                    break;
                }
            }
        });

        if self.do_update_check {
            let git_tx = event_tx.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_mins(15));
                loop {
                    interval.tick().await;
                    let github = updater::GithubAPI::new("Cyberdolfi", "ServerRawler", "ServerRawler");

                    let release = match github.get_latest_release().await {
                        Ok(var1) => Some(var1),
                        Err(_) => None,
                    };

                    let behind = match github.get_behind(&get_version_raw(), updater::ReleaseFilter::All).await {
                        Ok(var1) => Some(var1),
                        Err(_) => None
                    };

                    if git_tx.send(AppEvent::GithubData(release, behind)).await.is_err() {
                        break;
                    }
                }
            });
        }

        let tasks_tx = event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            let mut last_snapshot: Vec<(String, manager::TaskState)> = Vec::new();

            loop {
                interval.tick().await;
                let current_tasks = manager::TaskManager::list_all().await;

                if current_tasks.len() != last_snapshot.len() || !current_tasks.is_empty() {
                    if tasks_tx.send(AppEvent::TaskUpdate(current_tasks.clone())).await.is_err() {
                        break;
                    }
                    last_snapshot = current_tasks;
                }
            }
        });

        // Main loop
        while !self.should_quit {
            terminal.draw(|frame| {
                let _ = self.render(frame);
            })?;

            // Processing event data
            while let Ok(event) = event_rx.try_recv() {
                match event {
                    AppEvent::InternetStatus(status) => {
                        self.is_online = Some(status);
                    }

                    AppEvent::GithubData(release, behind) => {
                        if let Some(r) = release {
                            self.latest_version = Some(r);
                        }
                        if let Some(b) = behind {
                            self.behind = b;
                        }
                    }

                    AppEvent::TaskUpdate(updated_tasks) => {
                        self.tasks = updated_tasks;

                        let len = self.tasks.len();
                        if len == 0 {
                            self.tasks_state.select(None);
                        } else if let Some(selected) = self.tasks_state.selected() {
                            if selected >= len {
                                self.tasks_state.select(Some(len.saturating_sub(1)));
                            }
                        }

                        if self.tasks_autoscroll && len > 0 {
                            let last_idx = len.saturating_sub(1);
                            self.tasks_state.select(Some(last_idx));
                        }
                    }

                    AppEvent::AddLog(line) => {
                        self.add_log(line);
                    }
                }
            }

            if event::poll(Duration::from_millis(50))? {
                self.handle_events()?;
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut ratatui::Frame) -> crate::Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9),
                Constraint::Min(0)
            ])
            .split(frame.area());

        let lower_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Fill(1)
            ])
            .split(chunks[1]);

        let banner_lines = render_banner(self.is_online, self.latest_version.clone(), self.behind);
        let banner_block = Paragraph::new(banner_lines)
            .block(
                Block::default()
                    .title_bottom(
                        Line::from(" https://github.com/Cyberdolfi/ServerRawler ")
                            .fg(Color::Red)
                            .alignment(Alignment::Center)
                    )
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(117, 117, 117)))
            )
            .centered();

        frame.render_widget(banner_block, chunks[0]);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .thumb_symbol("┃")
            .track_symbol(Some("┆"))
            .style(Style::default().fg(Color::Rgb(255, 215, 0)))
            .track_style(Style::default().fg(Color::Rgb(64, 64, 64)))
            .begin_style(Style::default().fg(Color::Rgb(255, 69, 0)))
            .end_style(Style::default().fg(Color::Rgb(255, 69, 0)));

        // Console

        let console_block = Block::default()
            .title(Line::from(if self.focus == Focus::Console { " [ Console ] " } else { " Console " }).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.focus == Focus::Console { Color::Gray } else { Color::DarkGray }));

        let console_width = (lower_chunks[0].width as usize).saturating_sub(4);
        let indent_width = 21;
        let message_wrap_width = console_width.saturating_sub(indent_width);

        let logs: Vec<ListItem> = self.logs.iter().map(|l| {
            let full_text = l.spans.iter().map(|s| s.content.as_ref()).collect::<String>();

            if full_text.len() <= console_width {
                return ListItem::new(l.clone());
            }

            let (header_part, message_part) = if full_text.len() > indent_width {
                (&full_text[..indent_width], &full_text[indent_width..])
            } else {
                (full_text.as_str(), "")
            };

            let wrapped_message = textwrap::wrap(message_part, message_wrap_width);

            let mut lines_in_item = Vec::new();

            for (i, msg_line) in wrapped_message.into_iter().enumerate() {
                if i == 0 {
                    let mut spans = Vec::new();
                    let mut current_len = 0;

                    for span in &l.spans {
                        if current_len < indent_width {
                            spans.push(span.clone());
                            current_len += span.content.len();
                        }
                    }
                    spans.push(Span::raw(msg_line.to_string()));
                    lines_in_item.push(Line::from(spans));
                } else {
                    let indent = " ".repeat(indent_width);
                    lines_in_item.push(Line::from(vec![
                        Span::raw(indent),
                        Span::raw(msg_line.to_string())
                    ]));
                }
            }
            ListItem::new(lines_in_item)
        }).collect();

        let console_list = List::new(logs)
            .block(console_block)
            .highlight_symbol(Span::styled("❯ ", Style::default().fg(Color::Rgb(255, 69, 0)).bold()))
            .highlight_style(Style::default().bg(Color::Rgb(45, 45, 45)));

        frame.render_stateful_widget(console_list, lower_chunks[0], &mut self.console_state);

        if !self.logs.is_empty() {
            frame.render_stateful_widget(
                scrollbar.clone(),
                lower_chunks[0].inner(ratatui::prelude::Margin { vertical: 1, horizontal: 0 }),
                &mut ScrollbarState::new(self.logs.len()).position(self.console_state.selected().unwrap_or(0))
            );
        }

        // Running Tasks

        let tasks_block = Block::default()
            .title(Line::from(if self.focus == Focus::Tasks { " [ Running Tasks ] " } else { " Running Tasks " }).alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.focus == Focus::Tasks { Color::Gray } else { Color::DarkGray }));

        let task_items: Vec<ListItem> = self.tasks.iter().map(|(id, state)| {
            let width: usize = 15;

            let ratio = (state.progress_bar / 100.0).clamp(0.0, 1.0);
            let filled_len = (ratio * width as f32) as usize;
            let empty_len = width.saturating_sub(filled_len);

            let filled_bar = "█".repeat(filled_len);
            let empty_bar = "░".repeat(empty_len);

            let content = Line::from(vec![
                Span::styled("● ", Style::default().fg(if state.progress_bar >= 100.0 { Color::Green } else { Color::Rgb(255, 69, 0) })),

                Span::styled(format!("{:<10} ", id), Style::default().fg(Color::White).bold()),

                Span::styled(filled_bar, Style::default().fg(Color::Rgb(255, 215, 0))),
                Span::styled(empty_bar, Style::default().fg(Color::Rgb(64, 64, 64))),

                Span::styled(format!(" {:>5.1}% ", state.progress_bar), Style::default().fg(Color::Rgb(255, 255, 0))),
                Span::styled(
                    format!("[{}/{}] ", state.progress_current, state.progress_max),
                    Style::default().fg(Color::Rgb(100, 100, 100))
                ),

                Span::styled("» ", Style::default().fg(Color::Rgb(255, 69, 0))),
                Span::styled(state.message.clone(), Style::default().fg(Color::Gray).italic()),
            ]);

            ListItem::new(content)
        }).collect();

        let tasks_list = List::new(task_items)
            .block(tasks_block)
            .highlight_symbol(Span::styled("❯ ", Style::default().fg(Color::Rgb(255, 69, 0)).bold()))
            .highlight_style(Style::default().bg(Color::Rgb(45, 45, 45)));

        frame.render_stateful_widget(tasks_list, lower_chunks[1], &mut self.tasks_state);

        if !self.tasks.is_empty() {
            frame.render_stateful_widget(
                scrollbar.clone(),
                lower_chunks[1].inner(ratatui::prelude::Margin { vertical: 1, horizontal: 0 }),
                &mut ScrollbarState::new(self.tasks.len()).position(self.tasks_state.selected().unwrap_or(0))
            );
        }
        Ok(())
    }

    fn scroll_down(&mut self) {
        let state = match self.focus {
            Focus::Console => &mut self.console_state,
            Focus::Tasks => &mut self.tasks_state,
        };

        let len = match self.focus {
            Focus::Console => self.logs.len(),
            Focus::Tasks => self.tasks.len(),
        };

        if len == 0 { return; }

        let i = match state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    match self.focus {
                        Focus::Console => self.console_autoscroll = true,
                        Focus::Tasks => self.tasks_autoscroll = true,
                    }
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn scroll_up(&mut self) {
        match self.focus {
            Focus::Console => self.console_autoscroll = false,
            Focus::Tasks => self.tasks_autoscroll = false
        }

        let state = match self.focus {
            Focus::Console => &mut self.console_state,
            Focus::Tasks => &mut self.tasks_state,
        };

        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    fn add_log(&mut self, log: Line<'static>) {
        self.logs.push(log);
        if self.console_autoscroll {
            let last_idx = self.logs.len().saturating_sub(1);
            self.console_state.select(Some(last_idx));
        }
        if self.tasks_autoscroll {
            let last_idx = self.tasks.len().saturating_sub(1);
            self.tasks_state.select(Some(last_idx));
        }
    }

    fn handle_events(&mut self) -> crate::Result<()> {
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    // (Rage) quitting
                    // TODO: Make a safe quit (For example shutting down all tasks)
                    event::KeyCode::Char('q') => self.should_quit = true,
                    event::KeyCode::Esc => self.should_quit = true,

                    event::KeyCode::Tab => {
                        self.focus = match self.focus {
                            Focus::Console => Focus::Tasks,
                            Focus::Tasks => Focus::Console,
                        };
                    }

                    event::KeyCode::Up => self.scroll_up(),
                    event::KeyCode::Down => self.scroll_down(),

                    event::KeyCode::Right => self.focus = Focus::Tasks,
                    event::KeyCode::Left => self.focus = Focus::Console,
                    // TESTING
                    event::KeyCode::Char('b') => {
                        tokio::spawn(async move {
                            let ip = "209.222.115.42".to_string();
                            let port = 25565;

                            manager::TaskManager::spawn("Ping", move |_, _| async move {
                                match crate::minecraft::ping::execute_ping(ip, port, 767, Duration::from_secs(5)).await {
                                    Ok(res) => {
                                        logger::success(format!("Ping: {}ms", res.latency)).send().await;
                                    },
                                    Err(e) => {
                                        logger::error(format!("Fehler: {}", e)).send().await;
                                    },
                                }
                            }).await;
                        });
                    },
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
