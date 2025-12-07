use pengy_agent::model::model::model::Model;
use pengy_agent::agent::agent::agent::{Agent, AgentEvent};
use pengy_agent::agent::coder::coder::create_coder_agent;
use pengy_agent::agent::code_researcher::code_researcher::create_code_researcher_agent;
use pengy_agent::agent::test_agent::test_agent::create_test_agent;
use pengy_agent::agent::pengy_agent::pengy_agent::run_pengy_agent;
use pengy_agent::agent::control_agent::control_agent::create_control_agent;
use pengy_agent::agent::issue_agent::issue_agent::create_issue_agent;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    error::Error,
    io::stdout,
    path::PathBuf,
};
use tokio::sync::mpsc;

const VERSION: &str = "v0.1.0";
const CONFIG_FILE: &str = ".pengy_config.json";
const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Clone, PartialEq, Debug)]
enum AppState {
    Welcome,
    Chat,
    ModelSelector,
    Settings,
    Help,
    CustomModel,
    AgentSelector,
}

#[derive(Clone, PartialEq, Debug, Copy)]
enum AgentType {
    Coder,          // Coding agent
    CodeResearcher, // Code research agent
    TestAgent,      // Testing agent
    PengyAgent,     // Meta-agent (orchestrates all three)
    ControlAgent,   // Git and GitHub control agent
    IssueAgent,     // Issue finder and reporter
}

#[derive(Clone)]
enum ChatMessage {
    User(String),
    Assistant(String),
    ToolCall {
        id: String,
        name: String,
        args: String,
        result: Option<String>,
        status: ToolStatus,
    },
    Thinking(String),
    Step { step: u32, max: u32 },
    Error(String),
}

#[derive(Clone, PartialEq)]
enum ToolStatus {
    Running,
    Success,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
struct ModelOption {
    name: String,
    provider: String,
    base_url: String,
}

#[derive(Serialize, Deserialize)]
struct Config {
    api_key: String,
    selected_model: Option<ModelOption>,
}

struct App {
    state: AppState,
    api_key: String,
    selected_model: Option<ModelOption>,
    model: Option<Model>,
    agent: Option<Agent>,
    selected_agent: AgentType,
    chat_messages: Vec<ChatMessage>,
    logo: String,
    list_state: ListState,
    scroll_state: ScrollbarState,
    chat_input: String,
    input_cursor: usize,
    loading: bool,
    error: Option<String>,
    model_list_state: ListState,
    agent_list_state: ListState,
    settings_api_key: String,
    settings_base_url: String,
    settings_field: usize,
    search_query: String,
    show_command_hints: bool,
    custom_model_name: String,
    custom_base_url: String,
    custom_model_field: usize,
    previous_state: Option<AppState>,
    rx: mpsc::UnboundedReceiver<AgentEvent>,
    tx: mpsc::UnboundedSender<AgentEvent>,
    agent_rx: mpsc::UnboundedReceiver<Agent>,
    agent_tx: mpsc::UnboundedSender<Agent>,
}

impl App {
    fn load_logo() -> String {
        // Try executable directory first (common when installed)
        if let Ok(mut exe) = env::current_exe() {
            exe.pop();
            let candidate = exe.join("logo.txt");
            if candidate.exists() {
                if let Ok(content) = std::fs::read_to_string(&candidate) {
                    return content;
                }
            }
        }

        // Fallback: current working directory
        let cwd_logo = PathBuf::from("logo.txt");
        if cwd_logo.exists() {
            if let Ok(content) = std::fs::read_to_string(&cwd_logo) {
                return content;
            }
        }

        // Final fallback: built-in minimal logo
        "Pengy Agent".to_string()
    }

    fn config_path() -> PathBuf {
        let cwd_config = PathBuf::from(CONFIG_FILE);
        if cwd_config.exists() {
            return cwd_config;
        }

        if let Ok(home) = env::var("HOME") {
            let home_config = PathBuf::from(home).join(CONFIG_FILE);
            if home_config.exists() {
                return home_config;
            }
        }

        // Default to current working directory if no config exists
        cwd_config
    }

    fn new() -> Result<Self, Box<dyn Error>> {
        let logo = Self::load_logo();

        let config = Self::load_config();
        let api_key = config.api_key;
        let selected_model = config.selected_model.map(|mut m| {
            m.base_url = App::normalize_base_url(&m.base_url);
            m
        });

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut model_list_state = ListState::default();
        model_list_state.select(Some(0));

        let mut agent_list_state = ListState::default();
        agent_list_state.select(Some(0));

        let (tx, rx) = mpsc::unbounded_channel();
        let (agent_tx, agent_rx) = mpsc::unbounded_channel();

        let (custom_model_name, custom_base_url) = if let Some(ref m) = selected_model {
            if m.provider == "Custom" {
                (m.name.clone(), m.base_url.clone())
            } else {
                (String::new(), DEFAULT_BASE_URL.to_string())
            }
        } else {
            (String::new(), DEFAULT_BASE_URL.to_string())
        };

        let settings_base_url = selected_model
            .as_ref()
            .map(|m| m.base_url.clone())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        Ok(Self {
            state: AppState::Welcome,
            api_key: api_key.clone(),
            selected_model,
            model: None,
            agent: None,
            // selected_agent: AgentType::Coder, // Default
            selected_agent: AgentType::Coder,
            chat_messages: Vec::new(),
            logo,
            list_state,
            scroll_state: ScrollbarState::default(),
            chat_input: String::new(),
            input_cursor: 0,
            loading: false,
            error: None,
            model_list_state,
            agent_list_state,
            settings_api_key: api_key,
            settings_base_url,
            settings_field: 0,
            search_query: String::new(),
            show_command_hints: false,
            custom_model_name,
            custom_base_url,
            custom_model_field: 0,
            previous_state: None,
            rx,
            tx,
            agent_rx,
            agent_tx,
        })
    }

