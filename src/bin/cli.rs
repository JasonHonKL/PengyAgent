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
const EMBED_LOGO: &str = include_str!("../../logo.txt");
use serde_json;
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
    SessionSelector,
    BaseUrlSelector,
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
    session_list_state: ListState,
    sessions: Vec<String>,
    current_session: usize,
    settings_api_key: String,
    settings_base_url: String,
    settings_field: usize,
    search_query: String,
    model_search_focused: bool,  // Track if search field is focused in model selector
    show_command_hints: bool,
    custom_model_name: String,
    custom_base_url: String,
    custom_model_field: usize,
    previous_state: Option<AppState>,
    user_scrolled: bool,  // Track if user manually scrolled (don't auto-scroll)
    rx: mpsc::UnboundedReceiver<AgentEvent>,
    tx: mpsc::UnboundedSender<AgentEvent>,
    agent_rx: mpsc::UnboundedReceiver<Agent>,
    agent_tx: mpsc::UnboundedSender<Agent>,
    modified_files: std::collections::HashMap<String, (usize, usize)>, // file_path -> (added, removed) lines
    pending_tool_calls: Vec<(String, String, String)>, // Vec of (id, name, args) for pending tool calls
}

impl App {
    fn load_logo() -> String {
        EMBED_LOGO.to_string()
    }

    fn config_path() -> PathBuf {
        // Always use home directory for config file (single global location)
        if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(CONFIG_FILE)
        } else {
            // Fallback to current directory only if HOME is not set (rare)
            PathBuf::from(CONFIG_FILE)
        }
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

        // Clear todo list on initialization (treat as new session)
        let todo_file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy_todo.json");
        let _ = std::fs::remove_file(&todo_file);

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
        session_list_state: {
            let mut s = ListState::default();
            s.select(Some(0));
            s
        },
        sessions: vec!["New session - default".to_string()],
        current_session: 0,
            settings_api_key: api_key,
            settings_base_url,
            settings_field: 0,
            search_query: String::new(),
            model_search_focused: false,
            show_command_hints: false,
            custom_model_name,
            custom_base_url,
            custom_model_field: 0,
            previous_state: None,
            user_scrolled: false,
            rx,
            tx,
            agent_rx,
            agent_tx,
            modified_files: std::collections::HashMap::new(),
            pending_tool_calls: Vec::new(),
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

