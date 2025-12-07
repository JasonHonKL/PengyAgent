use crate::app::{App, AppState, ModelOption};
use crate::constants::DEFAULT_BASE_URL;
use std::error::Error;

pub(crate) fn handle_welcome_key(
    app: &mut App,
    key: crossterm::event::KeyCode,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn Error>> {
    match key {
        crossterm::event::KeyCode::Esc => return Err("quit".into()),
        crossterm::event::KeyCode::Enter => {
            if app.chat_input.starts_with('/') {
                let cmd = app.chat_input.clone();
                if cmd.starts_with("/new") {
                    app.create_new_session();
                    app.state = AppState::Welcome;
                    app.chat_input.clear();
                    app.input_cursor = 0;
                    return Ok(());
                }
                if cmd.starts_with("/sessions") {
                    app.previous_state = Some(AppState::Welcome);
                    app.state = AppState::SessionSelector;
                    app.session_list_state.select(Some(
                        app.current_session.min(app.sessions.len().saturating_sub(1)),
                    ));
                    app.chat_input.clear();
                    app.input_cursor = 0;
                    return Ok(());
                }
                handle_command_inline(app, &cmd, AppState::Welcome);
            } else if app.initialize_model().is_ok() {
                app.state = AppState::Chat;
                if !app.chat_input.trim().is_empty() {
                    rt.block_on(app.send_message())?;
                }
            }
        }
        crossterm::event::KeyCode::Char(c) => {
            app.chat_input.insert(app.input_cursor, c);
            app.input_cursor += 1;
            app.show_command_hints = app.chat_input.starts_with('/');
        }
        crossterm::event::KeyCode::Backspace => {
            if app.input_cursor > 0 {
                app.input_cursor -= 1;
                app.chat_input.remove(app.input_cursor);
                app.show_command_hints = app.chat_input.starts_with('/');
            }
        }
        crossterm::event::KeyCode::Left if app.input_cursor > 0 => {
            app.input_cursor -= 1;
        }
        crossterm::event::KeyCode::Right if app.input_cursor < app.chat_input.len() => {
            app.input_cursor += 1;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn handle_chat_key(
    app: &mut App,
    key: crossterm::event::KeyCode,
    rt: &tokio::runtime::Runtime,
) -> Result<(), Box<dyn Error>> {
    match key {
        crossterm::event::KeyCode::Esc => return Err("quit".into()),
        crossterm::event::KeyCode::Enter if !app.loading => {
            if app.chat_input.starts_with('/') {
                let cmd = app.chat_input.clone();
                if cmd.starts_with("/new") {
                    app.create_new_session();
                    app.state = AppState::Chat;
                    app.chat_input.clear();
                    app.input_cursor = 0;
                    return Ok(());
                }
                if cmd.starts_with("/sessions") {
                    app.previous_state = Some(AppState::Chat);
                    app.state = AppState::SessionSelector;
                    app.session_list_state.select(Some(
                        app.current_session.min(app.sessions.len().saturating_sub(1)),
                    ));
                    app.chat_input.clear();
                    app.input_cursor = 0;
                    return Ok(());
                }
                handle_command_inline(app, &cmd, AppState::Chat);
            } else if !app.chat_input.trim().is_empty() {
                rt.block_on(app.send_message())?;
                app.show_command_hints = false;
            }
        }
        crossterm::event::KeyCode::Up
        | crossterm::event::KeyCode::Down
        | crossterm::event::KeyCode::PageUp
        | crossterm::event::KeyCode::PageDown => {
            app.user_scrolled = true;
            let len = app.chat_messages.len();
            if len > 0 {
                let selected = app.list_state.selected().unwrap_or(len.saturating_sub(1));
                let amount = match key {
                    crossterm::event::KeyCode::PageUp
                    | crossterm::event::KeyCode::PageDown => 10,
                    _ => 1,
                };
                let new_selection = match key {
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::PageUp => {
                        if selected == 0 {
                            0
                        } else {
                            selected.saturating_sub(amount)
                        }
                    }
                    _ => {
                        let max_idx = len.saturating_sub(1);
                        (selected + amount).min(max_idx)
                    }
                };
                app.list_state.select(Some(new_selection));
            }
        }
        crossterm::event::KeyCode::End => {
            app.user_scrolled = false;
            if !app.chat_messages.is_empty() {
                app.list_state
                    .select(Some(app.chat_messages.len().saturating_sub(1)));
            }
        }
        crossterm::event::KeyCode::Home => {
            app.user_scrolled = true;
            app.list_state.select(Some(0));
        }
        crossterm::event::KeyCode::Char(c) => {
            app.chat_input.insert(app.input_cursor, c);
            app.input_cursor += 1;
            app.show_command_hints = app.chat_input.starts_with('/');
        }
        crossterm::event::KeyCode::Backspace => {
            if app.input_cursor > 0 {
                app.input_cursor -= 1;
                app.chat_input.remove(app.input_cursor);
                app.show_command_hints = app.chat_input.starts_with('/');
            }
        }
        crossterm::event::KeyCode::Left if app.input_cursor > 0 => {
            app.input_cursor -= 1;
        }
        crossterm::event::KeyCode::Right if app.input_cursor < app.chat_input.len() => {
            app.input_cursor += 1;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn handle_command_inline(app: &mut App, cmd: &str, previous_state: AppState) {
    if cmd.starts_with("/models") {
        app.previous_state = Some(previous_state);
        app.state = AppState::ModelSelector;
    } else if cmd.starts_with("/agents") {
        app.previous_state = Some(previous_state);
        app.state = AppState::AgentSelector;
    } else if cmd.starts_with("/settings") {
        app.previous_state = Some(previous_state);
        app.state = AppState::Settings;
        app.error = None;
        app.settings_api_key = app.api_key.clone();
        app.settings_base_url = app
            .selected_model
            .as_ref()
            .map(|m| m.base_url.clone())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        app.settings_field = 0;
        let models = App::get_available_models();
        if let Some(selected) = &app.selected_model {
            if let Some(idx) =
                models.iter().position(|m| m.name == selected.name && m.provider == selected.provider)
            {
                app.model_list_state.select(Some(idx));
            }
        }
    } else if cmd.starts_with("/baseurl") {
        app.previous_state = Some(previous_state);
        app.state = AppState::BaseUrlSelector;
        app.model_search_focused = true;
        app.search_query.clear();
        let models = App::get_available_models();
        let provider_models: Vec<&ModelOption> = models
            .iter()
            .filter(|m| m.name.starts_with("Provider:"))
            .collect();
        if let Some(ref selected) = app.selected_model {
            if let Some(idx) = provider_models
                .iter()
                .position(|m| m.base_url == selected.base_url)
            {
                app.model_list_state.select(Some(idx));
            } else {
                app.model_list_state.select(Some(0));
            }
        } else {
            app.model_list_state.select(Some(0));
        }
    } else if cmd.starts_with("/help") {
        app.previous_state = Some(previous_state);
        app.state = AppState::Help;
    } else if cmd.starts_with("/clear") {
        app.chat_messages.clear();
        app.agent = None;
        app.loading = false;
        app.error = None;
        let todo_file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy_todo.json");
        let _ = std::fs::remove_file(&todo_file);
        if !app.api_key.is_empty() {
            let _ = app.initialize_agent();
        }
    }
    app.chat_input.clear();
    app.input_cursor = 0;
    app.show_command_hints = false;
}