    fn load_config() -> Config {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    return config;
                }
            }
        }
        let api_key = std::env::var("API_KEY").unwrap_or_default();
        Config {
            api_key,
            selected_model: None,
        }
    }

    fn save_config(&self) -> Result<(), Box<dyn Error>> {
        let config = Config {
            api_key: self.api_key.clone(),
            selected_model: self.selected_model.clone(),
        };
        let config_json = serde_json::to_string_pretty(&config)?;
        let config_path = Self::config_path();
        std::fs::write(config_path, config_json)?;
        Ok(())
    }

    fn get_available_models() -> Vec<ModelOption> {
        vec![
            // Latest OpenAI Models (2025)
            ModelOption {
                name: "openai/gpt-5.1".to_string(),
                provider: "OpenAI".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "openai/polaris-alpha".to_string(),
                provider: "OpenAI".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "openai/gpt-4o".to_string(),
                provider: "OpenAI".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "openai/gpt-4o-mini".to_string(),
                provider: "OpenAI".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest Anthropic Models (2025)
            ModelOption {
                name: "anthropic/claude-sonnet-4.5".to_string(),
                provider: "Anthropic".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "anthropic/claude-opus-4.5".to_string(),
                provider: "Anthropic".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "anthropic/claude-3.5-sonnet".to_string(),
                provider: "Anthropic".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "anthropic/claude-3.5-haiku".to_string(),
                provider: "Anthropic".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest Google Models (2025)
            ModelOption {
                name: "google/gemini-3-pro".to_string(),
                provider: "Google".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "google/gemini-2.5-flash-exp:free".to_string(),
                provider: "Google".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "google/gemini-2.5-flash".to_string(),
                provider: "Google".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "google/gemini-2.0-flash-exp:free".to_string(),
                provider: "Google".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest xAI Models (2025)
            ModelOption {
                name: "x-ai/grok-4".to_string(),
                provider: "xAI".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "x-ai/grok-code-fast-1".to_string(),
                provider: "xAI".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest Mistral Models (2025)
            ModelOption {
                name: "mistralai/mistral-small-3.2-24b-instruct".to_string(),
                provider: "Mistral".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "mistralai/devstral-small-2507".to_string(),
                provider: "Mistral".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "mistralai/devstral-medium-2507".to_string(),
                provider: "Mistral".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "mistralai/mistral-large-latest".to_string(),
                provider: "Mistral".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest DeepSeek Models (2025)
            ModelOption {
                name: "deepseek/deepseek-v3.2".to_string(),
                provider: "DeepSeek".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "deepseek/deepseek-r1t-chimera".to_string(),
                provider: "DeepSeek".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "deepseek/deepseek-r1-distill-llama-70b".to_string(),
                provider: "DeepSeek".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "deepseek/deepseek-coder-v2-instruct".to_string(),
                provider: "DeepSeek".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest Meta Models (2025)
            ModelOption {
                name: "meta-llama/llama-4-maverick".to_string(),
                provider: "Meta".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "meta-llama/llama-4-scout:free".to_string(),
                provider: "Meta".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "meta-llama/llama-3.2-90b-vision-instruct".to_string(),
                provider: "Meta".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "meta-llama/llama-3.1-70b-instruct".to_string(),
                provider: "Meta".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Latest Qwen Models
            ModelOption {
                name: "qwen/qwen2.5-vl-32b-instruct".to_string(),
                provider: "Qwen".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            ModelOption {
                name: "qwen/qwen2.5-coder-32b-instruct".to_string(),
                provider: "Qwen".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // GLM Models
            ModelOption {
                name: "z-ai/glm-4.6".to_string(),
                provider: "GLM".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
            // Custom Model
            ModelOption {
                name: "Custom Model".to_string(),
                provider: "Custom".to_string(),
                base_url: "".to_string(),
            },
        ]
    }

    fn get_command_hints(&self) -> Vec<(&str, &str)> {
        vec![
            ("/models", "select model"),
            ("/agents", "select agent"),
            ("/settings", "configure API key / model / base URL"),
            ("/help", "show help"),
            ("/clear", "clear conversation and reset agent"),
        ]
    }

    fn get_available_agents() -> Vec<(&'static str, &'static str, AgentType)> {
        vec![
            ("Coder Agent", "Coding agent with tools (bash, edit, grep, todo, web)", AgentType::Coder),
            ("Code Researcher", "Research codebase with vector search", AgentType::CodeResearcher),
            ("Test Agent", "Testing agent for code validation", AgentType::TestAgent),
            ("Pengy Agent", "Meta-agent (orchestrates all three agents)", AgentType::PengyAgent),
            ("Control Agent", "Git and GitHub control agent (read diff, commit, list issues, create PR)", AgentType::ControlAgent),
            ("Issue Agent", "Find and publish GitHub issues with cleanup workflow", AgentType::IssueAgent),
        ]
    }

    fn normalize_base_url(base_url: &str) -> String {
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let mut normalized = trimmed.trim_end_matches('/').to_string();

        for suffix in ["/chat/completions", "/completions", "/completion"] {
            if normalized.ends_with(suffix) {
                normalized = normalized
                    .trim_end_matches('/')
                    .trim_end_matches(suffix)
                    .trim_end_matches('/')
                    .to_string();
                break;
            }
        }

        normalized
    }

    fn initialize_agent(&mut self) -> Result<(), Box<dyn Error>> {
        if self.api_key.is_empty() {
            return Err("API key is required".into());
        }

        let model_option = self.selected_model.clone().ok_or("Model not selected")?;
        let normalized_base_url = App::normalize_base_url(&model_option.base_url);
        if let Some(selected) = self.selected_model.as_mut() {
            selected.base_url = normalized_base_url.clone();
        }
        let model = Model::new(
            model_option.name.clone(),
            self.api_key.clone(),
            normalized_base_url.clone(),
        );

        match self.selected_agent {
            AgentType::PengyAgent => {
                self.model = Some(model);
                self.agent = None;
            }
            AgentType::Coder => {
                let agent = create_coder_agent(
                    model,
                    None, // Use default system prompt
                    Some(3),
                    Some(20),
                );
                self.agent = Some(agent);
            }
            AgentType::CodeResearcher => {
                let agent = create_code_researcher_agent(
                    model,
                    self.api_key.clone(),
                    normalized_base_url.clone(),
                    Some("openai/text-embedding-3-small".to_string()),
                    None,
                    Some(3),
                    Some(20),
                );
                self.agent = Some(agent);
            }
            AgentType::TestAgent => {
                let agent = create_test_agent(
                    model,
                    None,
                    Some(3),
                    Some(20),
                );
                self.agent = Some(agent);
            }
            AgentType::ControlAgent => {
                let agent = create_control_agent(
                    model,
                    None,
                    Some(3),
                    Some(20),
                );
                self.agent = Some(agent);
            }
            AgentType::IssueAgent => {
                let agent = create_issue_agent(
                    model,
                    None,
                    Some(3),
                    Some(20),
                );
                self.agent = Some(agent);
            }
        }
        Ok(())
    }

    fn initialize_model(&mut self) -> Result<(), Box<dyn Error>> {
        if self.api_key.is_empty() {
            return Err("API key is required. Use /settings to configure.".into());
        }
        let _model_option = self.selected_model.as_ref().ok_or("Model not selected. Use /models to select a model.")?;
        self.initialize_agent()?;
        let _ = self.save_config();
        self.state = AppState::Chat;
        Ok(())
    }

    async fn send_message(&mut self) -> Result<(), Box<dyn Error>> {
        if self.chat_input.trim().is_empty() {
            return Ok(());
        }

        let user_input = self.chat_input.clone();
        self.chat_input.clear();
        self.input_cursor = 0;

        self.chat_messages.push(ChatMessage::User(user_input.clone()));
        self.loading = true;
        self.error = None;

        let tx = self.tx.clone();
        
        // Prepare for async execution
        let model_option = self.selected_model.clone();
        let api_key = self.api_key.clone();
        
        // We need to handle mutable access to agent/model carefully
        // Since we can't move self into async block easily with shared state
        // We'll use the event system entirely for updates
        
        match self.selected_agent {
            AgentType::PengyAgent => {
                let model = self.model.clone().ok_or("Model not initialized")?;
                let base_url = model_option
                    .as_ref()
                    .map(|m| App::normalize_base_url(&m.base_url))
                    .unwrap_or_default();
                
                tokio::spawn(async move {
                    let callback_tx = tx.clone();
                    let callback = move |event: AgentEvent| {
                        let _ = callback_tx.send(event);
                    };
                    
                    let _ = run_pengy_agent(
                        model,
                        api_key,
                        base_url,
                        Some("openai/text-embedding-3-small".to_string()),
                        user_input,
                        Some(3),
                        Some(20),
                        callback
                    ).await;
                });
            }
            _ => {
                // Other agents
                let agent_tx = self.agent_tx.clone();
                if let Some(agent) = self.agent.take() {
                    let mut agent_to_run = agent; // Move it out
                    
                    tokio::spawn(async move {
                        let callback_tx = tx.clone();
                        let callback = move |event: AgentEvent| {
                            let _ = callback_tx.send(event);
                        };
                        
                        agent_to_run.run(user_input, callback).await;
                        
                        // Return the agent after completion
                        let _ = agent_tx.send(agent_to_run);
                    });
                } else {
                    // Re-init agent if missing
                    self.initialize_agent()?;
                    if let Some(mut agent) = self.agent.take() {
                         tokio::spawn(async move {
                            let callback_tx = tx.clone();
                            let callback = move |event: AgentEvent| {
                                let _ = callback_tx.send(event);
                            };
                            agent.run(user_input, callback).await;
                            
                            // Return the agent after completion
                            let _ = agent_tx.send(agent);
                        });
                    }
                }
            }
        }

        if !self.chat_messages.is_empty() {
            self.list_state.select(Some(self.chat_messages.len() - 1));
        }
        Ok(())
    }
    
    fn process_events(&mut self) {
        // First, restore agent if available
        while let Ok(agent) = self.agent_rx.try_recv() {
            self.agent = Some(agent);
        }
        
        // Then process agent events
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AgentEvent::Step { step, max_steps } => {
                    self.chat_messages.push(ChatMessage::Step { step, max: max_steps });
                }
                AgentEvent::ToolCall { tool_name, args } => {
                    self.chat_messages.push(ChatMessage::ToolCall {
                        id: format!("tool_{}", self.chat_messages.len()),
                        name: tool_name,
                        args,
                        result: None,
                        status: ToolStatus::Running,
                    });
                }
                AgentEvent::ToolResult { result } => {
                    // Find the last running tool call and update it
                    if let Some(ChatMessage::ToolCall { result: r, status, .. }) = self.chat_messages.iter_mut().rev().find(|m| matches!(m, ChatMessage::ToolCall { status: ToolStatus::Running, .. })) {
                        *r = Some(result);
                        *status = ToolStatus::Success;
                    }
                }
                AgentEvent::Thinking { content } => {
                    self.chat_messages.push(ChatMessage::Thinking(content));
                }
                AgentEvent::FinalResponse { content } => {
                    self.chat_messages.push(ChatMessage::Assistant(content));
                    self.loading = false;
                }
                AgentEvent::Error { error } => {
                    self.chat_messages.push(ChatMessage::Error(error.clone()));
                    // Also update tool status if running
                    if let Some(ChatMessage::ToolCall { status, .. }) = self.chat_messages.iter_mut().rev().find(|m| matches!(m, ChatMessage::ToolCall { status: ToolStatus::Running, .. })) {
                        *status = ToolStatus::Error;
                    }
                    self.loading = false; // Stop loading on error
                }
                AgentEvent::VisionAnalysis { status } => {
                    self.chat_messages.push(ChatMessage::Thinking(format!("👁 {}", status)));
                }
            }
            // Auto scroll
            self.list_state.select(Some(self.chat_messages.len().saturating_sub(1)));
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
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

        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            let evt = crossterm::event::read()?;
            if let crossterm::event::Event::Key(key) = evt {
                if key.kind != crossterm::event::KeyEventKind::Press { continue; }
                
                // Global exit on Ctrl+C
                if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) && key.code == crossterm::event::KeyCode::Char('c') {
                    break;
                }

                match app.state {
                    AppState::Welcome => {
                        match key.code {
                            crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc => break,
                            crossterm::event::KeyCode::Enter => {
                                if app.chat_input.starts_with("/models") {
                                    app.previous_state = Some(AppState::Welcome);
                                    app.state = AppState::ModelSelector;
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                                    app.show_command_hints = false;
                                } else if app.chat_input.starts_with("/agents") {
                                    app.previous_state = Some(AppState::Welcome);
                                    app.state = AppState::AgentSelector;
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                                    app.show_command_hints = false;
                                } else if app.chat_input.starts_with("/settings") {
                                    app.previous_state = Some(AppState::Welcome);
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
                                        if let Some(idx) = models.iter().position(|m| m.name == selected.name) {
                                            app.model_list_state.select(Some(idx));
                                        }
                                    }
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                                    app.show_command_hints = false;
                                } else if app.chat_input.starts_with("/help") {
                                    app.previous_state = Some(AppState::Welcome);
                                    app.state = AppState::Help;
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                                    app.show_command_hints = false;
                                } else if app.chat_input.starts_with("/clear") {
                                    // Clear conversation and reset agent
                                    app.chat_messages.clear();
                                    app.agent = None;
                                    app.loading = false;
                                    app.error = None;
                                    if !app.api_key.is_empty() {
                                        let _ = app.initialize_agent();
                                    }
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                                    app.show_command_hints = false;
                                } else {
                                    // Default action: Enter chat
                                    if app.initialize_model().is_ok() {
                                        app.state = AppState::Chat;
                                        if !app.chat_input.trim().is_empty() {
                                            rt.block_on(app.send_message())?;
                                        }
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
                            crossterm::event::KeyCode::Left => if app.input_cursor > 0 { app.input_cursor -= 1; },
                            crossterm::event::KeyCode::Right => if app.input_cursor < app.chat_input.len() { app.input_cursor += 1; },
                            _ => {}
                        }
                    }
                    AppState::Chat => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => break,
                            crossterm::event::KeyCode::Enter if !app.loading => {
                                if app.chat_input.starts_with('/') {
                                    // Command handling similar to Welcome
                                    if app.chat_input.starts_with("/models") {
                                        app.previous_state = Some(AppState::Chat);
                                        app.state = AppState::ModelSelector;
                                    } else if app.chat_input.starts_with("/agents") {
                                        app.previous_state = Some(AppState::Chat);
                                        app.state = AppState::AgentSelector;
                                    } else if app.chat_input.starts_with("/settings") {
                                        app.previous_state = Some(AppState::Chat);
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
                                            if let Some(idx) = models.iter().position(|m| m.name == selected.name) {
                                                app.model_list_state.select(Some(idx));
                                            }
                                        }
                                    } else if app.chat_input.starts_with("/help") {
                                        app.previous_state = Some(AppState::Chat);
                                        app.state = AppState::Help;
                                    } else if app.chat_input.starts_with("/clear") {
                                        // Clear conversation and reset agent
                                        app.chat_messages.clear();
                                        app.agent = None;
                                        app.loading = false;
                                        app.error = None;
                                        if !app.api_key.is_empty() {
                                            let _ = app.initialize_agent();
                                        }
                                    }
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                                    app.show_command_hints = false;
                                } else if !app.chat_input.trim().is_empty() {
                                    rt.block_on(app.send_message())?;
                                    app.show_command_hints = false;
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
                            crossterm::event::KeyCode::Left => if app.input_cursor > 0 { app.input_cursor -= 1; },
                            crossterm::event::KeyCode::Right => if app.input_cursor < app.chat_input.len() { app.input_cursor += 1; },
                            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Down | crossterm::event::KeyCode::PageUp | crossterm::event::KeyCode::PageDown => {
                                let len = app.chat_messages.len();
                                if len > 0 {
                                    let selected = app.list_state.selected().unwrap_or(0);
                                    let amount = match key.code {
                                        crossterm::event::KeyCode::PageUp | crossterm::event::KeyCode::PageDown => 10,
                                        _ => 1,
                                    };
                                    let new_selection = match key.code {
                                        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::PageUp => selected.saturating_sub(amount),
                                        _ => (selected + amount).min(len.saturating_sub(1)),
                                    };
                                    app.list_state.select(Some(new_selection));
                                }
                            }
                            _ => {}
                        }
                    }
                    // ... Other states (ModelSelector, Settings, etc.) use similar logic as before ...
                    // Copying existing logic for brevity, assuming it works.
                    AppState::ModelSelector => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => app.state = app.previous_state.clone().unwrap_or(AppState::Welcome),
                            crossterm::event::KeyCode::Enter => {
                                let models = App::get_available_models();
                                if let Some(selected) = app.model_list_state.selected() {
                                    if selected < models.len() {
                                        let model = models[selected].clone();
                                        if model.provider == "Custom" {
                                            app.custom_model_field = 0;
                                            app.custom_model_name.clear();
                                            app.state = AppState::CustomModel;
                                        } else {
                                            app.selected_model = Some(model);
                                            if !app.api_key.is_empty() {
                                                if app.initialize_model().is_ok() {
                                                    app.state = AppState::Chat;
                                                }
                                            } else {
                                                app.settings_api_key = app.api_key.clone();
                                                app.settings_base_url = app.selected_model.as_ref().map(|m| m.base_url.clone()).unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
                                                app.settings_field = 0;
                                                app.error = None;
                                                app.state = AppState::Settings;
                                            }
                                        }
                                    }
                                }
                            },
                            crossterm::event::KeyCode::Up => {
                                let i = app.model_list_state.selected().unwrap_or(0).saturating_sub(1);
                                app.model_list_state.select(Some(i));
                            },
                            crossterm::event::KeyCode::Down => {
                                let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(App::get_available_models().len() - 1);
                                app.model_list_state.select(Some(i));
                            },
                            _ => {}
                        }
                    },
                    AppState::AgentSelector => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => app.state = app.previous_state.clone().unwrap_or(AppState::Welcome),
                            crossterm::event::KeyCode::Enter => {
                                let agents = App::get_available_agents();
                                if let Some(selected) = app.agent_list_state.selected() {
                                    app.selected_agent = agents[selected].2;
                                    app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                                }
                            },
                            crossterm::event::KeyCode::Up => {
                                let i = app.agent_list_state.selected().unwrap_or(0).saturating_sub(1);
                                app.agent_list_state.select(Some(i));
                            },
                            crossterm::event::KeyCode::Down => {
                                let i = (app.agent_list_state.selected().unwrap_or(0) + 1).min(App::get_available_agents().len() - 1);
                                app.agent_list_state.select(Some(i));
                            },
                            _ => {}
                        }
                    },
                    AppState::Settings => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => app.state = app.previous_state.clone().unwrap_or(AppState::Welcome),
                            crossterm::event::KeyCode::Tab => {
                                app.settings_field = (app.settings_field + 1) % 3;
                            }
                            crossterm::event::KeyCode::BackTab => {
                                app.settings_field = (app.settings_field + 2) % 3;
                            }
                            crossterm::event::KeyCode::Up => {
                                if app.settings_field == 2 {
                                    let i = app.model_list_state.selected().unwrap_or(0).saturating_sub(1);
                                    app.model_list_state.select(Some(i));
                                }
                            }
                            crossterm::event::KeyCode::Down => {
                                if app.settings_field == 2 {
                                    let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(App::get_available_models().len() - 1);
                                    app.model_list_state.select(Some(i));
                                }
                            }
                            crossterm::event::KeyCode::Enter => {
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
                                let selected_idx = app.model_list_state.selected().unwrap_or(0).min(models.len().saturating_sub(1));
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
                            },
                            crossterm::event::KeyCode::Char(c) => {
                                match app.settings_field {
                                    0 => app.settings_api_key.push(c),
                                    1 => app.settings_base_url.push(c),
                                    _ => {}
                                }
                            }
                            crossterm::event::KeyCode::Backspace => {
                                match app.settings_field {
                                    0 => { app.settings_api_key.pop(); }
                                    1 => { app.settings_base_url.pop(); }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    },
                    AppState::CustomModel => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => app.state = AppState::ModelSelector,
                            crossterm::event::KeyCode::Tab => app.custom_model_field = (app.custom_model_field + 1) % 2,
                            crossterm::event::KeyCode::Enter => {
                                if !app.custom_model_name.is_empty() && !app.custom_base_url.is_empty() {
                                    let normalized_base_url = App::normalize_base_url(&app.custom_base_url);
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
                                            Err(e) => app.error = Some(format!("Error initializing: {}", e)),
                                        }
                                    }
                                }
                            },
                            crossterm::event::KeyCode::Char(c) => {
                                if app.custom_model_field == 0 { app.custom_model_name.push(c); } else { app.custom_base_url.push(c); }
                            },
                            crossterm::event::KeyCode::Backspace => {
                                if app.custom_model_field == 0 { app.custom_model_name.pop(); } else { app.custom_base_url.pop(); }
                            },
                            _ => {}
                        }
                    },
                    AppState::Help => {
                        if matches!(key.code, crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q')) {
                            app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),     // header
            Constraint::Min(8),        // main split
            Constraint::Length(3),     // input
            Constraint::Length(1),     // status bar
        ])
        .split(f.area());

    render_header(f, app, layout[0]);

    // Main area: special layout for Welcome to give logo space; Chat uses full width; others keep split view.
    match app.state {
        AppState::Welcome => {
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(layout[1]);
            render_messages(f, app, main_chunks[0]);
            render_welcome(f, app, main_chunks[1]);
        }
        AppState::Chat => {
            // Chat state uses full width - no right panel
            render_messages(f, app, layout[1]);
        }
        _ => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1]);

            render_messages(f, app, main_chunks[0]);

            match app.state {
                AppState::ModelSelector => render_model_selector(f, app, main_chunks[1]),
                AppState::AgentSelector => render_agent_selector(f, app, main_chunks[1]),
                AppState::Settings => render_settings(f, app, main_chunks[1]),
                AppState::Help => render_help(f, app, main_chunks[1]),
                AppState::CustomModel => render_custom_model(f, app, main_chunks[1]),
                AppState::Chat | AppState::Welcome => unreachable!(),
            }
        }
    }

    render_input(f, app, layout[2]);
    render_status_bar(f, app, layout[3]);
}