    fn create_new_session(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let name = format!("Session {}", ts);
        self.sessions.push(name);
        self.current_session = self.sessions.len().saturating_sub(1);
        self.session_list_state.select(Some(self.current_session));
        // Reset conversation
        self.chat_messages.clear();
        self.list_state.select(None);
        self.user_scrolled = false;
        self.agent = None;
        self.loading = false;
        self.modified_files.clear();
        // Clear todo list for new session
        let todo_file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy_todo.json");
        let _ = std::fs::remove_file(&todo_file);
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
            // Provider Base URLs (for quick selection)
            ModelOption {
                name: "Provider: Mistral".to_string(),
                provider: "Mistral".to_string(),
                base_url: "https://api.mistral.ai/v1".to_string(),
            },
            ModelOption {
                name: "Provider: DeepSeek".to_string(),
                provider: "DeepSeek".to_string(),
                base_url: "https://api.deepseek.com/v1".to_string(),
            },
            ModelOption {
                name: "Provider: OpenRouter".to_string(),
                provider: "OpenRouter".to_string(),
                base_url: "https://openrouter.ai/api/v1".to_string(),
            },
            ModelOption {
                name: "Provider: OpenAI".to_string(),
                provider: "OpenAI".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
            },
            ModelOption {
                name: "Provider: Anthropic".to_string(),
                provider: "Anthropic".to_string(),
                base_url: "https://api.anthropic.com/v1".to_string(),
            },
            ModelOption {
                name: "Provider: GLM".to_string(),
                provider: "GLM".to_string(),
                base_url: "https://open.bigmodel.cn/api/paas/v4".to_string(),
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
            ("/baseurl", "select provider base URL (required for custom models)"),
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
                    Some(50),
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
                    Some(50),
                );
                self.agent = Some(agent);
            }
            AgentType::TestAgent => {
                let agent = create_test_agent(
                    model,
                    None,
                    Some(3),
                    Some(50),
                );
                self.agent = Some(agent);
            }
            AgentType::ControlAgent => {
                let agent = create_control_agent(
                    model,
                    None,
                    Some(3),
                    Some(50),
                );
                self.agent = Some(agent);
            }
            AgentType::IssueAgent => {
                let agent = create_issue_agent(
                    model,
                    None,
                    Some(3),
                    Some(50),
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
        self.user_scrolled = false;  // Reset scroll state when sending new message

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
                        Some(50),
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

        if !self.chat_messages.is_empty() && !self.user_scrolled {
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
                AgentEvent::Step { .. } => {
                    // Hide step events (user does not want to see steps)
                }
                AgentEvent::ToolCall { tool_name, args } => {
                    // Store tool call info temporarily - don't show until we have result
                    let tool_id = format!("tool_{}", self.chat_messages.len() + self.pending_tool_calls.len());
                    self.pending_tool_calls.push((tool_id, tool_name, args));
                }
                AgentEvent::ToolResult { result } => {
                    // Get the most recent pending tool call and create ChatMessage with result
                    // This avoids showing "running" state and duplication
                    if let Some((tool_id, name, args_str)) = self.pending_tool_calls.pop() {
                        // Create tool call message directly with success status
                    self.chat_messages.push(ChatMessage::ToolCall {
                            id: tool_id.clone(),
                            name: name.clone(),
                            args: args_str.clone(),
                            result: Some(result.clone()),
                            status: ToolStatus::Success,
                        });
                        
                        // Track file modifications for edit tool
                        if name == "edit" {
                            if let Ok(json_args) = serde_json::from_str::<serde_json::Value>(&args_str) {
                                if let Some(file_path) = json_args.get("filePath").and_then(|v| v.as_str()) {
                                    if let (Some(old_str), Some(new_str)) = (
                                        json_args.get("oldString").and_then(|v| v.as_str()),
                                        json_args.get("newString").and_then(|v| v.as_str()),
                                    ) {
                                        let added = new_str.lines().count();
                                        let removed = old_str.lines().count();
                                        let entry = self.modified_files.entry(file_path.to_string())
                                            .or_insert((0, 0));
                                        entry.0 += added;
                                        entry.1 += removed;
                                    }
                                }
                            }
                        }
                    } else {
                        // No pending tool call found - create one anyway (shouldn't happen but handle gracefully)
                        self.chat_messages.push(ChatMessage::ToolCall {
                            id: format!("tool_{}", self.chat_messages.len()),
                            name: "unknown".to_string(),
                            args: String::new(),
                            result: Some(result),
                            status: ToolStatus::Success,
                        });
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
            // Don't auto-scroll here - let render_messages handle it
            // This prevents resetting scroll position when processing events
        }
    }
}

// Helper functions for key handling
fn handle_welcome_key(app: &mut App, key: crossterm::event::KeyCode, rt: &tokio::runtime::Runtime) -> Result<(), Box<dyn Error>> {
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
                    app.session_list_state.select(Some(app.current_session.min(app.sessions.len().saturating_sub(1))));
                                    app.chat_input.clear();
                                    app.input_cursor = 0;
                    return Ok(());
                }
                handle_command_inline(app, &cmd, AppState::Welcome);
                                } else {
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
    Ok(())
}

fn handle_chat_key(app: &mut App, key: crossterm::event::KeyCode, rt: &tokio::runtime::Runtime) -> Result<(), Box<dyn Error>> {
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
                    app.session_list_state.select(Some(app.current_session.min(app.sessions.len().saturating_sub(1))));
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
        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Down | 
        crossterm::event::KeyCode::PageUp | crossterm::event::KeyCode::PageDown => {
            app.user_scrolled = true;  // Mark that user is manually scrolling
            let len = app.chat_messages.len();
            if len > 0 {
                let selected = app.list_state.selected().unwrap_or(len.saturating_sub(1));
                let amount = match key {
                    crossterm::event::KeyCode::PageUp | crossterm::event::KeyCode::PageDown => 10,
                    _ => 1,
                };
                let new_selection = match key {
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::PageUp => {
                        if selected == 0 {
                            0  // Already at top
                        } else {
                            selected.saturating_sub(amount)
                        }
                    },
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
            if app.chat_messages.len() > 0 {
                app.list_state.select(Some(app.chat_messages.len().saturating_sub(1)));
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
        crossterm::event::KeyCode::Left => if app.input_cursor > 0 { app.input_cursor -= 1; },
        crossterm::event::KeyCode::Right => if app.input_cursor < app.chat_input.len() { app.input_cursor += 1; },
        _ => {}
    }
    Ok(())
}

fn handle_command_inline(app: &mut App, cmd: &str, previous_state: AppState) {
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
                                        // Find and select the currently selected model in the list
                                        let models = App::get_available_models();
                                        if let Some(selected) = &app.selected_model {
                                            if let Some(idx) = models.iter().position(|m| m.name == selected.name && m.provider == selected.provider) {
                                                app.model_list_state.select(Some(idx));
                                            }
                                        }
    } else if cmd.starts_with("/baseurl") {
        app.previous_state = Some(previous_state);
        app.state = AppState::BaseUrlSelector;
        app.model_search_focused = true;  // Default to search active
        app.search_query.clear();
        // Find and select the currently selected provider if any
        let models = App::get_available_models();
        let provider_models: Vec<&ModelOption> = models.iter()
            .filter(|m| m.name.starts_with("Provider:"))
            .collect();
        if let Some(ref selected) = app.selected_model {
            if let Some(idx) = provider_models.iter().position(|m| m.base_url == selected.base_url) {
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

/// Parse command-line arguments for cmd mode
/// Returns: (prompt, agent, model, provider, api_key, base_url)
fn parse_cmd_args() -> Option<(String, String, String, String, String, Option<String>)> {
    let args: Vec<String> = env::args().collect();
    
    // Check if we have at least some arguments (minimum 6 for required args)
    if args.len() < 6 {
        return None;
    }
    
    let mut prompt = None;
    let mut agent = None;
    let mut model = None;
    let mut provider = None;
    let mut api_key = None;
    let mut base_url = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prompt" => {
                if i + 1 < args.len() {
                    prompt = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return None;
                }
            }
            "--agent" => {
                if i + 1 < args.len() {
                    agent = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return None;
                }
            }
            "--model" => {
                if i + 1 < args.len() {
                    model = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return None;
                }
            }
            "--provider" => {
                if i + 1 < args.len() {
                    provider = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return None;
                }
            }
            "--api-key" => {
                if i + 1 < args.len() {
                    api_key = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return None;
                }
            }
            "--base-url" => {
                if i + 1 < args.len() {
                    base_url = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return None;
                }
            }
            _ => i += 1,
        }
    }
    
    if let (Some(p), Some(a), Some(m), Some(pr), Some(k)) = (prompt, agent, model, provider, api_key) {
        Some((p, a, m, pr, k, base_url))
    } else {
        None
    }
}

/// Convert agent string to AgentType
fn parse_agent_type(agent_str: &str) -> Result<AgentType, Box<dyn Error>> {
    match agent_str.to_lowercase().as_str() {
        "coder" | "coder agent" => Ok(AgentType::Coder),
        "code researcher" | "code-researcher" | "researcher" => Ok(AgentType::CodeResearcher),
        "test agent" | "test-agent" | "test" => Ok(AgentType::TestAgent),
        "pengy agent" | "pengy-agent" | "pengy" => Ok(AgentType::PengyAgent),
        "control agent" | "control-agent" | "control" => Ok(AgentType::ControlAgent),
        "issue agent" | "issue-agent" | "issue" => Ok(AgentType::IssueAgent),
        _ => Err(format!("Unknown agent type: {}. Available: coder, code-researcher, test-agent, pengy-agent, control-agent, issue-agent", agent_str).into()),
    }
}

/// Run agent in command mode (non-interactive)
async fn run_cmd_mode(
    prompt: String,
    agent_type: AgentType,
    model_name: String,
    provider: String,
    api_key: String,
    custom_base_url: Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Determine base URL based on provider or custom base URL
    let base_url = if let Some(custom_url) = custom_base_url {
        App::normalize_base_url(&custom_url)
    } else if provider.to_lowercase() == "custom" {
        // For custom provider without base URL, use default
        DEFAULT_BASE_URL.to_string()
    } else {
        // Try to find the base URL from available models
        let models = App::get_available_models();
        let found_model = models.iter().find(|m| {
            m.name == model_name && m.provider.to_lowercase() == provider.to_lowercase()
        });
        
        if let Some(m) = found_model {
            App::normalize_base_url(&m.base_url)
        } else {
            // Default to OpenRouter if not found
            DEFAULT_BASE_URL.to_string()
        }
    };
    
    let model = Model::new(model_name.clone(), api_key.clone(), base_url.clone());
    
    // Print initial info
    println!("Running agent in command mode...");
    println!("Agent: {:?}", agent_type);
    println!("Model: {} ({})", model_name, provider);
    println!("Prompt: {}\n", prompt);
    
    // Create callback to print events
    let callback = |event: AgentEvent| {
        match event {
            AgentEvent::Step { step, max_steps } => {
                println!("[Step {}/{}]", step, max_steps);
            }
            AgentEvent::ToolCall { tool_name, args } => {
                println!("[Tool Call] {} with args: {}", tool_name, args);
            }
            AgentEvent::ToolResult { result } => {
                println!("[Tool Result] {}", result);
            }
            AgentEvent::Thinking { content } => {
                println!("[Thinking] {}", content);
            }
            AgentEvent::FinalResponse { content } => {
                println!("\n[Final Response]\n{}", content);
            }
            AgentEvent::Error { error } => {
                eprintln!("[Error] {}", error);
            }
            AgentEvent::VisionAnalysis { status } => {
                println!("[Vision] {}", status);
            }
        }
    };
    
    // Run the appropriate agent
    match agent_type {
        AgentType::PengyAgent => {
            let _ = run_pengy_agent(
                model,
                api_key,
                base_url,
                Some("openai/text-embedding-3-small".to_string()),
                prompt,
                Some(3),
                Some(50),
                callback,
            ).await;
        }
        AgentType::Coder => {
            let mut agent = create_coder_agent(
                model,
                None,
                Some(3),
                Some(50),
            );
            agent.run(prompt, callback).await;
        }
        AgentType::CodeResearcher => {
            let mut agent = create_code_researcher_agent(
                model,
                api_key,
                base_url,
                Some("openai/text-embedding-3-small".to_string()),
                None,
                Some(3),
                Some(50),
            );
            agent.run(prompt, callback).await;
        }
        AgentType::TestAgent => {
            let mut agent = create_test_agent(
                model,
                None,
                Some(3),
                Some(50),
            );
            agent.run(prompt, callback).await;
        }
        AgentType::ControlAgent => {
            let mut agent = create_control_agent(
                model,
                None,
                Some(3),
                Some(50),
            );
            agent.run(prompt, callback).await;
        }
        AgentType::IssueAgent => {
            let mut agent = create_issue_agent(
                model,
                None,
                Some(3),
                Some(50),
            );
            agent.run(prompt, callback).await;
        }
    }
    
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Check if we're in command mode
    if let Some((prompt, agent_str, model, provider, api_key, base_url)) = parse_cmd_args() {
        // Run in command mode
        let rt = tokio::runtime::Runtime::new()?;
        match parse_agent_type(&agent_str) {
            Ok(agent_type) => {
                rt.block_on(run_cmd_mode(prompt, agent_type, model, provider, api_key, base_url))?;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!("\nUsage: pengy --prompt \"<prompt>\" --agent <agent-type> --model <model-name> --provider <provider> --api-key <api-key> [--base-url <base-url>]");
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
                eprintln!("  pengy --prompt \"Write a hello world function\" --agent coder --model openai/gpt-4o --provider OpenAI --api-key sk-...");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    
    // Otherwise, run in TUI mode
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

                let should_quit = match app.state {
                    AppState::Welcome => {
                        match handle_welcome_key(&mut app, key.code, &rt) {
                            Err(e) if e.to_string() == "quit" => true,
                            _ => false,
                        }
                    }
                    AppState::Chat => {
                        match handle_chat_key(&mut app, key.code, &rt) {
                            Err(e) if e.to_string() == "quit" => true,
                            _ => false,
                        }
                    }
                    AppState::SessionSelector => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                            }
                            crossterm::event::KeyCode::Enter => {
                                if let Some(idx) = app.session_list_state.selected() {
                                    if idx < app.sessions.len() {
                                        app.current_session = idx;
                                        app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                                    }
                                }
                            }
                            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                                let i = (app.session_list_state.selected().unwrap_or(0) + 1).min(app.sessions.len().saturating_sub(1));
                                app.session_list_state.select(Some(i));
                            }
                            crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                                let i = app.session_list_state.selected().unwrap_or(0).saturating_sub(1);
                                app.session_list_state.select(Some(i));
                            }
                            crossterm::event::KeyCode::Char('h') => {}
                            crossterm::event::KeyCode::Char('l') => {}
                            _ => {}
                        }
                        false
                    }
                    AppState::ModelSelector => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                    app.search_query.clear();
                                } else {
                                    app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                                }
                            },
                            crossterm::event::KeyCode::Tab => {
                                app.model_search_focused = !app.model_search_focused;
                                if !app.model_search_focused {
                                    // When switching to list, reset selection to first filtered item
                                    let all_models = App::get_available_models();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                        all_models.iter()
                                            .filter(|m| !m.name.starts_with("Provider:"))
                                            .collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        all_models.iter()
                                            .filter(|m| {
                                                !m.name.starts_with("Provider:") && (
                                                    m.name.to_lowercase().contains(&query_lower) ||
                                                    m.provider.to_lowercase().contains(&query_lower) ||
                                                    m.base_url.to_lowercase().contains(&query_lower)
                                                )
                                            })
                                            .collect()
                                    };
                                    if !filtered.is_empty() {
                                        app.model_list_state.select(Some(0));
                                    }
                                }
                            },
                            crossterm::event::KeyCode::Enter => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                } else {
                                let models = App::get_available_models();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                        models.iter()
                                            .filter(|m| !m.name.starts_with("Provider:"))
                                            .collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        models.iter()
                                            .filter(|m| {
                                                !m.name.starts_with("Provider:") && (
                                                    m.name.to_lowercase().contains(&query_lower) ||
                                                    m.provider.to_lowercase().contains(&query_lower) ||
                                                    m.base_url.to_lowercase().contains(&query_lower)
                                                )
                                            })
                                            .collect()
                                    };
                                    
