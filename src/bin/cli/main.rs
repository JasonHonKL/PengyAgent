mod app;
mod command;
mod constants;
mod handlers;
mod theme;
mod theme_select;
mod ui;

use app::{App, AppState};
use command::{parse_agent_type, parse_cmd_args, run_cmd_mode};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use handlers::{handle_state_key, scroll_chat_mouse};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    error::Error,
    io::{Stdout, stdout},
    time::Duration,
};
use tokio::runtime::Runtime;
use ui::ui;

fn main() -> Result<(), Box<dyn Error>> {
    if try_run_cmd_mode()? {
        return Ok(());
    }

    let rt = Runtime::new()?;
    let mut terminal = setup_terminal()?;
    let mut app = App::new()?;

    let result = run_tui(&rt, &mut terminal, &mut app);
    cleanup_terminal(&mut terminal)?;
    result
}

fn try_run_cmd_mode() -> Result<bool, Box<dyn Error>> {
    if let Some((prompt, agent_str, model, provider, api_key, base_url)) = parse_cmd_args() {
        let rt = Runtime::new()?;
        match parse_agent_type(&agent_str) {
            Ok(agent_type) => {
                rt.block_on(run_cmd_mode(
                    prompt, agent_type, model, provider, api_key, base_url,
                ))?;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                print_cmd_usage();
                std::process::exit(1);
            }
        }
        return Ok(true);
    }

    Ok(false)
}

fn print_cmd_usage() {
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
    eprintln!("  --base-url <base-url>       Custom base URL (required for Custom provider)");
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
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        Clear(ClearType::All)
    )?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn cleanup_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_tui(
    rt: &Runtime,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    loop {
        app.process_events();
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(16))? {
            let evt = event::read()?;
            match evt {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        if app.loading {
                            app.loading = false;
                            app.error = Some("Stopped by user (Ctrl+C)".to_string());
                            continue;
                        }
                        if !app.chat_input.is_empty() {
                            app.chat_input.clear();
                            app.input_cursor = 0;
                            app.show_command_hints = false;
                            continue;
                        }
                        break;
                    }

                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('p')
                    {
                        app.state = AppState::Chat;
                        app.chat_input = "/".to_string();
                        app.input_cursor = app.chat_input.len();
                        app.show_command_hints = true;
                        app.user_scrolled = false;
                        continue;
                    }

                    let should_quit = handle_state_key(app, key.code, rt)?;

                    if should_quit {
                        break;
                    }
                }
                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::ScrollUp
                        if app.state == AppState::Chat && !app.chat_messages.is_empty() =>
                    {
                        scroll_chat_mouse(app, -1)
                    }
                    MouseEventKind::ScrollDown
                        if app.state == AppState::Chat && !app.chat_messages.is_empty() =>
                    {
                        scroll_chat_mouse(app, 1)
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(())
}