fn render_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let messages: Vec<ListItem> = app.chat_messages.iter().map(|msg| {
        match msg {
            ChatMessage::User(content) => {
                ListItem::new(Line::from(vec![
                    Span::styled("You: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(content.clone()),
                ]))
            }
            ChatMessage::Assistant(content) => {
                let lines: Vec<Line> = content.lines().map(|l| Line::from(l)).collect();
                ListItem::new(vec![
                    Line::from(Span::styled("Pengy: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))),
                ].into_iter().chain(lines).collect::<Vec<_>>())
            }
            ChatMessage::ToolCall { name, args, result, status, .. } => {
                let status_style = match status {
                    ToolStatus::Running => Style::default().fg(Color::Yellow),
                    ToolStatus::Success => Style::default().fg(Color::Green),
                    ToolStatus::Error => Style::default().fg(Color::Red),
                };
                let icon = match status {
                    ToolStatus::Running => "⏳",
                    ToolStatus::Success => "✓",
                    ToolStatus::Error => "✗",
                };
                
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(format!("{} Tool: {} ", icon, name), status_style.add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::styled("  Args: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(args.clone(), Style::default().fg(Color::Gray)),
                    ]),
                ];
                
                if let Some(res) = result {
                    lines.push(Line::from(vec![
                        Span::styled("  Result: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(res.chars().take(100).collect::<String>() + if res.len() > 100 { "..." } else { "" }, Style::default().fg(Color::DarkGray)),
                    ]));
                }
                
                ListItem::new(lines).style(Style::default().bg(Color::Rgb(20, 20, 20)))
            }
            ChatMessage::Thinking(content) => {
                ListItem::new(Line::from(vec![
                    Span::styled("⚡ ", Style::default().fg(Color::Yellow)),
                    Span::styled(content.clone(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                ]))
            }
            ChatMessage::Step { step, max } => {
                let percent = (*step as f64 / *max as f64 * 100.0) as u16;
                // Simple progress bar
                let bars = "█".repeat((percent / 5) as usize);
                ListItem::new(Line::from(vec![
                    Span::styled(format!("Step {}/{}: ", step, max), Style::default().fg(Color::Cyan)),
                    Span::styled(bars, Style::default().fg(Color::Cyan)),
                ]))
            }
            ChatMessage::Error(err) => {
                ListItem::new(Line::from(vec![
                    Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled(err.clone(), Style::default().fg(Color::Red)),
                ]))
            }
        }
    }).collect();

    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::NONE))
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(messages_list, area, &mut app.list_state);

    // Scrollbar
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scroll_state = app.scroll_state;
    scroll_state = scroll_state.content_length(app.chat_messages.len());
    f.render_stateful_widget(scrollbar, area, &mut scroll_state);
    app.scroll_state = scroll_state;
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let state = format!("{:?}", app.state);
    let title = format!(" Pengy Agent {} │ State: {} ", VERSION, state);
    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).bg(Color::Black).add_modifier(Modifier::BOLD));
    f.render_widget(header, area);
}