                                if let Some(selected) = app.model_list_state.selected() {
                                        if selected < filtered.len() {
                                            let model = filtered[selected].clone();
                                        if model.provider == "Custom" {
                                            app.custom_model_field = 0;
                                            app.custom_model_name.clear();
                                                app.custom_base_url.clear(); // Clear base URL for custom model
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
                                }
                            },
                            crossterm::event::KeyCode::Up if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    all_models.iter()
                                        .filter(|m| !m.name.starts_with("Provider:"))
                                        .collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models.iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:") && (
                                                m.name.to_lowercase().contains(&query_lower) ||
                                                m.provider.to_lowercase().contains(&query_lower) ||
                                                m.base_url.to_lowercase().contains(&query_lower)
                                            )
                                        })
                                        .collect()
                                };
                                let i = app.model_list_state.selected().unwrap_or(0).saturating_sub(1);
                                app.model_list_state.select(Some(i.min(filtered.len().saturating_sub(1))));
                            },
                            crossterm::event::KeyCode::Down if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    all_models.iter()
                                        .filter(|m| !m.name.starts_with("Provider:"))
                                        .collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models.iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:") && (
                                                m.name.to_lowercase().contains(&query_lower) ||
                                                m.provider.to_lowercase().contains(&query_lower) ||
                                                m.base_url.to_lowercase().contains(&query_lower)
                                            )
                                        })
                                        .collect()
                                };
                                let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(filtered.len().saturating_sub(1));
                                app.model_list_state.select(Some(i));
                            },
                            crossterm::event::KeyCode::Char(c) if app.model_search_focused => {
                                app.search_query.push(c);
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models.iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:") && (
                                                m.name.to_lowercase().contains(&query_lower) ||
                                                m.provider.to_lowercase().contains(&query_lower) ||
                                                m.base_url.to_lowercase().contains(&query_lower)
                                            )
                                        })
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            },
                            crossterm::event::KeyCode::Backspace if app.model_search_focused => {
                                app.search_query.pop();
                                let all_models = App::get_available_models();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    all_models.iter()
                                        .filter(|m| !m.name.starts_with("Provider:"))
                                        .collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    all_models.iter()
                                        .filter(|m| {
                                            !m.name.starts_with("Provider:") && (
                                                m.name.to_lowercase().contains(&query_lower) ||
                                                m.provider.to_lowercase().contains(&query_lower) ||
                                                m.base_url.to_lowercase().contains(&query_lower)
                                            )
                                        })
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            },
                            _ => {}
                        }
                        false
                    },
                    AppState::AgentSelector => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => app.state = app.previous_state.clone().unwrap_or(AppState::Welcome),
                            crossterm::event::KeyCode::Tab => {
                                // Switch to next agent
                                let agents = App::get_available_agents();
                                let current = app.agent_list_state.selected().unwrap_or(0);
                                let next = (current + 1) % agents.len();
                                app.agent_list_state.select(Some(next));
                                app.selected_agent = agents[next].2;
                            },
                            crossterm::event::KeyCode::BackTab => {
                                // Switch to previous agent
                                let agents = App::get_available_agents();
                                let current = app.agent_list_state.selected().unwrap_or(0);
                                let prev = if current == 0 { agents.len() - 1 } else { current - 1 };
                                app.agent_list_state.select(Some(prev));
                                app.selected_agent = agents[prev].2;
                            },
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
                                let agents = App::get_available_agents();
                                app.selected_agent = agents[i].2;
                            },
                            crossterm::event::KeyCode::Down => {
                                let agents = App::get_available_agents();
                                let i = (app.agent_list_state.selected().unwrap_or(0) + 1).min(agents.len() - 1);
                                app.agent_list_state.select(Some(i));
                                app.selected_agent = agents[i].2;
                            },
                            _ => {}
                        }
                        false
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
                        false
                    },
                    AppState::BaseUrlSelector => {
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                    app.search_query.clear();
                                } else {
                                    app.state = app.previous_state.clone().unwrap_or(AppState::Welcome);
                                }
                            },
                            crossterm::event::KeyCode::Tab => {
                                app.model_search_focused = !app.model_search_focused;
                                if !app.model_search_focused {
                                    let all_models = App::get_available_models();
                                    let provider_models: Vec<&ModelOption> = all_models.iter()
                                        .filter(|m| m.name.starts_with("Provider:"))
                                        .collect();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                        provider_models.iter().copied().collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        provider_models.iter()
                                            .filter(|m| {
                                                m.name.to_lowercase().contains(&query_lower) ||
                                                m.provider.to_lowercase().contains(&query_lower) ||
                                                m.base_url.to_lowercase().contains(&query_lower)
                                            })
                                            .copied()
                                            .collect()
                                    };
                                    if !filtered.is_empty() {
                                        app.model_list_state.select(Some(0));
                                    }
                                }
                            },
                            crossterm::event::KeyCode::Enter => {
                                if app.model_search_focused {
                                    app.model_search_focused = false;
                                } else {
                                    let all_models = App::get_available_models();
                                    let provider_models: Vec<&ModelOption> = all_models.iter()
                                        .filter(|m| m.name.starts_with("Provider:"))
                                        .collect();
                                    let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                        provider_models.iter().copied().collect()
                                    } else {
                                        let query_lower = app.search_query.to_lowercase();
                                        provider_models.iter()
                                            .filter(|m| {
                                                m.name.to_lowercase().contains(&query_lower) ||
                                                m.provider.to_lowercase().contains(&query_lower) ||
                                                m.base_url.to_lowercase().contains(&query_lower)
                                            })
                                            .copied()
                                            .collect()
                                    };
                                    
                                    if let Some(selected) = app.model_list_state.selected() {
                                        if selected < filtered.len() {
                                            let provider = filtered[selected];
                                            // Update settings and selected model
                                            app.settings_api_key = app.api_key.clone();
                                            app.settings_base_url = provider.base_url.clone();
                                            app.settings_field = 1;
                                            app.error = None;
                                            
                                            // Update selected_model's base_url if one exists
                                            if let Some(ref mut selected) = app.selected_model {
                                                selected.base_url = provider.base_url.clone();
                                            }
                                            
                                            app.state = AppState::Settings;
                                        }
                                    }
                                }
                            },
                            crossterm::event::KeyCode::Up if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models.iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    provider_models.iter().copied().collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models.iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower) ||
                                            m.provider.to_lowercase().contains(&query_lower) ||
                                            m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                let i = app.model_list_state.selected().unwrap_or(0).saturating_sub(1);
                                app.model_list_state.select(Some(i.min(filtered.len().saturating_sub(1))));
                            },
                            crossterm::event::KeyCode::Down if !app.model_search_focused => {
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models.iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    provider_models.iter().copied().collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models.iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower) ||
                                            m.provider.to_lowercase().contains(&query_lower) ||
                                            m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                let i = (app.model_list_state.selected().unwrap_or(0) + 1).min(filtered.len().saturating_sub(1));
                                app.model_list_state.select(Some(i));
                            },
                            crossterm::event::KeyCode::Char(c) if app.model_search_focused => {
                                app.search_query.push(c);
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models.iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models.iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower) ||
                                            m.provider.to_lowercase().contains(&query_lower) ||
                                            m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            },
                            crossterm::event::KeyCode::Backspace if app.model_search_focused => {
                                app.search_query.pop();
                                let all_models = App::get_available_models();
                                let provider_models: Vec<&ModelOption> = all_models.iter()
                                    .filter(|m| m.name.starts_with("Provider:"))
                                    .collect();
                                let filtered: Vec<&ModelOption> = if app.search_query.is_empty() {
                                    provider_models.iter().copied().collect()
                                } else {
                                    let query_lower = app.search_query.to_lowercase();
                                    provider_models.iter()
                                        .filter(|m| {
                                            m.name.to_lowercase().contains(&query_lower) ||
                                            m.provider.to_lowercase().contains(&query_lower) ||
                                            m.base_url.to_lowercase().contains(&query_lower)
                                        })
                                        .copied()
                                        .collect()
                                };
                                if !filtered.is_empty() {
                                    app.model_list_state.select(Some(0));
                                }
                            },
                            _ => {}
                        }
                        false
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
                        false
                    },
                    AppState::Help => {
                        if key.code == crossterm::event::KeyCode::Esc {
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),     // header
            Constraint::Min(0),        // main area - takes all available space
            Constraint::Length(3),     // input (thin 3-line input like standard terminal/OpenCode)
            Constraint::Length(1),     // status bar
        ])
        .split(f.area());

    render_header(f, app, layout[0]);

    // Main area: special layout for Welcome to give logo space; Chat uses full width; others keep split view.
    match app.state {
        AppState::Welcome => {
            // Welcome uses full width for elegant centered design
            render_welcome(f, app, layout[1]);
        }
        AppState::Chat => {
            // Chat state with right sidebar for modified files - much thinner
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(85), Constraint::Length(25)]) // Much thinner sidebar
                .split(layout[1]);
            render_messages(f, app, main_chunks[0]);
            render_chat_sidebar(f, app, main_chunks[1]);
        }
        AppState::SessionSelector => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1]);
            render_messages(f, app, main_chunks[0]);
            render_session_selector(f, app, main_chunks[1]);
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
                AppState::BaseUrlSelector => render_baseurl_selector(f, app, main_chunks[1]),
                AppState::SessionSelector | AppState::Chat | AppState::Welcome => unreachable!(),
            }
        }
    }

    // Center input - welcome uses full screen centering
    let input_area = match app.state {
        AppState::Welcome => {
            // Center input area on welcome page (like OpenCode) - matches OpenCode welcome styling
            centered_rect(70, 12, f.area()) // Using 12% height for better OpenCode match
        }
        AppState::CustomModel => {
            // Don't render main input when in CustomModel - the custom model form handles input
            Rect::default() // Empty rect, won't be used
        }
        _ => {
            // Full width input for chat and other states
            layout[2]
        }
    };
    
    // Render command hints at frame level (not inside input area) so they're always visible
    if (app.state == AppState::Welcome || app.state == AppState::Chat) && app.show_command_hints && app.chat_input.starts_with('/') {
        render_command_hints(f, app, input_area);
    }
    
    // Render input
    match app.state {
        AppState::Welcome => {
            render_input(f, app, input_area);
        }
        AppState::CustomModel => {
            // Don't render main input when in CustomModel - the custom model form handles input
        }
        _ => {
            render_input(f, app, input_area);
        }
    }
    
    render_status_bar(f, app, layout[3]);
}

