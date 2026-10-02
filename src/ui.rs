use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Line,
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

pub struct App {
    pub tab: Tab,
    pub messages: Vec<Message>,
    pub input: String,
    pub api_key: String,
    pub model: String,
    pub settings_sel: usize,
    pub editing: bool,
    pub tokens_used: u64,
    pub tokens_limit: u64,
}

impl Default for App {
    fn default() -> Self {
        Self {
            tab: Tab::Chat,
            messages: vec![],
            input: String::new(),
            api_key: String::new(),
            model: "gemma4:31b".into(),
            settings_sel: 0,
            editing: false,
            tokens_used: 0,
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
        .block(Block::default().borders(Borders::ALL).title(" chatbuddy (Tab — сменить) "))
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
            .block(Block::default().borders(Borders::ALL).title(" Сообщение (Enter) ")),
        input,
    );
}

fn draw_stats(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let [info, bar] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);

    let user = app.messages.iter().filter(|m| m.from_user).count();
    let ai = app.messages.len() - user;
    let text = vec![
        Line::from(format!("Всего сообщений: {}", app.messages.len())),
        Line::from(format!("Твоих: {user}")),
        Line::from(format!("Ответов ИИ: {ai}")),
    ];
    f.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Статистика ")),
        info,
    );

    let ratio = (app.tokens_used as f64 / app.tokens_limit as f64).clamp(0.0, 1.0);
    f.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" Лимит токенов "))
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(ratio)
            .label(format!("{} / {}", app.tokens_used, app.tokens_limit)),
        bar,
    );
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
                .title(" Настройки (↑↓ выбор, Enter — править) "),
        ),
        area,
    );
}