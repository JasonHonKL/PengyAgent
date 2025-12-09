use crate::app::{App, AppState, ChatMessage, ModelOption};
use crate::constants::DEFAULT_BASE_URL;
use crossterm::event::KeyCode;
use std::error::Error;

fn reset_input(app: &mut App) {
    app.chat_input.clear();
    app.input_cursor = 0;
    app.show_command_hints = false;
}

fn handle_text_edit(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char(c) => {
            app.chat_input.insert(app.input_cursor, c);
            app.input_cursor += 1;
        }
        KeyCode::Backspace if app.input_cursor > 0 => {
            app.input_cursor -= 1;
            app.chat_input.remove(app.input_cursor);
        }
        KeyCode::Left if app.input_cursor > 0 => {
            app.input_cursor -= 1;
        }
        KeyCode::Right if app.input_cursor < app.chat_input.len() => {
            app.input_cursor += 1;
        }
        _ => return false,
    };

    app.show_command_hints = app.chat_input.starts_with('/');
    true
}

fn dispatch_slash_command(app: &mut App, cmd: &str, previous_state: AppState) {
    if cmd.starts_with("/new") {
        app.create_new_session();
        app.state = previous_state;
        reset_input(app);
        return;
    }

    if cmd.starts_with("/sessions") {
        app.previous_state = Some(previous_state);
        app.state = AppState::SessionSelector;
        app.session_list_state.select(Some(
            app.current_session
                .min(app.sessions.len().saturating_sub(1)),
        ));
        reset_input(app);
        return;
    }

    handle_command_inline(app, cmd, previous_state);
}

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
                dispatch_slash_command(app, &cmd, AppState::Welcome);
            } else if app.initialize_model().is_ok() {
                app.state = AppState::Chat;
                if !app.chat_input.trim().is_empty() {
                    rt.block_on(app.send_message())?;
                }
            }
        }
        other if handle_text_edit(app, other) => {}
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
        crossterm::event::KeyCode::Enter => {
            if app.chat_input.starts_with('/') {
                let cmd = app.chat_input.clone();
                dispatch_slash_command(app, &cmd, AppState::Chat);
            } else if !app.loading && !app.chat_input.trim().is_empty() {
                rt.block_on(app.send_message())?;
                app.show_command_hints = false;
            }
        }
        crossterm::event::KeyCode::PageUp => {
            scroll_chat(app, -6);
        }
        crossterm::event::KeyCode::PageDown => {
            scroll_chat(app, 6);
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
        crossterm::event::KeyCode::Tab => {
            let agents = App::get_available_agents();
            if !agents.is_empty() {
                let current = agents
                    .iter()
                    .position(|(_, _, a_type)| *a_type == app.selected_agent)
                    .unwrap_or(0);
                let next = (current + 1) % agents.len();
                app.selected_agent = agents[next].2;
                app.agent_list_state.select(Some(next));
            }
            app.user_scrolled = false;
            app.input_cursor = app.chat_input.len();
            app.show_command_hints = false;
        }
        other if handle_text_edit(app, other) => {}
        _ => {}
    }
    Ok(())
}

pub(crate) fn handle_state_key(
    app: &mut App,
    key: KeyCode,
    rt: &tokio::runtime::Runtime,
) -> Result<bool, Box<dyn Error>> {
    let should_quit = match app.state {
        AppState::Welcome => matches!(
            handle_welcome_key(app, key, rt),
            Err(e) if e.to_string() == "quit"
        ),
        AppState::Chat => matches!(
            handle_chat_key(app, key, rt),
            Err(e) if e.to_string() == "quit"
        ),
        AppState::SessionSelector => handle_session_selector_key(app, key),
        AppState::ModelSelector => handle_model_selector_key(app, key),
        AppState::ThemeSelector => handle_theme_selector_key(app, key),
        AppState::AgentSelector => handle_agent_selector_key(app, key),
        AppState::Settings => handle_settings_key(app, key),
        AppState::BaseUrlSelector => handle_baseurl_selector_key(app, key),
        AppState::CustomModel => handle_custom_model_key(app, key),
        AppState::Help => handle_help_key(app, key),
        AppState::Editor => {
            // Editor disabled for performance reasons - code kept for future use
            // match crate::editor::editor_handlers::handle_editor_key(app, key) {
            //     Ok(should_quit) => should_quit,
            //     Err(e) if e.to_string() == "quit" => true,
            //     Err(_) => false,
            // }
            // Just allow escape to go back
            if key == KeyCode::Esc {
                app.state = AppState::Welcome;
            }
            false
        }
    };

    Ok(should_quit)
}