fn parse_tool_args(args: &str) -> Option<serde_json::Value> {
    serde_json::from_str(args).ok()
}

fn format_tool_args_display(name: &str, args: &str) -> Vec<Line<'static>> {
    let parsed = parse_tool_args(args);
    
    // Format JSON args nicely for all tools - extract all values first to ensure ownership
    if let Some(json) = &parsed {
        let mut result = Vec::new();
        match name {
            "todo" => {
                if let Some(action_str) = json.get("action").and_then(|v| v.as_str()) {
                    let action = action_str.to_string();
                    let op = json.get("operation").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let desc = json.get("task_description").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let id = json.get("task_id").and_then(|v| v.as_u64());
                    
                    result.push(Line::from(vec![
                        Span::styled("  Action: ", Style::default().fg(Color::Gray)),
                        Span::styled(action, Style::default().fg(Color::White)),
                    ]));
                    if let Some(op) = op {
                        result.push(Line::from(vec![
                            Span::styled("  Operation: ", Style::default().fg(Color::Gray)),
                            Span::styled(op, Style::default().fg(Color::White)),
                        ]));
                    }
                    if let Some(desc) = desc {
                        let truncated = if desc.len() > 60 { format!("{}...", &desc[..60]) } else { desc };
                        result.push(Line::from(vec![
                            Span::styled("  Task: ", Style::default().fg(Color::Gray)),
                            Span::styled(truncated, Style::default().fg(Color::White)),
                        ]));
                    }
                    if let Some(id) = id {
                        result.push(Line::from(vec![
                            Span::styled("  Task ID: ", Style::default().fg(Color::Gray)),
                            Span::styled(id.to_string(), Style::default().fg(Color::White)),
                        ]));
                    }
                }
            }
            "edit" => {
                    if let (Some(file_path), Some(old_string), Some(new_string)) = (
                        json.get("filePath").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        json.get("oldString").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        json.get("newString").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    ) {
                        result.push(Line::from(vec![
                        Span::styled("  File: ", Style::default().fg(Color::Gray)),
                            Span::styled(file_path.clone(), Style::default().fg(Color::White)),
                        ]));
                        
                        let old_lines: Vec<String> = old_string.lines().map(|s| s.to_string()).collect();
                        let new_lines: Vec<String> = new_string.lines().map(|s| s.to_string()).collect();
                        
                        // Clean, elegant diff display - fixed-width side-by-side columns
                        let max_lines = old_lines.len().max(new_lines.len());
                        let max_display_lines = 25.min(max_lines);

                        let left_width: usize = 52;  // fixed width for old column (including line number)
                        let right_width: usize = 52; // fixed width for new column (including line number)

                        let truncate_to = |s: &str, width: usize| -> String {
                            if s.len() > width {
                                format!("{}…", &s[..width.saturating_sub(1)])
                            } else {
                                s.to_string()
                            }
                        };

                        for idx in 0..max_display_lines {
                            let old_line_owned = old_lines.get(idx).cloned().unwrap_or_else(|| "".to_string());
                            let new_line_owned = new_lines.get(idx).cloned().unwrap_or_else(|| "".to_string());

                            let line_num = idx + 1;

                            // Build fixed-width columns with line numbers
                            let old_display = truncate_to(&old_line_owned, left_width.saturating_sub(6)); // leave space for "#### "
                            let new_display = truncate_to(&new_line_owned, right_width.saturating_sub(6));

                            let left_col = format!("{:>4} {}", line_num, old_display);
                            let right_col = format!("{:>4} {}", line_num, new_display);

                            let padded_left = format!("{:<width$}", left_col, width = left_width);
                            let padded_right = format!("{:<width$}", right_col, width = right_width);

                            if old_line_owned != new_line_owned {
                                result.push(Line::from(vec![
                                    Span::styled(padded_left.clone(), Style::default().fg(Color::White).bg(Color::Rgb(40, 20, 20))), // Red bg, white text
                                    Span::styled(" │ ", Style::default().fg(Color::Rgb(60, 60, 60))),
                                    Span::styled(padded_right.clone(), Style::default().fg(Color::White).bg(Color::Rgb(20, 100, 20))), // Green bg, white text
                                ]));
                            } else if !new_line_owned.is_empty() {
                                // Unchanged/new line: show single column with neutral color
                                result.push(Line::from(vec![
                                    Span::styled(format!("{:<width$}", format!("{:>4} {}", line_num, new_display), width = left_width + 3 + right_width), Style::default().fg(Color::White)),
                                ]));
                            }
                        }
                        
                        if max_lines > max_display_lines {
                            result.push(Line::from(Span::styled(
                                format!("  ... {} more lines", max_lines - max_display_lines),
                                    Style::default().fg(Color::DarkGray)
                                )));
                    }
                }
            }
            "file_manager" => {
                if let Some(path) = json.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                    result.push(Line::from(vec![
                            Span::styled("  Path: ", Style::default().fg(Color::Gray)),
                        Span::styled(path.clone(), Style::default().fg(Color::White)),
                    ]));
                        if let Some(kind) = json.get("kind").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                            result.push(Line::from(vec![
                                Span::styled("  Kind: ", Style::default().fg(Color::Gray)),
                                Span::styled(kind, Style::default().fg(Color::White)),
                            ]));
                        }
                        if let Some(start) = json.get("startLine").and_then(|v| v.as_u64()) {
                            let end = json.get("endLine").and_then(|v| v.as_u64()).unwrap_or(start);
                            result.push(Line::from(vec![
                                Span::styled("  Range: ", Style::default().fg(Color::Gray)),
                                Span::styled(format!("{}-{}", start, end), Style::default().fg(Color::White)),
                            ]));
                        }
                        if let Some(overwrite) = json.get("overwrite").and_then(|v| v.as_bool()) {
                            result.push(Line::from(vec![
                                Span::styled("  Overwrite: ", Style::default().fg(Color::Gray)),
                                Span::styled(format!("{}", overwrite), Style::default().fg(Color::White)),
                            ]));
                        }
                        if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
                            result.push(Line::from(vec![Span::raw("")]));
                            result.push(Line::from(vec![
                                Span::styled("  Content:", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                            ]));
                            let content_lines: Vec<&str> = content.lines().take(8).collect();
                            for line in content_lines {
                                result.push(Line::from(vec![
                                    Span::styled("    ", Style::default()),
                                    Span::styled(line.to_string(), Style::default().fg(Color::White)),
                                ]));
                            }
                            if content.lines().count() > 8 {
                                result.push(Line::from(Span::styled(
                                    "    ... (truncated)",
                                    Style::default().fg(Color::DarkGray),
                                )));
                            }
                        }
                    } else if let Some(files) = json.get("files").and_then(|v| v.as_array()) {
                    // files array (batch)
                        result.push(Line::from(vec![
                            Span::styled("  Files:", Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)),
                        ]));
                        for (idx, file) in files.iter().take(5).enumerate() {
                            let path = file.get("path").and_then(|v| v.as_str()).unwrap_or("<path>");
                            let kind = file.get("kind").and_then(|v| v.as_str()).unwrap_or("file");
                            result.push(Line::from(vec![
                                Span::styled(format!("    {}. {}", idx + 1, path), Style::default().fg(Color::White)),
                                Span::styled(format!(" ({})", kind), Style::default().fg(Color::Gray)),
                            ]));
                            if let Some(content) = file.get("content").and_then(|v| v.as_str()) {
                                let sample = content.lines().next().unwrap_or("");
                                result.push(Line::from(vec![
                                    Span::styled("      preview: ", Style::default().fg(Color::Gray)),
                                    Span::styled(sample.to_string(), Style::default().fg(Color::White)),
                                ]));
                            }
                        }
                        if files.len() > 5 {
                            result.push(Line::from(Span::styled(
                                format!("    ... {} more", files.len() - 5),
                                Style::default().fg(Color::DarkGray),
                            )));
                }
            }
        }
            "bash" => {
                if let Some(cmd) = json.get("cmd").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                        result.push(Line::from(vec![
                        Span::styled("  Command: ", Style::default().fg(Color::Gray)),
                        Span::styled(cmd, Style::default().fg(Color::White)),
                    ]));
            }
        }
            "grep" => {
                if let Some(pattern) = json.get("pattern").and_then(|v| v.as_str()) {
                    result.push(Line::from(vec![
                            Span::styled("  Pattern: ", Style::default().fg(Color::Gray)),
                        Span::styled(pattern.to_string(), Style::default().fg(Color::White)),
                    ]));
                }
                if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
                    result.push(Line::from(vec![
                            Span::styled("  Path: ", Style::default().fg(Color::Gray)),
                        Span::styled(path.to_string(), Style::default().fg(Color::White)),
                    ]));
                }
            }
            _ => {
                // For unknown tools, format all JSON keys nicely
                if let Some(obj) = json.as_object() {
                    for (key, value) in obj {
                        let value_str = match value {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => value.to_string(),
                        };
                        let truncated = if value_str.len() > 60 { 
                            format!("{}...", &value_str[..60]) 
                        } else { 
                            value_str 
                        };
                        result.push(Line::from(vec![
                            Span::styled(format!("  {}: ", key), Style::default().fg(Color::Gray)),
                            Span::styled(truncated, Style::default().fg(Color::White)),
                        ]));
                    }
                }
            }
        }
        result
    } else {
        Vec::new()
    }
}

