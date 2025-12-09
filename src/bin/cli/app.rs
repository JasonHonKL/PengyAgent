use crate::constants::{CONFIG_FILE, DEFAULT_BASE_URL, EMBED_LOGO};
use crate::theme::{THEMES, Theme};
use pengy_agent::agent::agent::agent::{Agent, AgentEvent};
use pengy_agent::agent::chat_agent::chat_agent::create_chat_agent;
use pengy_agent::agent::code_researcher::code_researcher::create_code_researcher_agent;
use pengy_agent::agent::coder_v2::coder_v2::create_coder_v2_agent;
use pengy_agent::agent::control_agent::control_agent::create_control_agent;
use pengy_agent::agent::issue_agent::issue_agent::create_issue_agent;
use pengy_agent::agent::pengy_agent::pengy_agent::run_pengy_agent;
use pengy_agent::agent::test_agent::test_agent::create_test_agent;
use pengy_agent::model::model::model::Model;
use ratatui::widgets::{ListState, ScrollbarState};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use std::{env, error::Error, fs, fs::OpenOptions, path::PathBuf};
use tokio::sync::mpsc;

const SESSION_DIR: &str = ".pengy/pengy_sessions";
const SESSION_FILE_PREFIX: &str = "session_";
const MAX_TITLE_LEN: usize = 64;

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
    ThemeSelector,
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum AgentType {
    Coder,
    CodeResearcher,
    TestAgent,
    PengyAgent,
    ControlAgent,
    IssueAgent,
    ChatAgent,
}

#[derive(Clone)]
pub enum ChatMessage {
    User(String),
    Assistant(String),
    ToolCall {
        #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    pub theme_index: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct PersistedMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
struct PersistedSession {
    title: String,
    messages: Vec<PersistedMessage>,
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
    pub(crate) scroll_skip_ticks: u8,
    pub(crate) chat_input: String,
    pub(crate) input_cursor: usize,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
    pub(crate) model_list_state: ListState,
    pub(crate) agent_list_state: ListState,
    pub(crate) session_list_state: ListState,
    pub(crate) theme_list_state: ListState,
    pub(crate) sessions: Vec<String>,
    pub(crate) session_paths: Vec<PathBuf>,
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
    pub(crate) session_dirty: bool,
    pub(crate) last_token_usage: Option<(u32, u32, u32)>,
    pub(crate) theme_index: usize,
    pub(crate) theme_search_query: String,
    pub(crate) theme_search_focused: bool,
    pub(crate) rx: mpsc::UnboundedReceiver<AgentEvent>,
    pub(crate) tx: mpsc::UnboundedSender<AgentEvent>,
    pub(crate) agent_rx: mpsc::UnboundedReceiver<Agent>,
    pub(crate) agent_tx: mpsc::UnboundedSender<Agent>,
    pub(crate) sandbox_enabled: bool,
    pub(crate) sandbox_branch: Option<String>,
    pub(crate) sandbox_base_branch: Option<String>,
    pub(crate) sandbox_commit_count: u32,
    pub(crate) modified_files: HashMap<String, (usize, usize)>,
    pub(crate) pending_tool_calls: Vec<PendingToolCall>,
}

#[derive(Clone)]
pub(crate) struct PendingToolCall {
    pub id: String,
    pub name: String,
    pub args: String,
    pub message_index: usize,
}

impl App {
    fn load_logo() -> String {
        EMBED_LOGO.to_string()
    }

    pub(crate) fn current_theme(&self) -> Theme {
        THEMES
            .get(self.theme_index % THEMES.len())
            .cloned()
            .unwrap_or_else(|| THEMES[0].clone())
    }

    pub(crate) fn filtered_themes(&self) -> Vec<(usize, &'static str)> {
        THEMES
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                if self.theme_search_query.is_empty() {
                    true
                } else {
                    t.name
                        .to_lowercase()
                        .contains(&self.theme_search_query.to_lowercase())
                }
            })
            .map(|(i, t)| (i, t.name))
            .collect()
    }