fn handle_session_selector_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
        }
        KeyCode::Enter => {
            if let Some(idx) = app.session_list_state.selected() {
                if idx < app.sessions.len() {
                    app.load_session(idx);
                    app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                }
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let i = (app.session_list_state.selected().unwrap_or(0) + 1)
                .min(app.sessions.len().saturating_sub(1));
            app.session_list_state.select(Some(i));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let i = app
                .session_list_state
                .selected()
                .unwrap_or(0)
                .saturating_sub(1);
            app.session_list_state.select(Some(i));
        }
        _ => {}
    }
    false
}

fn filtered_models(app: &App) -> Vec<ModelOption> {
    let models = App::get_available_models();
    if app.search_query.is_empty() {
        models
            .into_iter()
            .filter(|m| !m.name.starts_with("Provider:"))
            .collect()
    } else {
        let query_lower = app.search_query.to_lowercase();
        models
            .into_iter()
            .filter(|m| {
                !m.name.starts_with("Provider:")
                    && (m.name.to_lowercase().contains(&query_lower)
                        || m.provider.to_lowercase().contains(&query_lower)
                        || m.base_url.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

fn filtered_provider_models(app: &App) -> Vec<ModelOption> {
    let mut provider_models: Vec<ModelOption> = App::get_available_models()
        .into_iter()
        .filter(|m| m.name.starts_with("Provider:"))
        .collect();
    
    // Add Custom Base URL option
    provider_models.push(ModelOption {
        name: "Provider: Custom Base URL".to_string(),
        provider: "Custom".to_string(),
        base_url: "".to_string(), // Empty means user will enter custom URL
    });

    if app.search_query.is_empty() {
        provider_models
    } else {
        let query_lower = app.search_query.to_lowercase();
        provider_models
            .into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower)
                    || m.provider.to_lowercase().contains(&query_lower)
                    || m.base_url.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

fn handle_model_selector_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            if app.model_search_focused {
                app.model_search_focused = false;
                app.search_query.clear();
            } else {
                app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
            }
        }
        KeyCode::Tab => {
            app.model_search_focused = !app.model_search_focused;
            let filtered = filtered_models(app);
            if !app.model_search_focused && !filtered.is_empty() {
                app.model_list_state.select(Some(0));
            }
        }
        KeyCode::Enter => {
            if app.model_search_focused {
                app.model_search_focused = false;
            } else if let Some(selected) = app.model_list_state.selected() {
                let filtered = filtered_models(app);
                if let Some(model) = filtered.get(selected).cloned() {
                    if model.provider == "Custom" {
                        app.previous_state = Some(AppState::ModelSelector);
                        app.reset_custom_model_fields();
                        app.state = AppState::CustomModel;
                    } else {
                        app.selected_model = Some(model);
                        if !app.api_key.is_empty() {
                            if app.initialize_model().is_ok() {
                                app.state = AppState::Chat;
                            }
                        } else {
                            app.settings_api_key = app.api_key.clone();
                            app.settings_base_url = app
                                .selected_model
                                .as_ref()
                                .map(|m| m.base_url.clone())
                                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
                            app.settings_field = 0;
                            app.error = None;
                            app.state = AppState::Settings;
                        }
                    }
                }
            }
        }
        KeyCode::Up if !app.model_search_focused => {
            let filtered = filtered_models(app);
            let i = app
                .model_list_state
                .selected()
                .unwrap_or(0)
                .saturating_sub(1);
            app.model_list_state
                .select(Some(i.min(filtered.len().saturating_sub(1))));
        }
        KeyCode::Down if !app.model_search_focused => {
            let filtered = filtered_models(app);
            let i = (app.model_list_state.selected().unwrap_or(0) + 1)
                .min(filtered.len().saturating_sub(1));
            app.model_list_state.select(Some(i));
        }
        KeyCode::Char(c) if app.model_search_focused => {
            app.search_query.push(c);
            if !filtered_models(app).is_empty() {
                app.model_list_state.select(Some(0));
            }
        }
        KeyCode::Backspace if app.model_search_focused => {
            app.search_query.pop();
            if !filtered_models(app).is_empty() {
                app.model_list_state.select(Some(0));
            }
        }
        _ => {}
    }

    false
}

fn handle_theme_selector_key(app: &mut App, key: KeyCode) -> bool {
    let filtered = app.filtered_themes();
    match key {
        KeyCode::Esc => {
            app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
            app.theme_search_focused = false;
            app.theme_search_query.clear();
        }
        KeyCode::Tab => {
            app.theme_search_focused = !app.theme_search_focused;
            if !app.theme_search_focused && !crate::theme::THEMES.is_empty() {
                app.theme_list_state.select(Some(0));
            }
        }
        KeyCode::Enter => {
            if app.theme_search_focused {
                app.theme_search_focused = false;
            } else if let Some(sel) = app.theme_list_state.selected() {
                if let Some((actual_idx, _)) = filtered.get(sel) {
                    app.theme_index = *actual_idx;
                    let _ = app.save_config();
                    app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                }
            }
        }
        KeyCode::Up => {
            if !app.theme_search_focused {
                let i = app
                    .theme_list_state
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                let new_sel = if filtered.is_empty() {
                    0
                } else {
                    i.min(filtered.len().saturating_sub(1))
                };
                app.theme_list_state.select(Some(new_sel));
                if let Some((actual_idx, _)) = filtered.get(new_sel) {
                    app.theme_index = *actual_idx;
                }
            }
        }
        KeyCode::Down => {
            if !app.theme_search_focused {
                let i = app.theme_list_state.selected().unwrap_or(0) + 1;
                let new_sel = if filtered.is_empty() {
                    0
                } else {
                    i.min(filtered.len().saturating_sub(1))
                };
                app.theme_list_state.select(Some(new_sel));
                if let Some((actual_idx, _)) = filtered.get(new_sel) {
                    app.theme_index = *actual_idx;
                }
            }
        }
        KeyCode::Char(c) if app.theme_search_focused => {
            app.theme_search_query.push(c);
            let filtered = app.filtered_themes();
            if !filtered.is_empty() {
                app.theme_list_state.select(Some(0));
                app.theme_index = filtered[0].0;
            }
        }
        KeyCode::Backspace if app.theme_search_focused => {
            app.theme_search_query.pop();
            let filtered = app.filtered_themes();
            if !filtered.is_empty() {
                app.theme_list_state.select(Some(0));
                app.theme_index = filtered[0].0;
            }
        }
        _ => {}
    }
    false
}

fn handle_agent_selector_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
        }
        KeyCode::Tab => {
            let agents = App::get_available_agents();
            let current = app.agent_list_state.selected().unwrap_or(0);
            let next = (current + 1) % agents.len();
            app.agent_list_state.select(Some(next));
            app.selected_agent = agents[next].2;
        }
        KeyCode::BackTab => {
            let agents = App::get_available_agents();
            let current = app.agent_list_state.selected().unwrap_or(0);
            let prev = if current == 0 {
                agents.len() - 1
            } else {
                current - 1
            };
            app.agent_list_state.select(Some(prev));
            app.selected_agent = agents[prev].2;
        }
        KeyCode::Enter => {
            let agents = App::get_available_agents();
            if let Some(selected) = app.agent_list_state.selected() {
                app.selected_agent = agents[selected].2;
                app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
            }
        }
        KeyCode::Up => {
            let i = app
                .agent_list_state
                .selected()
                .unwrap_or(0)
                .saturating_sub(1);
            app.agent_list_state.select(Some(i));
            let agents = App::get_available_agents();
            app.selected_agent = agents[i].2;
        }
        KeyCode::Down => {
            let agents = App::get_available_agents();
            let i = (app.agent_list_state.selected().unwrap_or(0) + 1).min(agents.len() - 1);
            app.agent_list_state.select(Some(i));
            app.selected_agent = agents[i].2;
        }
        _ => {}
    }
    false
}