fn render_input(f: &mut Frame, app: &mut App, area: Rect) {
    let label = if app.loading { "Sending..." } else { "Message" };
    let input_display = format!("{}: {}", label, app.chat_input);
    let input_paragraph = Paragraph::new(input_display)
        .block(Block::default().borders(Borders::ALL).title("Input").border_style(Style::default().fg(Color::DarkGray)))
        .style(Style::default().fg(Color::White));
    f.render_widget(input_paragraph, area);

    if app.show_command_hints {
        render_command_hints(f, app, area);
    }

    let cursor_x = (area.x + 2 + label.len() as u16 + 2 + app.input_cursor as u16)
        .min(area.x + area.width.saturating_sub(1));
    let cursor_y = area.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let model_name = app.selected_model.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| "None".to_string());
    let agent_name = format!("{:?}", app.selected_agent);
    let loading = if app.loading { "● Running" } else { "● Idle" };
    let status_text = format!(" Model: {} │ Agent: {} │ {} ", model_name, agent_name, loading);
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Black));
    f.render_widget(status, area);
}

fn render_chat_panel(f: &mut Frame, app: &App, area: Rect) {
    let commands = vec![
        "/models - Select Model",
        "/agents - Select Agent",
        "/settings - API key / model / base URL",
        "/help - Help",
        "/clear - Reset conversation",
    ];
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("Shortcuts", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    for c in commands {
        lines.push(Line::from(Span::styled(c, Style::default().fg(Color::Gray))));
    }

    if app.loading {
        lines.push(Line::from(Span::styled("Status: running...", Style::default().fg(Color::Yellow))));
    }

    let block = Block::default().borders(Borders::ALL).title("Panel");
    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(Clear, area);
    f.render_widget(p, area);
}

// Reuse other render functions...
fn render_welcome(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(area);

    // Logo
    let logo_lines: Vec<Line> = app.logo.lines().map(|l| Line::from(Span::styled(l, Style::default().fg(Color::Cyan)))).collect();
    let logo = Paragraph::new(logo_lines).alignment(Alignment::Center);
    f.render_widget(logo, vertical[0]);

    // Hint text
    let hint = Paragraph::new("Type /help for available commands")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, vertical[1]);
}