    fn session_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(SESSION_DIR)
    }

    fn ensure_session_dir() -> PathBuf {
        let dir = Self::session_dir();
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn truncate_title(input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.len() <= MAX_TITLE_LEN {
            return trimmed.to_string();
        }
        trimmed
            .chars()
            .take(MAX_TITLE_LEN)
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn session_file_from_title(title: &str) -> PathBuf {
        let sanitized: String = title
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::session_dir().join(format!("{}{}_{}.json", SESSION_FILE_PREFIX, ts, sanitized))
    }

    fn chat_to_persist(chat: &[ChatMessage]) -> Vec<PersistedMessage> {
        chat.iter()
            .map(|m| match m {
                ChatMessage::User(t) => PersistedMessage {
                    role: "user".to_string(),
                    content: t.clone(),
                },
                ChatMessage::Assistant(t) => PersistedMessage {
                    role: "assistant".to_string(),
                    content: t.clone(),
                },
                ChatMessage::ToolCall {
                    name, args, result, ..
                } => {
                    let content = if let Some(r) = result {
                        format!("[tool {}]\nargs: {}\nresult: {}", name, args, r)
                    } else {
                        format!("[tool {}]\nargs: {}", name, args)
                    };
                    PersistedMessage {
                        role: "assistant".to_string(),
                        content,
                    }
                }
                ChatMessage::Thinking(t) => PersistedMessage {
                    role: "assistant".to_string(),
                    content: t.clone(),
                },
                ChatMessage::Error(t) => PersistedMessage {
                    role: "assistant".to_string(),
                    content: format!("Error: {}", t),
                },
            })
            .collect()
    }

    fn persist_to_chat(msgs: &[PersistedMessage]) -> Vec<ChatMessage> {
        msgs.iter()
            .map(|m| match m.role.as_str() {
                "user" => ChatMessage::User(m.content.clone()),
                _ => ChatMessage::Assistant(m.content.clone()),
            })
            .collect()
    }

    fn write_session_file(path: &PathBuf, title: &str, messages: &[ChatMessage]) {
        let dir = Self::ensure_session_dir();
        let _ = fs::create_dir_all(&dir);
        let data = PersistedSession {
            title: title.to_string(),
            messages: Self::chat_to_persist(messages),
        };
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = fs::write(path, json);
        }
    }

    fn read_session_file(path: &PathBuf) -> Option<(String, Vec<ChatMessage>)> {
        let content = fs::read_to_string(path).ok()?;
        let parsed: PersistedSession = serde_json::from_str(&content).ok()?;
        let chat = Self::persist_to_chat(&parsed.messages);
        Some((parsed.title, chat))
    }

    fn load_sessions_from_disk() -> (Vec<String>, Vec<PathBuf>) {
        let dir = Self::ensure_session_dir();
        let mut entries: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&dir) {
            for entry in read_dir.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if entry
                        .path()
                        .extension()
                        .map(|e| e == "json")
                        .unwrap_or(false)
                    {
                        let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        entries.push((modified, entry.path()));
                    }
                }
            }
        }

        // Sort newest first
        entries.sort_by(|a, b| b.0.cmp(&a.0));

        let mut titles = Vec::new();
        let mut paths = Vec::new();

        for (_, path) in entries.iter() {
            if let Some((title, _messages)) = Self::read_session_file(&path.to_path_buf()) {
                titles.push(title);
                paths.push(path.to_path_buf());
            }
        }

        (titles, paths)
    }

    pub(crate) fn save_current_session(&mut self) {
        if let (Some(title), Some(path)) = (
            self.sessions.get(self.current_session),
            self.session_paths.get(self.current_session),
        ) {
            Self::write_session_file(path, title, &self.chat_messages);
            self.session_dirty = false;
        }
    }

    pub(crate) fn load_session(&mut self, idx: usize) {
        if let Some(path) = self.session_paths.get(idx).cloned() {
            if let Some((title, messages)) = Self::read_session_file(&path) {
                if let Some(slot) = self.sessions.get_mut(idx) {
                    *slot = title;
                }
                self.chat_messages = messages;
                self.current_session = idx;
                self.session_list_state.select(Some(idx));
                self.list_state.select(None);
                self.user_scrolled = false;
                self.reset_sandbox_state();
                self.session_dirty = false;
            }
        }
    }

    fn maybe_update_session_title(&mut self, user_input: &str) {
        if self.chat_messages.is_empty() {
            let new_title = Self::truncate_title(user_input);
            if let Some(title_slot) = self.sessions.get_mut(self.current_session) {
                *title_slot = new_title.clone();
            }
            self.session_dirty = true;
        }
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
        let theme_index = config.theme_index.unwrap_or(0);

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut model_list_state = ListState::default();
        model_list_state.select(Some(0));

        let mut agent_list_state = ListState::default();
        agent_list_state.select(Some(0));

        let mut theme_list_state = ListState::default();
        theme_list_state.select(Some(theme_index.min(THEMES.len().saturating_sub(1))));

        let (tx, rx) = mpsc::unbounded_channel();
        let (agent_tx, agent_rx) = mpsc::unbounded_channel();

        let (custom_model_name, custom_base_url) = if let Some(ref m) = selected_model {
            if m.provider == "Custom" {
                (m.name.clone(), m.base_url.clone())
            } else {
                (String::new(), String::new())
            }
        } else {
            (String::new(), String::new())
        };

        let settings_base_url = selected_model
            .as_ref()
            .map(|m| m.base_url.clone())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let todo_file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy_todo.json");
        let _ = std::fs::remove_file(&todo_file);

        let initial_messages: Vec<ChatMessage> = Vec::new();
        let (sessions, session_paths) = Self::load_sessions_from_disk();

        let mut app = Self {
            state: AppState::Welcome,
            api_key: api_key.clone(),
            selected_model,
            model: None,
            agent: None,
            selected_agent: AgentType::Coder,
            chat_messages: initial_messages,
            logo,
            list_state,
            scroll_state: ScrollbarState::default(),
            scroll_skip_ticks: 0,
            chat_input: String::new(),
            input_cursor: 0,
            loading: false,
            error: None,
            model_list_state,
            agent_list_state,
            theme_list_state,
            session_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            sessions,
            session_paths,
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
            session_dirty: false,
            last_token_usage: None,
            theme_index,
            theme_search_query: String::new(),
            theme_search_focused: false,
            rx,
            tx,
            agent_rx,
            agent_tx,
            sandbox_enabled: false,
            sandbox_branch: None,
            sandbox_base_branch: None,
            sandbox_commit_count: 0,
            modified_files: HashMap::new(),
            pending_tool_calls: Vec::new(),
        };

        // Always start with a fresh session; existing sessions are available via selector.
        app.create_new_session();

        Ok(app)
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
            theme_index: Some(0),
        }
    }

    pub(crate) fn save_config(&self) -> Result<(), Box<dyn Error>> {
        let config = Config {
            api_key: self.api_key.clone(),
            selected_model: self.selected_model.clone(),
            theme_index: Some(self.theme_index),
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
        let path = Self::session_file_from_title(&name);
        self.sessions.push(name.clone());
        self.session_paths.push(path.clone());
        self.current_session = self.sessions.len().saturating_sub(1);
        self.session_list_state.select(Some(self.current_session));
        self.chat_messages.clear();
        self.list_state.select(None);
        self.user_scrolled = false;
        self.agent = None;
        self.loading = false;
        self.reset_sandbox_state();
        self.modified_files.clear();
        let todo_file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy_todo.json");
        let _ = std::fs::remove_file(&todo_file);
        Self::write_session_file(&path, &name, &[]);
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
                name: "Provider: Ollama".to_string(),
                provider: "Ollama".to_string(),
                base_url: "http://localhost:11434/v1".to_string(),
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
            ("/sessions", "switch session"),
            ("/new", "create new session"),
            ("/theme", "cycle theme"),
            ("/settings", "configure API key / model / base URL"),
            (
                "/baseurl",
                "select provider base URL (required for custom models)",
            ),
            ("/help", "show help"),
            ("/clear", "clear conversation and reset agent"),
            ("/sandbox", "enable sandbox (auto-commit; merge with /save)"),
            ("/save", "merge sandbox branch and return to base branch"),
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
                "Chat Agent (read-only)",
                "Conversational agent that never modifies code; read-only tools",
                AgentType::ChatAgent,
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

    pub(crate) fn reset_custom_model_fields(&mut self) {
        self.custom_model_name.clear();
        // Default the custom base URL to whatever the user currently has configured
        // so the custom model screen doesn't need to ask for it explicitly.
        self.custom_base_url = if self.settings_base_url.is_empty() {
            DEFAULT_BASE_URL.to_string()
        } else {
            self.settings_base_url.clone()
        };
        self.custom_model_field = 0;
        self.error = None;
    }

    pub(crate) fn normalize_base_url(base_url: &str) -> String {
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let mut normalized = trimmed.trim_end_matches('/').to_string();

        // Auto-prepend https:// when user omits scheme (common for custom URLs)
        if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
            normalized = format!("https://{}", normalized);
        }

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

    fn reset_sandbox_state(&mut self) {
        self.sandbox_enabled = false;
        self.sandbox_branch = None;
        self.sandbox_base_branch = None;
        self.sandbox_commit_count = 0;
    }

    fn sanitize_branch_name(name: &str) -> String {
        let lowered = name.to_ascii_lowercase();
        let mut cleaned: String = lowered
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c
                } else if c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        while cleaned.contains("--") {
            cleaned = cleaned.replace("--", "-");
        }
        let trimmed = cleaned.trim_matches('-');
        if trimmed.is_empty() {
            "session".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn run_git_command(args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .map_err(|e| format!("failed to run git {}: {}", args.join(" "), e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                Ok(stdout)
            } else if stdout.is_empty() {
                Ok(stderr)
            } else {
                Ok(format!("{}\n{}", stdout, stderr).trim().to_string())
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let msg = if stderr.is_empty() { stdout } else { stderr };
            Err(if msg.is_empty() {
                format!("git {} failed", args.join(" "))
            } else {
                msg
            })
        }
    }

    fn ensure_git_repo() -> Result<(), String> {
        let inside = Self::run_git_command(&["rev-parse", "--is-inside-work-tree"])?;
        if inside.trim() == "true" {
            Ok(())
        } else {
            Err("Not a git repository; sandbox mode requires git.".to_string())
        }
    }

    fn current_git_branch() -> Result<String, String> {
        let branch = Self::run_git_command(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if branch.trim().is_empty() || branch.trim() == "HEAD" {
            Err("Unable to determine current branch (detached HEAD)".to_string())
        } else {
            Ok(branch.trim().to_string())
        }
    }

    fn git_branch_exists(branch: &str) -> Result<bool, String> {
        let status = Command::new("git")
            .args([
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", branch),
            ])
            .status()
            .map_err(|e| format!("failed to check branch existence: {}", e))?;
        Ok(status.success())
    }

    pub(crate) fn enable_sandbox_mode(&mut self) -> Result<String, String> {
        if self.sandbox_enabled {
            if let Some(branch) = &self.sandbox_branch {
                return Ok(format!(
                    "Sandbox already enabled on branch {}. Use /save to merge.",
                    branch
                ));
            }
        }

        Self::ensure_git_repo()?;
        let base_branch = Self::current_git_branch()?;
        let session_name = self
            .sessions
            .get(self.current_session)
            .cloned()
            .unwrap_or_else(|| "session".to_string());
        let sandbox_branch = format!("_{}", Self::sanitize_branch_name(&session_name));

        if base_branch == sandbox_branch {
            return Err(format!(
                "Current branch ({}) is already the sandbox branch. Switch to a base branch before enabling sandbox or run /save if you meant to merge.",
                base_branch
            ));
        }

        if Self::git_branch_exists(&sandbox_branch)? {
            Self::run_git_command(&["checkout", &sandbox_branch])?;
        } else {
            Self::run_git_command(&["checkout", "-b", &sandbox_branch])?;
        }

        self.sandbox_enabled = true;
        self.sandbox_branch = Some(sandbox_branch.clone());
        self.sandbox_base_branch = Some(base_branch.clone());
        self.sandbox_commit_count = 0;

        Ok(format!(
            "Sandbox enabled on {} (base {}). Auto-commits will run after each agent response. Use /save to merge and return to {}.",
            sandbox_branch, base_branch, base_branch
        ))
    }

    pub(crate) fn disable_sandbox_mode(&mut self) -> Result<String, String> {
        if !self.sandbox_enabled {
            return Ok("Sandbox mode is already off.".to_string());
        }

        Self::ensure_git_repo()?;
        let base_branch = self
            .sandbox_base_branch
            .clone()
            .unwrap_or_else(|| Self::current_git_branch().unwrap_or_else(|_| "main".to_string()));

        if Self::current_git_branch().unwrap_or_default() != base_branch {
            let _ = Self::run_git_command(&["checkout", &base_branch]);
        }

        let sandbox_branch = self.sandbox_branch.clone();
        self.reset_sandbox_state();

        Ok(format!(
            "Sandbox disabled. Switched back to {}{}.",
            base_branch,
            sandbox_branch
                .map(|b| format!(" (sandbox branch was {})", b))
                .unwrap_or_default()
        ))
    }

    pub(crate) fn maybe_auto_commit_sandbox(&mut self) -> Result<Option<String>, String> {
        if !self.sandbox_enabled {
            return Ok(None);
        }

        Self::ensure_git_repo()?;
        let sandbox_branch = self
            .sandbox_branch
            .clone()
            .ok_or_else(|| "Sandbox branch not set".to_string())?;

        let current_branch = Self::current_git_branch()?;
        if current_branch != sandbox_branch {
            Self::run_git_command(&["checkout", &sandbox_branch])?;
        }

        let status = Self::run_git_command(&["status", "--porcelain"])?;
        if status.trim().is_empty() {
            return Ok(None);
        }

        Self::run_git_command(&["add", "-A"])?;
        self.sandbox_commit_count += 1;
        let commit_message = format!(
            "sandbox auto-commit #{} ({})",
            self.sandbox_commit_count, sandbox_branch
        );
        Self::run_git_command(&["commit", "-m", &commit_message])?;
        self.modified_files.clear();

        Ok(Some(format!(
            "Sandbox commit saved to {} ({})",
            sandbox_branch, commit_message
        )))
    }

    pub(crate) fn save_sandbox_changes(&mut self) -> Result<String, String> {
        if !self.sandbox_enabled {
            return Err("Sandbox mode is not enabled. Use /sandbox to enable it.".to_string());
        }

        Self::ensure_git_repo()?;
        let sandbox_branch = self
            .sandbox_branch
            .clone()
            .ok_or_else(|| "Sandbox branch missing".to_string())?;
        let base_branch = self
            .sandbox_base_branch
            .clone()
            .unwrap_or_else(|| Self::current_git_branch().unwrap_or_else(|_| "main".to_string()));

        if sandbox_branch == base_branch {
            return Err(
                "Sandbox branch matches the base branch; switch to a different branch before saving."
                    .to_string(),
            );
        }

        if let Err(e) = self.maybe_auto_commit_sandbox() {
            return Err(format!(
                "Could not commit sandbox changes before saving: {}",
                e
            ));
        }

        Self::run_git_command(&["checkout", &base_branch])?;
        match Self::run_git_command(&["merge", "--no-ff", &sandbox_branch]) {
            Ok(_) => {
                self.reset_sandbox_state();
                Ok(format!(
                    "Merged {} into {} and returned to {}. Sandbox mode is now off.",
                    sandbox_branch, base_branch, base_branch
                ))
            }
            Err(err) => {
                self.reset_sandbox_state();
                Err(format!(
                    "Failed to merge {} into {}: {}. Resolve conflicts manually or retry.",
                    sandbox_branch, base_branch, err
                ))
            }
        }
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
                let agent = create_coder_v2_agent(model, None, Some(3), Some(50));
                self.agent = Some(agent);
            }
            AgentType::ChatAgent => {
                let agent = create_chat_agent(model, None, Some(3), Some(50));
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
        self.maybe_update_session_title(&user_input);
        self.chat_input.clear();
        self.input_cursor = 0;

        self.chat_messages
            .push(ChatMessage::User(user_input.clone()));
        self.session_dirty = true;
        self.loading = true;
        self.error = None;
        self.user_scrolled = false;
        self.last_token_usage = None;

        let tx = self.tx.clone();

        let model_option = self.selected_model.clone();
        let api_key = self.api_key.clone();

        // Build a lightweight conversation history for Pengy (last 20 user/assistant messages)
        let conversation_history = {
            let mut entries = Vec::new();
            let take_n = 20;
            let start = self.chat_messages.len().saturating_sub(take_n);
            for msg in self.chat_messages.iter().skip(start) {
                match msg {
                    ChatMessage::User(text) => entries.push(format!("User: {}", text)),
                    ChatMessage::Assistant(text) => entries.push(format!("Assistant: {}", text)),
                    _ => {}
                }
            }
            if entries.is_empty() {
                None
            } else {
                Some(entries.join("\n"))
            }
        };

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
                        conversation_history,
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
        self.save_current_session();
        Ok(())
    }

    pub(crate) fn process_events(&mut self) {
        let mut changed = false;
        while let Ok(agent) = self.agent_rx.try_recv() {
            self.agent = Some(agent);
        }

        while let Ok(event) = self.rx.try_recv() {
            match event {
                AgentEvent::Step { .. } => {}
                AgentEvent::ToolCall { tool_name, args } => {
                    let tool_id = format!(
                        "tool_{}",
                        self.chat_messages.len() + self.pending_tool_calls.len()
                    );
                    let message_index = self.chat_messages.len();
                    self.chat_messages.push(ChatMessage::ToolCall {
                        id: tool_id.clone(),
                        name: tool_name.clone(),
                        args: args.clone(),
                        result: None,
                        status: ToolStatus::Running,
                    });
                    self.log_event("tool_call", &format!("{} | args: {}", tool_name, args));
                    self.pending_tool_calls.push(PendingToolCall {
                        id: tool_id,
                        name: tool_name,
                        args,
                        message_index,
                    });
                    changed = true;
                }
                AgentEvent::ToolResult { result } => {
                    let result_clone = result.clone();
                    if let Some(pending) = self.pending_tool_calls.pop() {
                        if let Some(ChatMessage::ToolCall {
                            result: existing_result,
                            status,
                            ..
                        }) = self.chat_messages.get_mut(pending.message_index)
                        {
                            *existing_result = Some(result_clone.clone());
                            *status = ToolStatus::Success;
                        } else {
                            self.chat_messages.push(ChatMessage::ToolCall {
                                id: pending.id.clone(),
                                name: pending.name.clone(),
                                args: pending.args.clone(),
                                result: Some(result_clone.clone()),
                                status: ToolStatus::Success,
                            });
                        }
                        changed = true;

                        // Track modified files for edit and edit_file tools
                        if pending.name == "edit" || pending.name == "edit_file" {
                            if let Ok(json_args) =
                                serde_json::from_str::<serde_json::Value>(&pending.args)
                            {
                                if let Some(file_path) =
                                    json_args.get("filePath").and_then(|v| v.as_str())
                                {
                                    // Just mark the file as modified, don't try to count lines accurately
                                    self.modified_files
                                        .entry(file_path.to_string())
                                        .or_insert((1, 0));
                                }
                            }
                        }
                        // Track file_manager writes
                        if pending.name == "file_manager" {
                            if let Ok(json_args) =
                                serde_json::from_str::<serde_json::Value>(&pending.args)
                            {
                                if let Some(op) =
                                    json_args.get("operation").and_then(|v| v.as_str())
                                {
                                    if op == "write" || op == "create" {
                                        if let Some(path) =
                                            json_args.get("path").and_then(|v| v.as_str())
                                        {
                                            self.modified_files
                                                .entry(path.to_string())
                                                .or_insert((1, 0));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        self.chat_messages.push(ChatMessage::ToolCall {
                            id: format!("tool_{}", self.chat_messages.len()),
                            name: "unknown".to_string(),
                            args: String::new(),
                            result: Some(result_clone.clone()),
                            status: ToolStatus::Success,
                        });
                        changed = true;
                    }
                    self.log_event("tool_result", &result_clone);
                }
                AgentEvent::TokenUsage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                } => {
                    self.last_token_usage = Some((
                        prompt_tokens.unwrap_or(0),
                        completion_tokens.unwrap_or(0),
                        total_tokens.unwrap_or(0),
                    ));
                }
                AgentEvent::Thinking { content } => {
                    self.chat_messages
                        .push(ChatMessage::Thinking(content.clone()));
                    self.log_event("thinking", &content);
                    changed = true;
                }
                AgentEvent::FinalResponse { content } => {
                    self.chat_messages
                        .push(ChatMessage::Assistant(content.clone()));
                    self.log_event("assistant", &content);
                    self.loading = false;
                    changed = true;
                    if self.sandbox_enabled {
                        match self.maybe_auto_commit_sandbox() {
                            Ok(Some(msg)) => {
                                self.chat_messages.push(ChatMessage::Assistant(msg));
                                changed = true;
                            }
                            Ok(None) => {}
                            Err(err) => {
                                self.chat_messages
                                    .push(ChatMessage::Error(format!("[sandbox] {}", err)));
                                changed = true;
                            }
                        }
                    }
                }
                AgentEvent::Error { error } => {
                    self.chat_messages.push(ChatMessage::Error(error.clone()));
                    self.log_event("error", &error);
                    changed = true;
                    if let Some(ChatMessage::ToolCall { status, .. }) =
                        self.chat_messages.iter_mut().rev().find(|m| {
                            matches!(
                                m,
                                ChatMessage::ToolCall {
                                    status: ToolStatus::Running,
                                    ..
                                }
                            )
                        })
                    {
                        *status = ToolStatus::Error;
                    }
                    self.loading = false;
                }
                AgentEvent::VisionAnalysis { status } => {
                    self.chat_messages
                        .push(ChatMessage::Thinking(format!("[vision] {}", status)));
                    changed = true;
                }
            }
        }

        if changed {
            self.session_dirty = true;
            // Auto-scroll to bottom on new messages/events
            self.user_scrolled = false;
            if !self.chat_messages.is_empty() {
                let last = self.chat_messages.len().saturating_sub(1);
                self.list_state.select(Some(last));
                self.scroll_state = self.scroll_state.position(last);
            }
            self.save_current_session();
        }
    }

    fn log_event(&self, role: &str, content: &str) {
        if content.is_empty() {
            return;
        }
        let session_name = self
            .sessions
            .get(self.current_session)
            .cloned()
            .unwrap_or_else(|| "session".to_string());
        let sanitized: String = session_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        // Write logs to .pengy directory for low-level capture.
        let file = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".pengy")
            .join(format!("pengy_json_{}.json", sanitized));
        let _ = fs::create_dir_all(file.parent().unwrap_or(&std::path::PathBuf::from(".pengy")));
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let payload = serde_json::json!({
            "ts": ts,
            "session": session_name,
            "role": role,
            "content": content,
        });
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(file) {
            let _ = writeln!(f, "{}", payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    #[test]
    fn sanitize_branch_name_normalizes_and_prefixes_are_stable() {
        assert_eq!(App::sanitize_branch_name("Session 123"), "session-123");
        assert_eq!(App::sanitize_branch_name("My Project!"), "my-project");
        assert_eq!(App::sanitize_branch_name("___weird___name___"), "___weird___name___");
        assert_eq!(App::sanitize_branch_name(""), "session");
    }
}