fn render_tool_call_card(name: &str, args: &str, result: &Option<String>, status: &ToolStatus) -> ListItem<'static> {
                let status_style = match status {
                    ToolStatus::Running => Style::default().fg(Color::Yellow),
        ToolStatus::Success => Style::default().fg(Color::Rgb(100, 200, 100)), // Softer green
        ToolStatus::Error => Style::default().fg(Color::Rgb(200, 100, 100)), // Softer red
                };
                let icon = match status {
                    ToolStatus::Running => "⏳",
                    ToolStatus::Success => "✓",
                    ToolStatus::Error => "✗",
                };
                
    // Clean, minimal card design
    let card_bg = Color::Rgb(25, 25, 25); // Slightly lighter for better visibility
    
    // Minimal header - just icon and name, no extra spacing
                let mut lines = vec![
                    Line::from(vec![
            Span::styled(format!("{} ", icon), status_style.add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}", name), status_style.add_modifier(Modifier::BOLD)),
        ]),
    ];
    
    // Add formatted args display - cleaner, more compact
    let args_lines = format_tool_args_display(name, args);
    if !args_lines.is_empty() {
        lines.extend(args_lines);
    }
    
    // Add result if available - minimal, clean display
                if let Some(res) = result {
        if !res.trim().is_empty() {
            // For edit tool, just show success message briefly
            if name == "edit" && status == &ToolStatus::Success {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(res.clone(), Style::default().fg(Color::Rgb(100, 200, 100))),
                ]));
            } else if name != "edit" {
                // For other tools, show result (truncated if too long)
                let result_lines: Vec<&str> = res.lines().take(8).collect();
                for line in result_lines {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(line.to_string(), Style::default().fg(Color::Rgb(200, 200, 200))),
                    ]));
                }
                if res.lines().count() > 8 {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  ... ({} more lines)", res.lines().count() - 8),
                            Style::default().fg(Color::Rgb(100, 100, 100))
                        ),
                    ]));
                }
            }
        }
    }
    
    // Create clean card
    ListItem::new(lines)
        .style(Style::default().bg(card_bg))
}

