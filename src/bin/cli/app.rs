use crate::constants::{CONFIG_FILE, DEFAULT_BASE_URL, EMBED_LOGO};
use pengy_agent::agent::agent::agent::{Agent, AgentEvent};
use pengy_agent::agent::code_researcher::code_researcher::create_code_researcher_agent;
use pengy_agent::agent::coder::coder::create_coder_agent;
use pengy_agent::agent::control_agent::control_agent::create_control_agent;
use pengy_agent::agent::issue_agent::issue_agent::create_issue_agent;
use pengy_agent::agent::pengy_agent::pengy_agent::run_pengy_agent;
use pengy_agent::agent::test_agent::test_agent::create_test_agent;
use pengy_agent::model::model::model::Model;
use ratatui::widgets::{ListState, ScrollbarState};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::{env, error::Error, path::PathBuf};
use tokio::sync::mpsc;

#[derive(Clone, PartialEq, Debug)]
pub enum AppState {
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
pub enum AgentType {
    Coder,
    CodeResearcher,
    TestAgent,
    PengyAgent,
    ControlAgent,
    IssueAgent,
}

#[derive(Clone)]
pub enum ChatMessage {
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
pub enum ToolStatus {
    Running,
    Success,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModelOption {
    pub name: String,
    pub provider: String,
    pub base_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub selected_model: Option<ModelOption>,
}

pub struct App {
    pub(crate) state: AppState,
    pub(crate) api_key: String,
    pub(crate) selected_model: Option<ModelOption>,
    pub(crate) model: Option<Model>,
    pub(crate) agent: Option<Agent>,
    pub(crate) selected_agent: AgentType,
    pub(crate) chat_messages: Vec<ChatMessage>,
    pub(crate) logo: String,
    pub(crate) list_state: ListState,
    pub(crate) scroll_state: ScrollbarState,
    pub(crate) chat_input: String,
    pub(crate) input_cursor: usize,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) model_list_state: ListState,
    pub(crate) agent_list_state: ListState,
    pub(crate) session_list_state: ListState,
    pub(crate) sessions: Vec<String>,
    pub(crate) current_session: usize,
    pub(crate) settings_api_key: String,
    pub(crate) settings_base_url: String,
    pub(crate) settings_field: usize,
    pub(crate) search_query: String,
    pub(crate) model_search_focused: bool,
    pub(crate) show_command_hints: bool,
    pub(crate) custom_model_name: String,
    pub(crate) custom_base_url: String,
    pub(crate) custom_model_field: usize,
    pub(crate) previous_state: Option<AppState>,
    pub(crate) user_scrolled: bool,
    pub(crate) rx: mpsc::UnboundedReceiver<AgentEvent>,
    pub(crate) tx: mpsc::UnboundedSender<AgentEvent>,
    pub(crate) agent_rx: mpsc::UnboundedReceiver<Agent>,
    pub(crate) agent_tx: mpsc::UnboundedSender<Agent>,
    pub(crate) modified_files: HashMap<String, (usize, usize)>,
    pub(crate) pending_tool_calls: Vec<(String, String, String)>,
}

impl App {
    fn load_logo() -> String {
        EMBED_LOGO.to_string()
    }

    fn config_path() -> PathBuf {
        if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(CONFIG_FILE)
        } else {
            PathBuf::from(CONFIG_FILE)
        }
    }

    pub(crate) fn new() -> Result<Self, Box<dyn Error>> {
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
            modified_files: HashMap::new(),
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

    pub(crate) fn save_config(&self) -> Result<(), Box<dyn Error>> {
        let config = Config {
            api_key: self.api_key.clone(),
            selected_model: self.selected_model.clone(),
        };
        let config_json = serde_json::to_string_pretty(&config)?;
        let config_path = Self::config_path();
        std::fs::write(config_path, config_json)?;
        Ok(())
    }

    pub(crate) fn create_new_session(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let name = format!("Session {}", ts);
        self.sessions.push(name);
        self.current_session = self.sessions.len().saturating_sub(1);
        self.session_list_state.select(Some(self.current_session));
        self.chat_messages.clear();
        self.list_state.select(None);
        self.user_scrolled = false;
        self.agent = None;
        self.loading = false;
        self.modified_files.clear();
        let todo_file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy_todo.json");
        let _ = std::fs::remove_file(&todo_file);
    }

    pub(crate) fn get_available_models() -> Vec<ModelOption> {
        vec![
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
            ModelOption {
                name: "z-ai/glm-4.6".to_string(),
                provider: "GLM".to_string(),
                base_url: DEFAULT_BASE_URL.to_string(),
            },
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
            ModelOption {
                name: "Custom Model".to_string(),
                provider: "Custom".to_string(),
                base_url: "".to_string(),
            },
        ]
    }

    pub(crate) fn get_command_hints(&self) -> Vec<(&str, &str)> {
        vec![
            ("/models", "select model"),
            ("/agents", "select agent"),
            ("/settings", "configure API key / model / base URL"),
            ("/baseurl", "select provider base URL (required for custom models)"),
            ("/help", "show help"),
            ("/clear", "clear conversation and reset agent"),
        ]
    }

    pub(crate) fn get_available_agents() -> Vec<(&'static str, &'static str, AgentType)> {
        vec![
            (
                "Coder Agent",
                "Coding agent with tools (bash, edit, grep, todo, web)",
                AgentType::Coder,
            ),
            (
                "Code Researcher",
                "Research codebase with vector search",
                AgentType::CodeResearcher,
            ),
            (
                "Test Agent",
                "Testing agent for code validation",
                AgentType::TestAgent,
            ),
            (
                "Pengy Agent",
                "Meta-agent (orchestrates all three agents)",
                AgentType::PengyAgent,
            ),
            (
                "Control Agent",
                "Git and GitHub control agent (read diff, commit, list issues, create PR)",
                AgentType::ControlAgent,
            ),
            (
                "Issue Agent",
                "Find and publish GitHub issues with cleanup workflow",
                AgentType::IssueAgent,
            ),
        ]
    }

    pub(crate) fn normalize_base_url(base_url: &str) -> String {
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

    pub(crate) fn initialize_agent(&mut self) -> Result<(), Box<dyn Error>> {
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
                let agent = create_coder_agent(model, None, Some(3), Some(50));
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
                let agent = create_test_agent(model, None, Some(3), Some(50));
                self.agent = Some(agent);
            }
            AgentType::ControlAgent => {
                let agent = create_control_agent(model, None, Some(3), Some(50));
                self.agent = Some(agent);
            }
            AgentType::IssueAgent => {
                let agent = create_issue_agent(model, None, Some(3), Some(50));
                self.agent = Some(agent);
            }
        }
        Ok(())
    }

