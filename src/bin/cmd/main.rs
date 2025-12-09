use pengy_agent::agent::agent::agent::AgentEvent;
use pengy_agent::agent::chat_agent::chat_agent::create_chat_agent;
use pengy_agent::agent::code_researcher::code_researcher::create_code_researcher_agent;
use pengy_agent::agent::coder_v2::coder_v2::create_coder_v2_agent;
use pengy_agent::agent::control_agent::control_agent::create_control_agent;
use pengy_agent::agent::issue_agent::issue_agent::create_issue_agent;
use pengy_agent::agent::pengy_agent::pengy_agent::run_pengy_agent;
use pengy_agent::agent::test_agent::test_agent::create_test_agent;
use pengy_agent::model::model::model::Model;
use std::env;
use std::error::Error;

const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone)]
enum AgentType {
    Coder,
    CodeResearcher,
    TestAgent,
    PengyAgent,
    ControlAgent,
    IssueAgent,
    ChatAgent,
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut normalized = trimmed.trim_end_matches('/').to_string();

    // Auto-prepend https:// when user omits scheme
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

fn parse_agent_type(agent_str: &str) -> Result<AgentType, String> {
    match agent_str.to_lowercase().as_str() {
        "coder" | "coder-agent" => Ok(AgentType::Coder),
        "code-researcher" | "researcher" | "code_researcher" => Ok(AgentType::CodeResearcher),
        "test-agent" | "test" | "test_agent" => Ok(AgentType::TestAgent),
        "pengy-agent" | "pengy" | "pengy_agent" => Ok(AgentType::PengyAgent),
        "control-agent" | "control" | "control_agent" => Ok(AgentType::ControlAgent),
        "issue-agent" | "issue" | "issue_agent" => Ok(AgentType::IssueAgent),
        "chat-agent" | "chat" | "chat_agent" => Ok(AgentType::ChatAgent),
        _ => Err(format!(
            "Unknown agent type: {}. Available: coder, code-researcher, test-agent, pengy-agent, control-agent, issue-agent, chat-agent",
            agent_str
        )),
    }
}

fn parse_args() -> Result<(String, AgentType, String, String, Option<String>, bool), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Err("Missing required arguments".into());
    }

    let mut api_key = None;
    let mut model = None;
    let mut prompt = None;
    let mut agent = None;
    let mut base_url = None;
    let mut yolo = false;

    for arg in args.iter().skip(1) {
        if arg == "--yolo" {
            yolo = true;
            continue;
        }

        if let Some((key, value)) = arg.split_once('=') {
            match key {
                "--apikey" | "--api-key" => {
                    api_key = Some(value.to_string());
                }
                "--model" => {
                    model = Some(value.to_string());
                }
                "--prompt" => {
                    prompt = Some(value.to_string());
                }
                "--agent" => {
                    agent = Some(value.to_string());
                }
                "--base-url" | "--baseurl" => {
                    base_url = Some(value.to_string());
                }
                _ => {
                    eprintln!("Warning: Unknown argument: {}", key);
                }
            }
        } else if arg.starts_with("--") {
            // Handle flags without values
            match arg.as_str() {
                "--yolo" => yolo = true,
                _ => {
                    eprintln!("Warning: Unknown flag: {}", arg);
                }
            }
        }
    }

    let api_key = api_key.ok_or("Missing required argument: --apikey=")?;
    let model = model.ok_or("Missing required argument: --model=")?;
    let prompt = prompt.ok_or("Missing required argument: --prompt=")?;
    let agent_str = agent.unwrap_or_else(|| "coder".to_string());
    let agent_type = parse_agent_type(&agent_str)?;

    Ok((api_key, agent_type, model, prompt, base_url, yolo))
}

