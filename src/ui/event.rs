use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;

use crate::app::App;
use crate::domain::SortMode;
use crate::ui::render::render;

pub fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| render(f, app))?;

        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.search_active {
                        handle_search_input(app, key.code)?;
                    } else {
                        if handle_normal_input(app, key.code)? {
                            return Ok(());
                        }
                    }
                }
            }
        }

        app.refresh();
    }
}

fn handle_search_input(app: &mut App, key_code: KeyCode) -> Result<()> {
    match key_code {
        KeyCode::Esc => {
            app.filter_text.clear();
            app.search_active = false;
            app.table_state.select(Some(0));
        }
        KeyCode::Enter => {
            app.search_active = false;
        }
        KeyCode::Backspace => {
            app.filter_text.pop();
            app.table_state.select(Some(0));
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            app.table_state.select(Some(0));
        }
        _ => {}
    }
    Ok(())
}

fn handle_normal_input(app: &mut App, key_code: KeyCode) -> Result<bool> {
    match key_code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc => {
            if !app.filter_text.is_empty() {
                app.filter_text.clear();
                app.table_state.select(Some(0));
            } else {
                return Ok(true);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.previous(),
        KeyCode::Char(' ') | KeyCode::Char('x') => app.toggle_current(),
        KeyCode::Char('c') => app.toggle_sort_mode(SortMode::Cpu),
        KeyCode::Char('m') => app.toggle_sort_mode(SortMode::Memory),
        KeyCode::Char('d') => app.toggle_sort_mode(SortMode::DiskIo),
        KeyCode::Char('n') => app.toggle_sort_mode(SortMode::Name),
        KeyCode::Char('s') => {
            app.search_active = true;
        }
        _ => {}
    }
    Ok(false)
}
