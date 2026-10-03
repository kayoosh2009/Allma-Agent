mod ai;
mod database;
mod ui;

use ai::AiEvent;
use std::{sync::mpsc, time::Duration};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ui::{App, Message, Tab};

use ratatui::crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};

fn main() -> std::io::Result<()> {
    let db = database::Db::open().expect("не удалось открыть базу данных");
    let mut terminal = ratatui::init();
    let _ = execute!(std::io::stdout(), EnableBracketedPaste);
    if ratatui::crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false) {
        let _ = execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        );
    }
    let mut app = App::default();
    app.messages = db
        .load_messages()
        .into_iter()
        .map(|(from_user, text)| Message { from_user, text })
        .collect();
    app.stats = db.stats();
    if let Some(k) = db.get_setting("api_key") {
        app.api_key = k;
    }
    if let Some(m) = db.get_setting("model") {
        app.model = m;
    }
    let (tx, rx) = mpsc::channel::<AiEvent>();
    let res = (|| loop {
        while let Ok(ev) = rx.try_recv() {
            app.waiting = false;
            match ev {
                AiEvent::Reply { text, tokens } => {
                    let (mut text, notes) = ai::extract_diary(&text);
                    for n in &notes {
                        let _ = db.append_diary(&n.replace('\n', " "));
                    }
                    if !notes.is_empty() {
                        if !text.is_empty() {
                            text.push_str("\n\n");
                        }
                        text.push_str(&format!("{} Записал в дневник", ui::DIARY_MARK));
                    }
                    let _ = db.add_message(false, &text, tokens);
                    app.messages.push(Message { from_user: false, text });
                    app.scroll = 0;
                    app.stats = db.stats();
                }
                AiEvent::Error(e) => app.messages.push(Message {
                    from_user: false,
                    text: format!("⚠ {e}"),
                }),
            }
        }
        terminal.draw(|f| ui::draw(f, &mut app))?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let k = match event::read()? {
            Event::Key(k) => k,
            Event::Paste(text) => {
                let clean: String = text
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .chars()
                    .filter(|c| *c == '\n' || !c.is_control())
                    .collect();
                if app.tab == Tab::Settings && app.editing {
                    app.field_mut().push_str(clean.replace('\n', " ").trim());
                } else if app.tab == Tab::Chat {
                    app.input.push_str(&clean);
                }
                continue;
            }
            _ => continue,
        };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
            break Ok(());
        }
        if k.code == KeyCode::Esc && !app.editing {
            break Ok(());
        }
        if !app.editing {
            match k.code {
                KeyCode::Right => { app.next_tab(); continue; }
                KeyCode::Left => { app.prev_tab(); continue; }
                _ => {}
            }
        }
        match app.tab {
            Tab::Chat => match k.code {
                KeyCode::Up => app.scroll -= 1,
                KeyCode::Down => app.scroll += 1,
                KeyCode::PageUp => app.scroll -= 10,
                KeyCode::PageDown => app.scroll += 10,
                KeyCode::Char(c) => app.input.push(c),
                KeyCode::Backspace => { app.input.pop(); }
                KeyCode::Enter if k.modifiers.intersects(KeyModifiers::SHIFT | KeyModifiers::ALT) => {
                    app.input.push('\n')
                }
                KeyCode::Enter if !app.input.is_empty() && !app.waiting => {
                    let text = std::mem::take(&mut app.input);
                    if text.trim() == "/clear" {
                        db.clear_chat();
                        app.messages.clear();
                        app.scroll = 0;
                        continue;
                    }
                    let _ = db.add_message(true, &text, 0);
                    app.messages.push(Message { from_user: true, text });
                    app.scroll = 0;
                    app.stats = db.stats();
                    if app.api_key.is_empty() {
                        app.messages.push(Message {
                            from_user: false,
                            text: "⚠ Нет API-ключа: вставь его во вкладке Settings".into(),
                        });
                    } else {
                        app.waiting = true;
                        let mut history: Vec<ai::ChatMsg> = app
                            .messages
                            .iter()
                            .filter(|m| !m.text.starts_with('⚠'))
                            .map(|m| ai::ChatMsg {
                                role: (if m.from_user { "user" } else { "assistant" }).into(),
                                content: m
                                    .text
                                    .lines()
                                    .filter(|l| !l.starts_with(ui::DIARY_MARK))
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            })
                            .collect();
                        history.insert(
                            0,
                            ai::ChatMsg {
                                role: "system".into(),
                                content: ai::system_prompt(&db.read_prompt(), &db.read_diary()),
                            },
                        );
                        ai::ask(app.api_key.clone(), app.model.clone(), history, tx.clone());
                    }
                }
                _ => {}
            },
            Tab::Settings if app.editing => match k.code {
                KeyCode::Char(c) => app.field_mut().push(c),
                KeyCode::Backspace => { app.field_mut().pop(); }
                KeyCode::Enter | KeyCode::Esc => {
                    app.editing = false;
                    let _ = db.set_setting("api_key", &app.api_key);
                    let _ = db.set_setting("model", &app.model);
                }
                _ => {}
            },
            Tab::Settings => match k.code {
                KeyCode::Up => app.settings_sel = 0,
                KeyCode::Down => app.settings_sel = 1,
                KeyCode::Enter | KeyCode::Char('e' | 'E' | 'у' | 'У') => app.editing = true,
                _ => {}
            },
            Tab::Stats => {}
        }
    })();
    let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags, DisableBracketedPaste);
    ratatui::restore();
    res
}