fn render_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let messages: Vec<ListItem> = app.chat_messages.iter().map(|msg| {
        match msg {
            ChatMessage::User(content) => {
                // Thicker user message with more spacing
                let user_lines = vec![
                    Line::from(vec![Span::raw("")]), // Top padding
                    Line::from(vec![Span::raw("")]),
                    Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::Gray)),
                        Span::styled(content.clone(), Style::default().fg(Color::White)),
                    ]),
                    Line::from(vec![Span::raw("")]), // Bottom padding
                    Line::from(vec![Span::raw("")]),
                ];
                ListItem::new(user_lines)
            }
            ChatMessage::Assistant(content) => {
                // Thicker assistant message with more spacing
                let mut assistant_lines = vec![Line::from(vec![Span::raw("")]), Line::from(vec![Span::raw("")])]; // Top padding
                let content_lines: Vec<Line> = content.lines().map(|l| {
                    Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::raw(l.to_string()),
                    ])
                }).collect();
                assistant_lines.extend(content_lines);
                assistant_lines.push(Line::from(vec![Span::raw("")])); // Bottom padding
                assistant_lines.push(Line::from(vec![Span::raw("")]));
                ListItem::new(assistant_lines)
            }
            ChatMessage::ToolCall { name, args, result, status, .. } => {
                render_tool_call_card(name, args, result, status)
            }
            ChatMessage::Thinking(content) => {
                ListItem::new(Line::from(vec![
                    Span::styled("⚡ ", Style::default().fg(Color::Yellow)),
                    Span::styled(content.clone(), Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)),
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

    let messages_len = messages.len();
    
    // Handle scrolling: only auto-scroll if user hasn't manually scrolled
    if !app.user_scrolled && messages_len > 0 {
        // Auto-scroll to bottom
        app.list_state.select(Some(messages_len.saturating_sub(1)));
    } else if let Some(selected) = app.list_state.selected() {
        // Ensure selected is within bounds when user is scrolling
        if selected >= messages_len && messages_len > 0 {
            app.list_state.select(Some(messages_len.saturating_sub(1)));
        }
    } else if messages_len > 0 {
        // If nothing selected, select last item
        app.list_state.select(Some(messages_len.saturating_sub(1)));
    }
    
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::NONE))
        .highlight_style(Style::default().bg(Color::Rgb(40, 40, 40)));
    
    // Render list - List widget automatically scrolls to show selected item
    f.render_stateful_widget(messages_list, area, &mut app.list_state);

    // Scrollbar - automatically syncs with list state
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scroll_state = app.scroll_state;
    let selected_idx = app.list_state.selected().unwrap_or(0);
    scroll_state = scroll_state.content_length(app.chat_messages.len()).position(selected_idx.saturating_sub(1));
    f.render_stateful_widget(scrollbar, area, &mut scroll_state);
    app.scroll_state = scroll_state;
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let state = format!("{:?}", app.state);
    let title = format!(" Pengy Agent {} │ State: {} ", VERSION, state);
    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD));
    f.render_widget(header, area);
}

fn render_input(f: &mut Frame, app: &mut App, area: Rect) {
    // Thick input area with colored background similar to OpenCode
    let input_bg_color = Color::Rgb(30, 30, 30); // Dark gray for strong contrast
    
    // Fill the background
    let bg_block = Block::default()
        .style(Style::default().bg(input_bg_color));
    f.render_widget(bg_block, area);
    
    // Inner area with minimal padding for comfortable typing
    // Less vertical padding for a tighter, more OpenCode-like feel
    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,  // Reduced top padding
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),  // Reduced bottom padding
    };
    
    let prompt = "> ";
    
    // Calculate available width for input text
    let available_width = inner_area.width.saturating_sub(prompt.len() as u16);
    
    // Wrap text to fit width (character-based, not word-based for code/commands)
    let input_text = &app.chat_input;
    let mut wrapped_lines = Vec::new();
    let mut current_line = String::new();
    
    for ch in input_text.chars() {
        if current_line.len() as u16 >= available_width {
            wrapped_lines.push(current_line);
            current_line = ch.to_string();
        } else {
            current_line.push(ch);
        }
    }
    if !current_line.is_empty() || wrapped_lines.is_empty() {
        wrapped_lines.push(current_line);
    }
    
    // Render input lines - OpenCode style with white text
    let mut input_content = Vec::new();
    
    // Dynamic vertical centering logic
    let content_lines = wrapped_lines.len();
    let available_height = inner_area.height as usize;
    
    // Add vertical padding to center content
    let top_padding = available_height.saturating_sub(content_lines) / 2;
    for _ in 0..top_padding {
        input_content.push(Line::from(vec![Span::raw("")]));
    }
    
    for (i, line) in wrapped_lines.iter().enumerate() {
        let prefix = if i == 0 { prompt } else { "  " };
        input_content.push(Line::from(vec![
            Span::styled(prefix.to_string(), Style::default().fg(Color::Rgb(180, 180, 180)).bg(input_bg_color)), 
            Span::styled(line.clone(), Style::default().fg(Color::White).bg(input_bg_color).add_modifier(Modifier::BOLD)),
        ]));
        // Add spacing only if there's room
        if i < wrapped_lines.len() - 1 && top_padding > 0 {
             // No extra spacing lines when tight
        }
    }
    
    // Render input - NO style override on Paragraph, let spans control colors
    let input_paragraph = Paragraph::new(input_content);
    f.render_widget(input_paragraph, inner_area);

    // Calculate cursor position (adjusted for new inner area)
    let mut char_count = 0;
    let mut cursor_line = 0;
    let mut cursor_col = 0;
    
    for (line_idx, line) in wrapped_lines.iter().enumerate() {
        if char_count + line.len() >= app.input_cursor {
            cursor_line = line_idx;
            cursor_col = app.input_cursor - char_count;
            break;
        }
        char_count += line.len();
    }
    
    if cursor_line >= wrapped_lines.len() && !wrapped_lines.is_empty() {
        cursor_line = wrapped_lines.len() - 1;
        cursor_col = wrapped_lines[cursor_line].len();
    }
    
    let prefix_len = if cursor_line == 0 { prompt.len() } else { 2 };
    let cursor_x = (inner_area.x + prefix_len as u16 + cursor_col as u16)
        .min(inner_area.x + inner_area.width.saturating_sub(1));
    // Account for calculated top padding
    let cursor_y = inner_area.y + top_padding as u16 + cursor_line as u16;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let model_name = app.selected_model.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| "None".to_string());
    let agent_name = format!("{:?}", app.selected_agent);
    let loading = if app.loading { "● Running" } else { "● Idle" };
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .to_string();
    let status_text = format!(" {} │ Model: {} │ Agent: {} │ {} ", cwd, model_name, agent_name, loading);
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Rgb(150, 150, 150)).bg(Color::Black));
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