    pub(crate) fn initialize_model(&mut self) -> Result<(), Box<dyn Error>> {
        if self.api_key.is_empty() {
            return Err("API key is required. Use /settings to configure.".into());
        }
        let _model_option = self
            .selected_model
            .as_ref()
            .ok_or("Model not selected. Use /models to select a model.")?;
        self.initialize_agent()?;
        let _ = self.save_config();
        self.state = AppState::Chat;
        Ok(())
    }

    pub(crate) async fn send_message(&mut self) -> Result<(), Box<dyn Error>> {
        if self.chat_input.trim().is_empty() {
            return Ok(());
        }

        let user_input = self.chat_input.clone();
        self.chat_input.clear();
        self.input_cursor = 0;

        self.chat_messages.push(ChatMessage::User(user_input.clone()));
        self.loading = true;
        self.error = None;
        self.user_scrolled = false;

        let tx = self.tx.clone();

        let model_option = self.selected_model.clone();
        let api_key = self.api_key.clone();

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
                        callback,
                    )
                    .await;
                });
            }
            _ => {
                let agent_tx = self.agent_tx.clone();
                if let Some(agent) = self.agent.take() {
                    let mut agent_to_run = agent;

                    tokio::spawn(async move {
                        let callback_tx = tx.clone();
                        let callback = move |event: AgentEvent| {
                            let _ = callback_tx.send(event);
                        };

                        agent_to_run.run(user_input, callback).await;

                        let _ = agent_tx.send(agent_to_run);
                    });
                } else {
                    self.initialize_agent()?;
                    if let Some(mut agent) = self.agent.take() {
                        tokio::spawn(async move {
                            let callback_tx = tx.clone();
                            let callback = move |event: AgentEvent| {
                                let _ = callback_tx.send(event);
                            };
                            agent.run(user_input, callback).await;

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

    pub(crate) fn process_events(&mut self) {
        while let Ok(agent) = self.agent_rx.try_recv() {
            self.agent = Some(agent);
        }

        while let Ok(event) = self.rx.try_recv() {
            match event {
                AgentEvent::Step { .. } => {}
                AgentEvent::ToolCall { tool_name, args } => {
                    let tool_id =
                        format!("tool_{}", self.chat_messages.len() + self.pending_tool_calls.len());
                    self.pending_tool_calls.push((tool_id, tool_name, args));
                }
                AgentEvent::ToolResult { result } => {
                    if let Some((tool_id, name, args_str)) = self.pending_tool_calls.pop() {
                        self.chat_messages.push(ChatMessage::ToolCall {
                            id: tool_id.clone(),
                            name: name.clone(),
                            args: args_str.clone(),
                            result: Some(result.clone()),
                            status: ToolStatus::Success,
                        });

                        if name == "edit" {
                            if let Ok(json_args) =
                                serde_json::from_str::<serde_json::Value>(&args_str)
                            {
                                if let Some(file_path) =
                                    json_args.get("filePath").and_then(|v| v.as_str())
                                {
                                    if let (Some(old_str), Some(new_str)) = (
                                        json_args.get("oldString").and_then(|v| v.as_str()),
                                        json_args.get("newString").and_then(|v| v.as_str()),
                                    ) {
                                        let added = new_str.lines().count();
                                        let removed = old_str.lines().count();
                                        let entry = self
                                            .modified_files
                                            .entry(file_path.to_string())
                                            .or_insert((0, 0));
                                        entry.0 += added;
                                        entry.1 += removed;
                                    }
                                }
                            }
                        }
                    } else {
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
                    if let Some(ChatMessage::ToolCall { status, .. }) = self
                        .chat_messages
                        .iter_mut()
                        .rev()
                        .find(|m| matches!(m, ChatMessage::ToolCall { status: ToolStatus::Running, .. }))
                    {
                        *status = ToolStatus::Error;
                    }
                    self.loading = false;
                }
                AgentEvent::VisionAnalysis { status } => {
                    self.chat_messages
                        .push(ChatMessage::Thinking(format!("👁 {}", status)));
                }
            }
        }
    }
}