fn handle_settings_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => app.state = app.previous_state.clone().unwrap_or(AppState::Welcome),
        KeyCode::Tab => {
            app.settings_field = (app.settings_field + 1) % 3;
        }
        KeyCode::BackTab => {
            app.settings_field = (app.settings_field + 2) % 3;
        }
        KeyCode::Up => {
            if app.settings_field == 2 {
                let i = app
                    .model_list_state
                    .selected()
                    .unwrap_or(0)
                    .saturating_sub(1);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Down => {
            if app.settings_field == 2 {
                let i = (app.model_list_state.selected().unwrap_or(0) + 1)
                    .min(App::get_available_models().len() - 1);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Enter => {
            app.api_key = app.settings_api_key.clone();
            let normalized_base_url = {
                let normalized = App::normalize_base_url(&app.settings_base_url);
                if normalized.is_empty() {
                    DEFAULT_BASE_URL.to_string()
                } else {
                    normalized
                }
            };
            app.settings_base_url = normalized_base_url.clone();

            let models = App::get_available_models();
            let selected_idx = app
                .model_list_state
                .selected()
                .unwrap_or(0)
                .min(models.len().saturating_sub(1));
            let mut model = models[selected_idx].clone();

            if model.provider != "Custom" {
                // Only update model's base_url if we're actually selecting a new model
                // If we're just updating settings, preserve the current selected model
                if let Some(ref current_model) = app.selected_model {
                    if current_model.name == model.name && current_model.provider == model.provider {
                        // Same model - preserve it, just update base_url for initialization
                        let mut updated_model = current_model.clone();
                        updated_model.base_url = normalized_base_url.clone();
                        app.selected_model = Some(updated_model);
                    } else {
                        // Different model selected - update it
                        model.base_url = normalized_base_url.clone();
                        app.selected_model = Some(model);
                    }
                } else {
                    // No model selected - set the new one
                    model.base_url = normalized_base_url.clone();
                    app.selected_model = Some(model);
                }
                let _ = app.save_config();
                match app.initialize_model() {
                    Ok(_) => {
                        app.error = None;
                        app.state = AppState::Chat;
                    }
                    Err(e) => {
                        app.error = Some(e.to_string());
                    }
                }
            } else {
                // Custom model - preserve custom_base_url, don't overwrite it
                if app.custom_base_url.is_empty() {
                    app.custom_base_url = normalized_base_url.clone();
                }
                app.selected_model = Some(model);
                let _ = app.save_config();
                app.state = AppState::CustomModel;
            }
        }
        KeyCode::Char(c) => match app.settings_field {
            0 => app.settings_api_key.push(c),
            1 => app.settings_base_url.push(c),
            _ => {}
        },
        KeyCode::Backspace => match app.settings_field {
            0 => {
                app.settings_api_key.pop();
            }
            1 => {
                app.settings_base_url.pop();
            }
            _ => {}
        },
        _ => {}
    }
    false
}

fn handle_baseurl_selector_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => {
            if app.model_search_focused {
                app.model_search_focused = false;
                app.search_query.clear();
            } else {
                app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
            }
        }
        KeyCode::Tab => {
            app.model_search_focused = !app.model_search_focused;
            let filtered = filtered_provider_models(app);
            if !app.model_search_focused && !filtered.is_empty() {
                app.model_list_state.select(Some(0));
            }
        }
        KeyCode::Enter => {
            let filtered = filtered_provider_models(app);
            if let Some(selected) = app
                .model_list_state
                .selected()
                .and_then(|i| filtered.get(i))
            {
                // If Custom Base URL is selected, go to settings to enter it
                if selected.name == "Provider: Custom Base URL" {
                    app.settings_api_key = app.api_key.clone();
                    // Keep current settings_base_url or use empty to prompt for input
                    if app.settings_base_url.is_empty() {
                        app.settings_base_url = DEFAULT_BASE_URL.to_string();
                    }
                    app.settings_field = 1; // Focus on base URL field
                    app.error = None;
                    app.model_search_focused = false;
                    app.search_query.clear();
                    app.state = AppState::Settings;
                } else {
                    let normalized_base = App::normalize_base_url(&selected.base_url);
                    app.settings_api_key = app.api_key.clone();
                    app.settings_base_url = normalized_base.clone();
                    app.settings_field = 1;
                    app.error = None;
                    app.model_search_focused = false;
                    app.search_query.clear();

                    // Don't change the selected model's base_url - only update settings
                    // The model will use settings_base_url when initialized

                    app.state = AppState::Settings;
                }
            }
        }
        KeyCode::Up if !app.model_search_focused => {
            let filtered = filtered_provider_models(app);
            let i = app
                .model_list_state
                .selected()
                .unwrap_or(0)
                .saturating_sub(1);
            app.model_list_state
                .select(Some(i.min(filtered.len().saturating_sub(1))));
        }
        KeyCode::Down if !app.model_search_focused => {
            let filtered = filtered_provider_models(app);
            let i = (app.model_list_state.selected().unwrap_or(0) + 1)
                .min(filtered.len().saturating_sub(1));
            app.model_list_state.select(Some(i));
        }
        KeyCode::Char(c) if app.model_search_focused => {
            app.search_query.push(c);
            if !filtered_provider_models(app).is_empty() {
                app.model_list_state.select(Some(0));
            }
        }
        KeyCode::Backspace if app.model_search_focused => {
            app.search_query.pop();
            if !filtered_provider_models(app).is_empty() {
                app.model_list_state.select(Some(0));
            }
        }
        _ => {}
    }

    false
}