fn render_chat_sidebar(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);
    
    // OpenCode-style sidebar with modified files
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Title
            Constraint::Min(5),      // Modified files
            Constraint::Length(2),   // Spacing
            Constraint::Length(3),   // Context info
            Constraint::Min(0),      // Fill remaining
        ])
        .split(area);
    
    // Title - elegant like OpenCode
    let title = Paragraph::new("Modified Files")
        .style(Style::default().fg(Color::Rgb(200, 200, 200)).add_modifier(Modifier::BOLD));
    f.render_widget(title, vertical[0]);
    
    // Modified files list
    if !app.modified_files.is_empty() {
        let mut file_items: Vec<Line> = Vec::new();
        for (file_path, (added, removed)) in app.modified_files.iter() {
            // Extract just filename for display
            let filename = std::path::Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path);
            
            let added_str = format!("+{}", added);
            let removed_str = format!("-{}", removed);
            
            file_items.push(Line::from(vec![
                Span::styled(filename.to_string(), Style::default().fg(Color::Rgb(220, 220, 220))),
                Span::raw(" "),
                Span::styled(added_str, Style::default().fg(Color::Rgb(100, 200, 100))), // Green for additions
                Span::raw(" "),
                Span::styled(removed_str, Style::default().fg(Color::Rgb(200, 100, 100))), // Red for removals
            ]));
        }
        
        let files_para = Paragraph::new(file_items)
            .wrap(Wrap { trim: true });
        f.render_widget(files_para, vertical[1]);
    } else {
        let empty_msg = Paragraph::new("No files modified yet")
            .style(Style::default().fg(Color::Rgb(100, 100, 100)))
            .wrap(Wrap { trim: true });
        f.render_widget(empty_msg, vertical[1]);
    }
    
    // Session info - show current session and working directory
    let current_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .to_string_lossy()
        .to_string();
    
    let session_name = app.sessions.get(app.current_session)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    
    let session_lines = vec![
        Line::from(vec![
            Span::styled("Session", Style::default().fg(Color::Rgb(150, 150, 150)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::Rgb(200, 200, 200))),
            Span::styled(session_name, Style::default().fg(Color::Rgb(255, 200, 0))), // Gold for current session
        ]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![
            Span::styled("  ", Style::default().fg(Color::Rgb(120, 120, 120))),
            Span::styled(current_dir, Style::default().fg(Color::Rgb(150, 150, 150))),
        ]),
    ];
    let session_para = Paragraph::new(session_lines)
        .style(Style::default().fg(Color::Rgb(150, 150, 150)))
        .wrap(Wrap { trim: true });
    f.render_widget(session_para, vertical[3]);
}

