use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Chat,
    Stats,
    Settings,
}

pub struct Message {
    pub from_user: bool,
    pub text: String,
}

#[derive(Default)]
pub struct Stats {
    pub messages_sent: u64,
    pub day: u64,
    pub prev_day: u64,
    pub week: u64,
    pub prev_week: u64,
    pub month: u64,
    pub prev_month: u64,
    pub total: u64,
    pub db_bytes: u64,
}

pub struct App {
    pub tab: Tab,
    pub messages: Vec<Message>,
    pub input: String,
    pub api_key: String,
    pub model: String,
    pub settings_sel: usize,
    pub editing: bool,
    pub stats: Stats,
    pub waiting: bool,
    pub tokens_limit: u64,
}

impl Default for App {
    fn default() -> Self {
        Self {
            tab: Tab::Chat,
            messages: vec![],
            input: String::new(),
            api_key: std::env::var("OLLAMA_API_KEY").unwrap_or_default(),
            model: "gemma4:31b".into(),
            settings_sel: 0,
            editing: false,
            stats: Stats::default(),
            waiting: false,
            tokens_limit: 3_000_000,
        }
    }
}

impl App {
    pub fn next_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Chat => Tab::Stats,
            Tab::Stats => Tab::Settings,
            Tab::Settings => Tab::Chat,
        };
    }

    pub fn prev_tab(&mut self) {
        self.tab = match self.tab {
            Tab::Chat => Tab::Settings,
            Tab::Stats => Tab::Chat,
            Tab::Settings => Tab::Stats,
        };
    }

    pub fn field_mut(&mut self) -> &mut String {
        if self.settings_sel == 0 { &mut self.api_key } else { &mut self.model }
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let [tabs_area, body] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(f.area());

    let idx = match app.tab {
        Tab::Chat => 0,
        Tab::Stats => 1,
        Tab::Settings => 2,
    };
    let tabs = Tabs::new(["Chat", "Stats", "Settings"])
        .select(idx)
        .block(Block::default().borders(Borders::ALL).title(" Allma Agent (←/→ — сменить вкладку) "))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, tabs_area);

    match app.tab {
        Tab::Chat => draw_chat(f, app, body),
        Tab::Stats => draw_stats(f, app, body),
        Tab::Settings => draw_settings(f, app, body),
    }
}

fn draw_chat(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let [log, input] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);

    let lines: Vec<Line> = app
        .messages
        .iter()
        .map(|m| {
            if m.from_user {
                Line::styled(format!("Ты: {}", m.text), Style::default().fg(Color::Green))
            } else {
                Line::styled(format!("ИИ: {}", m.text), Style::default().fg(Color::Yellow))
            }
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Чат ")),
        log,
    );
    f.render_widget(
        Paragraph::new(app.input.as_str())
            .block(Block::default().borders(Borders::ALL).title(if app.waiting { " ИИ думает… " } else { " Сообщение (Enter) " })),
        input,
    );
}

fn draw_stats(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let [info, bar] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);

    let s = &app.stats;
    let sep = "─".repeat(info.width.saturating_sub(2) as usize);
    let text = vec![
        Line::from(format!("Отправлено сообщений: {}", fmt_num(s.messages_sent))),
        stat_line("Токенов за день", s.day, s.prev_day),
        stat_line("Токенов за неделю", s.week, s.prev_week),
        stat_line("Токенов за месяц", s.month, s.prev_month),
        Line::from(format!("Токенов за всё время: {}", fmt_num(s.total))),
        Line::styled(sep, Style::default().fg(Color::DarkGray)),
        Line::from(format!("Размер базы данных: {}", fmt_bytes(s.db_bytes))),
    ];
    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Статистика ")),
        info,
    );

    let ratio = (s.total as f64 / app.tokens_limit as f64).clamp(0.0, 1.0);
    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" Лимит токенов "))
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(ratio)
            .label(format!("{} / {}", fmt_num(s.total), fmt_num(app.tokens_limit))),
        bar,
    );
}

fn stat_line(label: &str, cur: u64, prev: u64) -> Line<'static> {
    let mut spans = vec![Span::raw(format!("{label}: {}  ", fmt_num(cur)))];
    if prev == 0 {
        spans.push(Span::styled("—", Style::default().fg(Color::DarkGray)));
    } else {
        let pct = (cur as f64 - prev as f64) / prev as f64 * 100.0;
        let (arrow, color) = if pct >= 0.0 {
            ("▲", Color::Red)
        } else {
            ("▼", Color::Green)
        };
        spans.push(Span::styled(
            format!("{arrow} {:.1}%", pct.abs()),
            Style::default().fg(color),
        ));
    }
    Line::from(spans)
}

fn fmt_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn fmt_bytes(b: u64) -> String {
    const U: [&str; 4] = ["Б", "КБ", "МБ", "ГБ"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{b} Б") } else { format!("{v:.1} {}", U[i]) }
}

fn draw_settings(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let masked = "*".repeat(app.api_key.chars().count());
    let rows = [
        format!("Ollama API key: {masked}"),
        format!("Модель: {}", app.model),
    ];
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut style = Style::default();
            if i == app.settings_sel {
                style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
            }
            let mark = if i == app.settings_sel && app.editing { " ✎" } else { "" };
            ListItem::new(format!("{r}{mark}")).style(style)
        })
        .collect();
    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Настройки (↑↓ выбор, E — править) "),
        ),
        area,
    );
}