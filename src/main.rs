mod ai;
mod ui;

use ai::AiEvent;
use std::{sync::mpsc, time::Duration};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ui::{App, Message, Tab};

use ratatui::crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
};

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let _ = execute!(std::io::stdout(), EnableBracketedPaste);
    let mut app = App::default();
    let (tx, rx) = mpsc::channel::<AiEvent>();
    let res = (|| loop {
        while let Ok(ev) = rx.try_recv() {
            app.waiting = false;
            match ev {
                AiEvent::Reply { text, tokens } => {
                    app.messages.push(Message { from_user: false, text });
                    let s = &mut app.stats;
                    s.day += tokens;
                    s.week += tokens;
                    s.month += tokens;
                    s.total += tokens;
                }
                AiEvent::Error(e) => app.messages.push(Message {
                    from_user: false,
                    text: format!("⚠ {e}"),
                }),
            }
        }
        terminal.draw(|f| ui::draw(f, &app))?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let k = match event::read()? {
            Event::Key(k) => k,
            Event::Paste(text) => {
                let clean: String = text
                    .replace(['\r', '\n'], " ")
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect();
                if app.tab == Tab::Settings && app.editing {
                    app.field_mut().push_str(clean.trim());
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
                KeyCode::Char(c) => app.input.push(c),
                KeyCode::Backspace => { app.input.pop(); }
                KeyCode::Enter if !app.input.is_empty() && !app.waiting => {
                    let text = std::mem::take(&mut app.input);
                    app.messages.push(Message { from_user: true, text });
                    app.stats.messages_sent += 1;
                    if app.api_key.is_empty() {
                        app.messages.push(Message {
                            from_user: false,
                            text: "⚠ Нет API-ключа: вставь его во вкладке Settings".into(),
                        });
                    } else {
                        app.waiting = true;
                        let history = app
                            .messages
                            .iter()
                            .filter(|m| !m.text.starts_with('⚠'))
                            .map(|m| ai::ChatMsg {
                                role: (if m.from_user { "user" } else { "assistant" }).into(),
                                content: m.text.clone(),
                            })
                            .collect();
                        ai::ask(app.api_key.clone(), app.model.clone(), history, tx.clone());
                    }
                }
                _ => {}
            },
            Tab::Settings if app.editing => match k.code {
                KeyCode::Char(c) => app.field_mut().push(c),
                KeyCode::Backspace => { app.field_mut().pop(); }
                KeyCode::Enter | KeyCode::Esc => app.editing = false,
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
    let _ = execute!(std::io::stdout(), DisableBracketedPaste);
    ratatui::restore();
    res
}