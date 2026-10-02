mod ui;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ui::{App, Message, Tab};

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::default();
    let res = (|| loop {
        terminal.draw(|f| ui::draw(f, &app))?;
        let Event::Key(k) = event::read()? else { continue };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
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
                KeyCode::Enter if !app.input.is_empty() => {
                    let text = std::mem::take(&mut app.input);
                    app.messages.push(Message { from_user: true, text });
                    app.stats.messages_sent += 1;
                    // TODO: запрос к Ollama
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
    ratatui::restore();
    res
}