fn print_usage() {
    eprintln!("\nUsage: pengy-cmd --apikey=<key> --model=<model> --prompt=\"<prompt>\" [options]");
    eprintln!("\nRequired arguments:");
    eprintln!("  --apikey=<key>          API key for the model provider");
    eprintln!("  --model=<model>         Model name (e.g., openai/gpt-4o)");
    eprintln!("  --prompt=\"<prompt>\"    The prompt/question for the agent");
    eprintln!("\nOptional arguments:");
    eprintln!("  --agent=<type>          Agent type (default: coder)");
    eprintln!("                         Available: coder, code-researcher, test-agent, pengy-agent, control-agent, issue-agent, chat-agent");
    eprintln!("  --base-url=<url>        Custom base URL (default: https://openrouter.ai/api/v1)");
    eprintln!("  --yolo                  Auto-approve all actions (always yes)");
    eprintln!("\nExamples:");
    eprintln!("  pengy-cmd --apikey=sk-... --model=openai/gpt-4o --prompt=\"Write hello world\"");
    eprintln!("  pengy-cmd --apikey=sk-... --model=openai/gpt-4o --prompt=\"Research codebase\" --agent=code-researcher --yolo");
    eprintln!("  pengy-cmd --apikey=sk-... --model=custom/model --prompt=\"Do something\" --base-url=https://api.example.com/v1");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (api_key, agent_type, model_name, prompt, custom_base_url, yolo) = parse_args()?;

    let base_url = if let Some(url) = custom_base_url {
        normalize_base_url(&url)
    } else {
        DEFAULT_BASE_URL.to_string()
    };

    let model = Model::new(model_name.clone(), api_key.clone(), base_url.clone());

    if yolo {
        eprintln!("[YOLO MODE] Auto-approving all actions");
    }

    eprintln!("Running agent in non-interactive mode...");
    eprintln!("Agent: {:?}", agent_type);
    eprintln!("Model: {}", model_name);
    eprintln!("Base URL: {}", base_url);
    eprintln!("Prompt: {}\n", prompt);

    let callback = |event: AgentEvent| {
        match event {
            AgentEvent::Step { step, max_steps } => {
                eprintln!("[Step {}/{}]", step, max_steps);
            }
            AgentEvent::ToolCall { tool_name, args } => {
                eprintln!("[Tool Call] {} with args: {}", tool_name, args);
            }
            AgentEvent::ToolResult { result } => {
                // Truncate very long results for readability
                let display_result = if result.len() > 500 {
                    format!("{}... (truncated)", &result[..500])
                } else {
                    result.clone()
                };
                eprintln!("[Tool Result] {}", display_result);
            }
            AgentEvent::TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            } => {
                eprintln!(
                    "[Usage] prompt: {:?}, completion: {:?}, total: {:?}",
                    prompt_tokens, completion_tokens, total_tokens
                );
            }
            AgentEvent::Thinking { content } => {
                eprintln!("[Thinking] {}", content);
            }
            AgentEvent::FinalResponse { content } => {
                println!("\n{}", content);
            }
            AgentEvent::Error { error } => {
                eprintln!("[Error] {}", error);
            }
            AgentEvent::VisionAnalysis { status } => {
                eprintln!("[Vision] {}", status);
            }
        }
    };

    match agent_type {
        AgentType::PengyAgent => {
            let _ = run_pengy_agent(
                model,
                api_key,
                base_url,
                Some("openai/text-embedding-3-small".to_string()),
                prompt,
                None,
                Some(3),
                Some(50),
                callback,
            )
            .await;
        }
        AgentType::Coder => {
            let mut agent = create_coder_v2_agent(model, None, Some(3), Some(50));
            agent.run(prompt, callback).await;
        }
        AgentType::ChatAgent => {
            let mut agent = create_chat_agent(model, None, Some(3), Some(50));
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
            let mut agent = create_test_agent(model, None, Some(3), Some(50));
            agent.run(prompt, callback).await;
        }
        AgentType::ControlAgent => {
            let mut agent = create_control_agent(model, None, Some(3), Some(50));
            agent.run(prompt, callback).await;
        }
        AgentType::IssueAgent => {
            let mut agent = create_issue_agent(model, None, Some(3), Some(50));
            agent.run(prompt, callback).await;
        }
    }

    Ok(())
}