fn handle_custom_model_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc => app.state = AppState::ModelSelector,
        // Only one field now; keep focus fixed.
        KeyCode::Tab => app.custom_model_field = 0,
        KeyCode::Enter => {
            if !app.custom_model_name.is_empty() {
                // Preserve the current base URL - don't change it
                // Use custom_base_url if set, otherwise use settings_base_url, otherwise default
                let normalized_base_url = {
                    let normalized = App::normalize_base_url(&app.custom_base_url);
                    if normalized.is_empty() {
                        let from_settings = App::normalize_base_url(&app.settings_base_url);
                        if from_settings.is_empty() {
                            DEFAULT_BASE_URL.to_string()
                        } else {
                            from_settings
                        }
                    } else {
                        normalized
                    }
                };

                // Don't update custom_base_url or settings_base_url - preserve them
                app.selected_model = Some(ModelOption {
                    name: app.custom_model_name.clone(),
                    provider: "Custom".to_string(),
                    base_url: normalized_base_url.clone(),
                });

                if app.api_key.is_empty() {
                    app.previous_state = Some(AppState::CustomModel);
                    app.state = AppState::Settings;
                } else {
                    match app.initialize_model() {
                        Ok(_) => {
                            // After saving a custom model, return to the Welcome screen.
                            app.state = AppState::Welcome;
                        }
                        Err(e) => app.error = Some(format!("Error initializing: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            app.custom_model_name.push(c);
        }
        KeyCode::Backspace => {
            app.custom_model_name.pop();
        }
        _ => {}
    }
    false
}

fn handle_help_key(app: &mut App, key: KeyCode) -> bool {
    if key == KeyCode::Esc {
        app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
    }
    false
}

/// Scroll the chat history using mouse wheel or other scroll events.
/// Negative `delta` scrolls up, positive scrolls down.
pub(crate) fn scroll_chat(app: &mut App, delta: i32) {
    let len = app.chat_messages.len();
    if len == 0 || delta == 0 {
        return;
    }

    app.user_scrolled = true;
    let current = app.list_state.selected().unwrap_or(len.saturating_sub(1));

    let new_index = if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        (current + delta as usize).min(len.saturating_sub(1))
    };

    app.list_state.select(Some(new_index));
    app.input_cursor = app.chat_input.len();
}

/// Mouse scroll with simple throttling to reduce sensitivity.
pub(crate) fn scroll_chat_mouse(app: &mut App, delta: i32) {
    // Throttle mouse scroll: act once every 3 ticks.
    if app.scroll_skip_ticks > 0 {
        app.scroll_skip_ticks -= 1;
        return;
    }
    app.scroll_skip_ticks = 2;
    scroll_chat(app, delta);
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
            if let Some(idx) = models
                .iter()
                .position(|m| m.name == selected.name && m.provider == selected.provider)
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
        app.session_dirty = true;
        app.save_current_session();
    } else if cmd.starts_with("/sandbox") {
        let enable = !(cmd.contains("off") || cmd.contains("disable"));
        let result = if enable {
            app.enable_sandbox_mode()
        } else {
            app.disable_sandbox_mode()
        };
        match result {
            Ok(msg) => {
                app.chat_messages.push(ChatMessage::Assistant(msg));
            }
            Err(err) => {
                app.chat_messages
                    .push(ChatMessage::Error(format!("[sandbox] {}", err)));
            }
        }
        app.session_dirty = true;
        app.save_current_session();
    } else if cmd.starts_with("/save") {
        match app.save_sandbox_changes() {
            Ok(msg) => app.chat_messages.push(ChatMessage::Assistant(msg)),
            Err(err) => app
                .chat_messages
                .push(ChatMessage::Error(format!("[sandbox] {}", err))),
        }
        app.session_dirty = true;
        app.save_current_session();
    } else if cmd.starts_with("/theme") {
        app.previous_state = Some(previous_state);
        app.state = AppState::ThemeSelector;
        let themes_len = crate::theme::THEMES.len();
        let idx = app.theme_index.min(themes_len.saturating_sub(1));
        app.theme_list_state.select(Some(idx));
        app.theme_search_query.clear();
        app.theme_search_focused = false;
    } else if cmd.starts_with("/editor") {
        // Editor disabled for performance reasons - code kept for future use
        // app.previous_state = Some(previous_state);
        // app.state = AppState::Editor;
        // app.editor_state = crate::editor::editor::EditorState::new();
        // Show error message instead
        app.chat_messages.push(crate::app::ChatMessage::Error(
            "Editor mode is currently disabled for performance optimization. The code is preserved for future use.".to_string()
        ));
    }
    app.chat_input.clear();
    app.input_cursor = 0;
    app.show_command_hints = false;
}