// Helper functions
fn render_command_hints(f: &mut Frame, app: &App, area: Rect) {
    let hints = app.get_command_hints();
    
    // If user has typed just "/" or it starts with "/", show all matching commands
    // Filter commands that start with what the user has typed
    let filtered: Vec<ListItem> = hints.iter()
        .filter(|(c, _)| {
            if app.chat_input == "/" {
                // Show all commands when just "/" is typed
                true
            } else {
                // Otherwise filter by what they've typed
                c.starts_with(&app.chat_input)
            }
        })
        .map(|(c, d)| {
            ListItem::new(format!("{} - {}", c, d))
        })
        .collect();
    
    if filtered.is_empty() { return; }
    
    // Show more commands when just "/" is typed, limit to 6 for better visibility
    let max_height = if app.chat_input == "/" { 6 } else { 5 };
    let height = filtered.len().min(max_height) as u16;
    let popup_area = Rect { 
        x: area.x, 
        y: area.y.saturating_sub(height + 1), 
        width: area.width, 
        height: height + 1 
    };
    
    let title = if app.chat_input == "/" {
        "Available Commands (type to filter)"
    } else {
        "Commands"
    };
    
    let list = List::new(filtered)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().bg(Color::Rgb(30, 30, 30)).fg(Color::White))
        )
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(Clear, popup_area);
    f.render_widget(list, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Missing render functions need implementation or copying back
fn render_model_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Select Model");
    let items: Vec<ListItem> = App::get_available_models().iter().map(|m| ListItem::new(m.name.clone())).collect();
    let list = List::new(items).block(block).highlight_style(Style::default().fg(Color::Yellow));
    f.render_stateful_widget(list, rect, &mut app.model_list_state);
}

