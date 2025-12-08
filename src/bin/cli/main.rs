mod app;
mod command;
mod constants;
mod handlers;
mod ui;

use app::{App, AppState, ModelOption};
use command::{parse_agent_type, parse_cmd_args, run_cmd_mode};
use constants::DEFAULT_BASE_URL;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use handlers::{handle_chat_key, handle_welcome_key};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{error::Error, io::stdout, time::Duration};
use ui::ui;

fn main() -> Result<(), Box<dyn Error>> {
    if let Some((prompt, agent_str, model, provider, api_key, base_url)) = parse_cmd_args() {
        let rt = tokio::runtime::Runtime::new()?;
        match parse_agent_type(&agent_str) {
            Ok(agent_type) => {
                rt.block_on(run_cmd_mode(
                    prompt, agent_type, model, provider, api_key, base_url,
                ))?;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!(
                    "\nUsage: pengy --prompt \"<prompt>\" --agent <agent-type> --model <model-name> --provider <provider> --api-key <api-key> [--base-url <base-url>]"
                );
                eprintln!("\nRequired arguments:");
                eprintln!("  --prompt \"<prompt>\"        The prompt/question for the agent");
                eprintln!("  --agent <agent-type>        The agent type to use");
                eprintln!("  --model <model-name>        The model name (e.g., openai/gpt-4o)");
                eprintln!("  --provider <provider>       The provider name (e.g., OpenAI, Custom)");
                eprintln!("  --api-key <api-key>         Your API key");
                eprintln!("\nOptional arguments:");
                eprintln!(
                    "  --base-url <base-url>       Custom base URL (required for Custom provider)"
                );
                eprintln!("\nAvailable agent types:");
                eprintln!("  - coder");
                eprintln!("  - code-researcher");
                eprintln!("  - test-agent");
                eprintln!("  - pengy-agent");
                eprintln!("  - control-agent");
                eprintln!("  - issue-agent");
                eprintln!("\nExample:");
                eprintln!(
                    "  pengy --prompt \"Write a hello world function\" --agent coder --model openai/gpt-4o --provider OpenAI --api-key sk-..."
                );
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()?;
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    loop {
        app.process_events();
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(16))? {
            let evt = event::read()?;
            if let event::Event::Key(key) = evt {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                let should_quit = match app.state {
                    AppState::Welcome => matches!(
                        handle_welcome_key(&mut app, key.code, &rt),
                        Err(e) if e.to_string() == "quit"
                    ),
                    AppState::Chat => matches!(
                        handle_chat_key(&mut app, key.code, &rt),
                        Err(e) if e.to_string() == "quit"
                    ),
                    AppState::SessionSelector => {
                        match key.code {
                            KeyCode::Esc => {
                                app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                            }
                            KeyCode::Enter => {
                                if let Some(idx) = app.session_list_state.selected() {
                                    if idx < app.sessions.len() {
                                        app.current_session = idx;
                                        app.state =
                                            app.previous_state.clone().unwrap_or(AppState::Welcome);
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
                    AppState::ModelSelector => {
                        match key.code {
                            KeyCode::Esc => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                    app.search_query.clear();
                                } else {
                                    app.state =
                                        app.previous_state.clone().unwrap_or(AppState::Welcome);
                                }
                            }
                            KeyCode::Tab => {
                                app.model_search_focused = !app.model_search_focused;
                                if !app.model_search_focused {
                                    let all_models = App::get_available_models();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty()
                                    {
                                        all_models
                                            .iter()
                                            .filter(|m| !m.name.starts_with("Provider:"))
                                            .collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        all_models
                                            .iter()
                                            .filter(|m| {
                                                !m.name.starts_with("Provider:")
                                                    && (m
                                                        .name
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                        || m.provider
                                                            .to_lowercase()
                                                            .contains(&query_lower)
                                                        || m.base_url
                                                            .to_lowercase()
                                                            .contains(&query_lower))
                                            })
                                            .collect()
                                    };
                                    if !filtered.is_empty() {
                                        app.model_list_state.select(Some(0));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                } else {
                                    let models = App::get_available_models();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty()
                                    {
                                        models
                                            .iter()
                                            .filter(|m| !m.name.starts_with("Provider:"))
                                            .collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        models
                                            .iter()
                                            .filter(|m| {
                                                !m.name.starts_with("Provider:")
                                                    && (m
                                                        .name
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                        || m.provider
                                                            .to_lowercase()
                                                            .contains(&query_lower)
                                                        || m.base_url
                                                            .to_lowercase()
                                                            .contains(&query_lower))
                                            })
                                            .collect()
                                    };

                                    if let Some(selected) = app.model_list_state.selected() {
                                        if selected < filtered.len() {
                                            let model = filtered[selected].clone();
                                            if model.provider == "Custom" {
                                                app.custom_model_field = 0;
                                                app.custom_model_name.clear();
                                                app.custom_base_url.clear();
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
                                                        .unwrap_or_else(|| {
                                                            DEFAULT_BASE_URL.to_string()
                                                        });
                                                    app.settings_field = 0;
                                                    app.error = None;
                                                    app.state = AppState::Settings;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Up if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    all_models
                                        .iter()
                                        .filter(|m| !m.name.starts_with("Provider:"))
                                        .collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models
                                        .iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:")
                                                && (m.name.to_lowercase().contains(&query_lower)
                                                    || m.provider
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                    || m.base_url
                                                        .to_lowercase()
                                                        .contains(&query_lower))
                                        })
                                        .collect()
                                };
                                let i = app
                                    .model_list_state
                                    .selected()
                                    .unwrap_or(0)
                                    .saturating_sub(1);
                                app.model_list_state
                                    .select(Some(i.min(filtered.len().saturating_sub(1))));
                            }
                            KeyCode::Down if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    all_models
                                        .iter()
                                        .filter(|m| !m.name.starts_with("Provider:"))
                                        .collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models
                                        .iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:")
                                                && (m.name.to_lowercase().contains(&query_lower)
                                                    || m.provider
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                    || m.base_url
                                                        .to_lowercase()
                                                        .contains(&query_lower))
                                        })
                                        .collect()
                                };
                                let i = (app.model_list_state.selected().unwrap_or(0) + 1)
                                    .min(filtered.len().saturating_sub(1));
                                app.model_list_state.select(Some(i));
                            }
                            KeyCode::Char(c) if app.model_search_focused => {
                                app.search_query.push(c);
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models
                                        .iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:")
                                                && (m.name.to_lowercase().contains(&query_lower)
                                                    || m.provider
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                    || m.base_url
                                                        .to_lowercase()
                                                        .contains(&query_lower))
                                        })
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            }
                            KeyCode::Backspace if app.model_search_focused => {
                                app.search_query.pop();
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    all_models
                                        .iter()
                                        .filter(|m| !m.name.starts_with("Provider:"))
                                        .collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models
                                        .iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:")
                                                && (m.name.to_lowercase().contains(&query_lower)
                                                    || m.provider
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                    || m.base_url
                                                        .to_lowercase()
                                                        .contains(&query_lower))
                                        })
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            }
                            _ => {}
                        }
                        false
                    }
                    AppState::AgentSelector => {
                        match key.code {
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
                                    app.state =
                                        app.previous_state.clone().unwrap_or(AppState::Welcome);
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
                                let i = (app.agent_list_state.selected().unwrap_or(0) + 1)
                                    .min(agents.len() - 1);
                                app.agent_list_state.select(Some(i));
                                app.selected_agent = agents[i].2;
                            }
                            _ => {}
                        }
                        false
                    }
                    AppState::Settings => {
                        match key.code {
                            KeyCode::Esc => {
                                app.state = app.previous_state.clone().unwrap_or(AppState::Welcome)
                            }
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
                                    let normalized =
                                        App::normalize_base_url(&app.settings_base_url);
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
                                    model.base_url = normalized_base_url.clone();
                                    app.selected_model = Some(model);
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
                                    app.custom_base_url = normalized_base_url.clone();
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
                    AppState::BaseUrlSelector => {
                        match key.code {
                            KeyCode::Esc => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                    app.search_query.clear();
                                } else {
                                    app.state =
                                        app.previous_state.clone().unwrap_or(AppState::Welcome);
                                }
                            }
                            KeyCode::Tab => {
                                app.model_search_focused = !app.model_search_focused;
                                if !app.model_search_focused {
                                    let all_models = App::get_available_models();
                                    let provider_models: Vec<&ModelOption> = all_models
                                        .iter()
                                        .filter(|m| m.name.starts_with("Provider:"))
                                        .collect();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty()
                                    {
                                        provider_models.iter().copied().collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        provider_models
                                            .iter()
                                            .filter(|m| {
                                                m.name.to_lowercase().contains(&query_lower)
                                                    || m.provider
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                    || m.base_url
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                            })
                                            .copied()
                                            .collect()
                                    };
                                    if !filtered.is_empty() {
                                        app.model_list_state.select(Some(0));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                } else {
                                    let all_models = App::get_available_models();
                                    let provider_models: Vec<&ModelOption> = all_models
                                        .iter()
                                        .filter(|m| m.name.starts_with("Provider:"))
                                        .collect();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty()
                                    {
                                        provider_models.iter().copied().collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        provider_models
                                            .iter()
                                            .filter(|m| {
                                                m.name.to_lowercase().contains(&query_lower)
                                                    || m.provider
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                                    || m.base_url
                                                        .to_lowercase()
                                                        .contains(&query_lower)
                                            })
                                            .copied()
                                            .collect()
                                    };

                                    if let Some(selected) = app.model_list_state.selected() {
                                        if selected < filtered.len() {
                                            let provider = filtered[selected];
                                            app.settings_api_key = app.api_key.clone();
                                            app.settings_base_url = provider.base_url.clone();
                                            app.settings_field = 1;
                                            app.error = None;

                                            if let Some(ref mut selected) = app.selected_model {
                                                selected.base_url = provider.base_url.clone();
                                            }

                                            app.state = AppState::Settings;
                                        }
                                    }
                                }
                            }
                            KeyCode::Up if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models
                                    .iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    provider_models.iter().copied().collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models
                                        .iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower)
                                                || m.provider.to_lowercase().contains(&query_lower)
                                                || m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                let i = app
                                    .model_list_state
                                    .selected()
                                    .unwrap_or(0)
                                    .saturating_sub(1);
                                app.model_list_state
                                    .select(Some(i.min(filtered.len().saturating_sub(1))));
                            }
                            KeyCode::Down if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models
                                    .iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    provider_models.iter().copied().collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models
                                        .iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower)
                                                || m.provider.to_lowercase().contains(&query_lower)
                                                || m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                let i = (app.model_list_state.selected().unwrap_or(0) + 1)
                                    .min(filtered.len().saturating_sub(1));
                                app.model_list_state.select(Some(i));
                            }
                            KeyCode::Char(c) if app.model_search_focused => {
                                app.search_query.push(c);
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models
                                    .iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models
                                        .iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower)
                                                || m.provider.to_lowercase().contains(&query_lower)
                                                || m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            }
                            KeyCode::Backspace if app.model_search_focused => {
                                app.search_query.pop();
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models
                                    .iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    provider_models.iter().copied().collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models
                                        .iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower)
                                                || m.provider.to_lowercase().contains(&query_lower)
                                                || m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            }
                            _ => {}
                        }
                        false
                    }
                    AppState::CustomModel => {
                        match key.code {
                            KeyCode::Esc => app.state = AppState::ModelSelector,
                            KeyCode::Tab => {
                                app.custom_model_field = (app.custom_model_field + 1) % 2
                            }
                            KeyCode::Enter => {
                                if !app.custom_model_name.is_empty()
                                    && !app.custom_base_url.is_empty()
                                {
                                    let normalized_base_url =
                                        App::normalize_base_url(&app.custom_base_url);
                                    app.custom_base_url = normalized_base_url.clone();
                                    app.selected_model = Some(ModelOption {
                                        name: app.custom_model_name.clone(),
                                        provider: "Custom".to_string(),
                                        base_url: normalized_base_url,
                                    });

                                    if app.api_key.is_empty() {
                                        app.previous_state = Some(AppState::CustomModel);
                                        app.state = AppState::Settings;
                                    } else {
                                        match app.initialize_model() {
                                            Ok(_) => app.state = AppState::Chat,
                                            Err(e) => {
                                                app.error =
                                                    Some(format!("Error initializing: {}", e))
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if app.custom_model_field == 0 {
                                    app.custom_model_name.push(c);
                                } else {
                                    app.custom_base_url.push(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if app.custom_model_field == 0 {
                                    app.custom_model_name.pop();
                                } else {
                                    app.custom_base_url.pop();
                                }
                            }
                            _ => {}
                        }
                        false
                    }
                    AppState::Help => {
                        if key.code == KeyCode::Esc {
                            app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                        }
                        false
                    }
                };

                if should_quit {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