// Reuse other render functions...
fn render_welcome(f: &mut Frame, app: &App, area: Rect) {
    f.render_widget(Clear, area);
    
    // Create a centered layout - just logo and version
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),  // Top spacing
            Constraint::Length(12),      // Logo + version
            Constraint::Min(0),          // Remaining space
        ])
        .split(area);

    // Logo - centered, elegant
    let logo_lines: Vec<Line> = app.logo.lines().map(|l| Line::from(Span::styled(l, Style::default().fg(Color::White)))).collect();
    let logo = Paragraph::new(logo_lines).alignment(Alignment::Center);
    f.render_widget(logo, vertical[1]);

    // Version below logo - subtle gray
    let version_text = Paragraph::new(VERSION)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(100, 100, 100)));
    let version_area = Rect {
        x: vertical[1].x,
        y: vertical[1].y + vertical[1].height.saturating_sub(1),
        width: vertical[1].width,
        height: 1,
    };
    f.render_widget(version_text, version_area);
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
            } else if app.chat_input.len() > 1 {
                // Filter by what they've typed (case-insensitive)
                let input_lower = app.chat_input.to_lowercase();
                let cmd_lower = c.to_lowercase();
                // Match if command starts with the input (e.g., "/baseurl" starts with "/b")
                cmd_lower.starts_with(&input_lower)
            } else {
                // Single character after "/" - show all
                true
            }
        })
        .map(|(c, d)| {
            ListItem::new(format!("{} - {}", c, d))
        })
        .collect();
    
    if filtered.is_empty() { 
        return; 
    }
    
    // Show more commands when just "/" is typed, limit to 6 for better visibility
    let max_height = if app.chat_input == "/" { 6 } else { 5 };
    let height = filtered.len().min(max_height) as u16;
    
    // Calculate popup position - ensure it's visible above the input area
    // For welcome page with centered input, position hints directly above the input
    let popup_y = if area.y >= height + 3 {
        area.y.saturating_sub(height + 3)
    } else {
        // If not enough space above, show below instead
        area.y + area.height + 2
    };
    
    // Ensure popup doesn't go off-screen
    let popup_y = popup_y.max(1);
    
    let popup_area = Rect { 
        x: area.x, 
        y: popup_y,
        width: area.width.min(60), // Limit width for better visibility
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
    // Use full area with small margins
    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    f.render_widget(Clear, rect);
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Select Model")
        .title_style(Style::default().fg(Color::White));
    
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search field
            Constraint::Min(10),    // Model list
            Constraint::Length(2),  // Footer hints
        ])
        .split(inner);
    
    // Search field
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.model_search_focused { "Search (active)" } else { "Search" })
        .title_style(if app.model_search_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });
    
    let search_text = if app.search_query.is_empty() {
        "Type to search models or providers...".to_string()
    } else {
        app.search_query.clone()
    };
    
    let search_style = if app.search_query.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    
    let search_para = Paragraph::new(search_text)
        .block(search_block)
        .style(search_style);
    f.render_widget(search_para, layout[0]);
    
    // Filter models based on search query - exclude provider entries (they're only in base URL selector)
    let all_models = App::get_available_models();
    let filtered_models: Vec<&ModelOption> = if app.search_query.is_empty() {
        all_models.iter()
            .filter(|m| !m.name.starts_with("Provider:"))
            .collect()
    } else {
        let query_lower = app.search_query.to_lowercase();
        all_models.iter()
            .filter(|m| {
                !m.name.starts_with("Provider:") && (
                    m.name.to_lowercase().contains(&query_lower) ||
                    m.provider.to_lowercase().contains(&query_lower) ||
                    m.base_url.to_lowercase().contains(&query_lower)
                )
            })
            .collect()
    };
    
    // Only update selection if it's out of bounds or not set - don't force it to current model
    // This allows users to navigate and select different models
    if let Some(selected) = app.model_list_state.selected() {
        if selected >= filtered_models.len() {
            // Selection is out of bounds, reset to first item
            app.model_list_state.select(Some(0.max(filtered_models.len().saturating_sub(1))));
        }
    } else if !filtered_models.is_empty() {
        // No selection set, try to find current model or default to first
        let current_selection = if let Some(ref selected_model) = app.selected_model {
            filtered_models.iter().position(|m| m.name == selected_model.name && m.provider == selected_model.provider)
        } else {
            None
        };
        if let Some(idx) = current_selection {
            app.model_list_state.select(Some(idx));
        } else {
            app.model_list_state.select(Some(0));
        }
    }
    
    let items: Vec<ListItem> = filtered_models.iter()
        .map(|m| {
            let display_name = format!("{} ({})", m.name, m.provider);
            ListItem::new(display_name)
        })
        .collect();
    
    let model_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Models ({} found)", filtered_models.len()));
    
    let list = List::new(items)
        .block(model_block)
        .highlight_style(Style::default().fg(Color::Yellow).bg(Color::Rgb(50, 50, 50)));
    
    f.render_stateful_widget(list, layout[1], &mut app.model_list_state);
    
    // Footer hints
    let hints = if app.model_search_focused {
        "Type to search | Tab: switch to list | Esc: back"
    } else {
        "Tab: focus search | ↑/↓: navigate | Enter: select | Esc: back"
    };
    let hints_para = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hints_para, layout[2]);
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

    // Model list - don't force selection, let user navigate freely
    let models = App::get_available_models();
    
    // Only ensure selection is valid (not out of bounds), but don't force it to current model
    if let Some(selected) = app.model_list_state.selected() {
        if selected >= models.len() {
            // Selection out of bounds, reset to first item
            app.model_list_state.select(Some(0));
        }
    } else if !models.is_empty() {
        // No selection set, try to find current model or default to first
        if let Some(ref selected_model) = app.selected_model {
            if let Some(idx) = models.iter().position(|m| m.name == selected_model.name && m.provider == selected_model.provider) {
                app.model_list_state.select(Some(idx));
            } else {
                app.model_list_state.select(Some(0));
            }
        } else {
            app.model_list_state.select(Some(0));
        }
    }
    
    let items: Vec<ListItem> = models
        .iter()
        .map(|m| {
            let is_selected = app.selected_model.as_ref()
                .map(|sm| sm.name == m.name && sm.provider == m.provider)
                .unwrap_or(false);
            
            let caption = if m.provider == "Custom" {
                format!("{} (custom)", m.name)
            } else if m.name.starts_with("Provider:") {
                format!("{} → {}", m.name, m.base_url)
            } else {
                format!("{} - {}", m.name, m.provider)
            };
            
            let style = if is_selected {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            ListItem::new(Line::from(vec![
                Span::styled(if is_selected { "✓ " } else { "  " }, style),
                Span::styled(caption, style),
            ]))
        })
        .collect();

    let model_block = Block::default().borders(Borders::ALL).title("Models (↑/↓ to select)");
    let model_list = List::new(items)
        .block(model_block)
        .highlight_style(Style::default().fg(Color::Yellow).bg(Color::Rgb(50, 50, 50)).add_modifier(Modifier::BOLD));
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

fn render_session_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Sessions (hjkl/↑↓)");
    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let marker = if i == app.current_session { "●" } else { " " };
            ListItem::new(format!("{} {}", marker, s))
        })
        .collect();
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, rect, &mut app.session_list_state);
}

fn render_help(f: &mut Frame, _app: &App, area: Rect) {
    f.render_widget(Clear, area);
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);
    let block = Block::default().borders(Borders::ALL).title("Help");
    let text = "Available Commands:\n\n/models - Select Model\n/agents - Select Agent\n/settings - Configure API key / model / base URL\n/baseurl - Select provider base URL (Mistral, DeepSeek, OpenRouter, etc.)\n/help - Show this help screen\n/clear - Clear conversation and reset agent\n\nNavigation:\nUse Arrows to navigate lists.\nTab to switch between fields/agents.\nEnter to select.\nEsc to go back.\n\nTip: Type '/' in the input to see all available commands with autocomplete.";
    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, rect);
}

fn render_baseurl_selector(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    // Use full area with small margins
    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    f.render_widget(Clear, rect);
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Select Provider Base URL")
        .title_style(Style::default().fg(Color::White));
    
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Search field
            Constraint::Min(10),    // Provider list
            Constraint::Length(2),  // Footer hints
        ])
        .split(inner);
    
    // Search field
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.model_search_focused { "Search (active)" } else { "Search" })
        .title_style(if app.model_search_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });
    
    let search_text = if app.search_query.is_empty() {
        "Type to search providers...".to_string()
    } else {
        app.search_query.clone()
    };
    
    let search_style = if app.search_query.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    
    let search_para = Paragraph::new(search_text)
        .block(search_block)
        .style(search_style);
    f.render_widget(search_para, layout[0]);
    
    // Filter to only provider entries
    let all_models = App::get_available_models();
    let provider_models: Vec<&ModelOption> = all_models.iter()
        .filter(|m| m.name.starts_with("Provider:"))
        .collect();
    
    let filtered_providers: Vec<&ModelOption> = if app.search_query.is_empty() {
        provider_models
    } else {
        let query_lower = app.search_query.to_lowercase();
        provider_models.into_iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower) ||
                m.provider.to_lowercase().contains(&query_lower) ||
                m.base_url.to_lowercase().contains(&query_lower)
            })
            .collect()
    };
    
    // Only update selection if it's out of bounds or not set
    if let Some(selected) = app.model_list_state.selected() {
        if selected >= filtered_providers.len() {
            app.model_list_state.select(Some(0.max(filtered_providers.len().saturating_sub(1))));
        }
    } else if !filtered_providers.is_empty() {
        // Try to find current base URL or default to first
        if let Some(ref selected_model) = app.selected_model {
            if let Some(idx) = filtered_providers.iter().position(|m| m.base_url == selected_model.base_url) {
                app.model_list_state.select(Some(idx));
            } else {
                app.model_list_state.select(Some(0));
            }
        } else {
            app.model_list_state.select(Some(0));
        }
    }
    
    let items: Vec<ListItem> = filtered_providers.iter()
        .map(|m| {
            let display_name = format!("{} → {}", m.name, m.base_url);
            ListItem::new(display_name)
        })
        .collect();
    
    let provider_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Providers ({} found)", filtered_providers.len()));
    
    let list = List::new(items)
        .block(provider_block)
        .highlight_style(Style::default().fg(Color::Yellow).bg(Color::Rgb(50, 50, 50)));
    
    f.render_stateful_widget(list, layout[1], &mut app.model_list_state);
    
    // Footer hints
    let hints = if app.model_search_focused {
        "Type to search | Tab: switch to list | Esc: back"
    } else {
        "Tab: focus search | ↑/↓: navigate | Enter: select | Esc: back"
    };
    let hints_para = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hints_para, layout[2]);
}

fn render_custom_model(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_widget(Clear, area);
    // Use full area with small margins
    let rect = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
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