fn render_agent_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Select Agent");
    let items: Vec<ListItem> = App::get_available_agents().iter().map(|(n, d, _)| ListItem::new(format!("{} - {}", n, d))).collect();
    let list = List::new(items).block(block).highlight_style(Style::default().fg(Color::Yellow));
    f.render_stateful_widget(list, rect, &mut app.agent_list_state);
}

fn render_settings(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 70, area);
    f.render_widget(Clear, rect);

    let block = Block::default().borders(Borders::ALL).title("Settings");
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),   // API key row
            Constraint::Length(4),   // Base URL row
            Constraint::Min(8),      // Model list
            Constraint::Length(3),   // Selection summary
            Constraint::Length(3),   // Footer / errors
        ])
        .split(inner);

    let truncate = |s: &str, max_len: usize| -> String {
        if s.len() > max_len {
            format!("{}…", s.chars().take(max_len).collect::<String>())
        } else {
            s.to_string()
        }
    };

    // API key block
    let api_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.settings_field == 0 { "API Key (active)" } else { "API Key" });
    let masked_key = if app.settings_api_key.is_empty() {
        "<not set>".to_string()
    } else {
        let visible_tail: String = app
            .settings_api_key
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let hidden_len = app.settings_api_key.len().saturating_sub(visible_tail.len());
        format!("{}{}", "*".repeat(hidden_len), visible_tail)
    };
    let api_para = Paragraph::new(masked_key.clone())
        .block(api_block)
        .wrap(Wrap { trim: true });
    f.render_widget(api_para, layout[0]);

    // Base URL block
    let url_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.settings_field == 1 { "Base URL (active)" } else { "Base URL" });
    let base_url = if app.settings_base_url.is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        app.settings_base_url.clone()
    };
    let url_para = Paragraph::new(truncate(&base_url, 64))
        .block(url_block)
        .wrap(Wrap { trim: true });
    f.render_widget(url_para, layout[1]);

    // Model list
    let models = App::get_available_models();
    let items: Vec<ListItem> = models
        .iter()
        .map(|m| {
            let caption = if m.provider == "Custom" {
                format!("{} (custom)", m.name)
            } else {
                format!("{} - {}", m.name, m.provider)
            };
            ListItem::new(caption)
        })
        .collect();

    let model_block = Block::default().borders(Borders::ALL).title("Models (↑/↓)");
    let model_list = List::new(items)
        .block(model_block)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(model_list, layout[2], &mut app.model_list_state);

    // Selection summary
    let summary = {
        let current = app
            .selected_model
            .as_ref()
            .map(|m| format!("{} ({})", m.name, m.provider))
            .unwrap_or_else(|| "None selected".to_string());
        let base = truncate(&base_url, 48);
        format!("Selected: {}  |  Base: {}", current, base)
    };
    let summary_para = Paragraph::new(summary)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(summary_para, layout[3]);

    // Footer / errors
    let mut footer_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw("/"),
            Span::styled("Shift+Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" move fields  "),
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" models  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" close"),
        ]),
    ];

    if let Some(err) = &app.error {
        footer_lines.push(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let footer = Paragraph::new(footer_lines).wrap(Wrap { trim: true });
    f.render_widget(footer, layout[4]);

    // Cursor placement for active field
    if app.settings_field == 0 {
        let cursor_x = (layout[0].x + 1 + masked_key.len() as u16)
            .min(layout[0].x + layout[0].width.saturating_sub(1));
        let cursor_y = layout[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    } else if app.settings_field == 1 {
        let cursor_x = (layout[1].x + 1 + base_url.len() as u16)
            .min(layout[1].x + layout[1].width.saturating_sub(1));
        let cursor_y = layout[1].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_help(f: &mut Frame, _app: &App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Help");
    let text = "Available Commands:\n\n/models - Select Model\n/agents - Select Agent\n/settings - Configure API key / model / base URL\n/help - Show this help screen\n/clear - Clear conversation and reset agent\n\nNavigation:\nUse Arrows to navigate lists.\nEnter to select.\nEsc to go back.\n\nTip: Type '/' in the input to see all available commands.";
    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, rect);
}

fn render_custom_model(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 40, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Custom Model");
    
    let err_msg = if let Some(ref e) = app.error { 
        format!("\n\nError: {}", e) 
    } else { 
        String::new() 
    };
    
    let name_label = if app.custom_model_field == 0 { "> Name: " } else { "  Name: " };
    let url_label = if app.custom_model_field == 1 { "> Base URL: " } else { "  Base URL: " };

    let text = format!("{}{}\n{}{}\n(Tab to switch, Enter save){}", 
        name_label, app.custom_model_name, url_label, app.custom_base_url, err_msg);
        
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, rect);

    let (active_label, active_value, line_offset) = if app.custom_model_field == 0 {
        (name_label, &app.custom_model_name, 1)
    } else {
        (url_label, &app.custom_base_url, 2)
    };

    let cursor_x = (rect.x + 1 + (active_label.len() + active_value.len()) as u16)
        .min(rect.x + rect.width.saturating_sub(1));
    let cursor_y = rect.y + line_offset;
    f.set_cursor_position((cursor_x, cursor_y));
}
