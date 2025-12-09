use crate::app::{AgentType, App};
use crate::constants::DEFAULT_BASE_URL;
use pengy_agent::agent::agent::agent::AgentEvent;
use pengy_agent::agent::code_researcher::code_researcher::create_code_researcher_agent;
use pengy_agent::agent::chat_agent::chat_agent::create_chat_agent;
use pengy_agent::agent::coder_v2::coder_v2::create_coder_v2_agent;
use pengy_agent::agent::control_agent::control_agent::create_control_agent;
use pengy_agent::agent::issue_agent::issue_agent::create_issue_agent;
use pengy_agent::agent::pengy_agent::pengy_agent::run_pengy_agent;
use pengy_agent::agent::test_agent::test_agent::create_test_agent;
use pengy_agent::model::model::model::Model;
use std::{env, error::Error};

pub(crate) fn parse_cmd_args() -> Option<(String, String, String, String, String, Option<String>)> {
    let args: Vec<String> = env::args().collect();

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

    if let (Some(p), Some(a), Some(m), Some(pr), Some(k)) =
        (prompt, agent, model, provider, api_key)
    {
        Some((p, a, m, pr, k, base_url))
    } else {
        None
    }
}

pub(crate) fn parse_agent_type(agent_str: &str) -> Result<AgentType, Box<dyn Error>> {
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

pub(crate) async fn run_cmd_mode(
    prompt: String,
    agent_type: AgentType,
    model_name: String,
    provider: String,
    api_key: String,
    custom_base_url: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let base_url = if let Some(custom_url) = custom_base_url {
        App::normalize_base_url(&custom_url)
    } else if provider.to_lowercase() == "custom" {
        DEFAULT_BASE_URL.to_string()
    } else {
        let models = App::get_available_models();
        let found_model = models
            .iter()
            .find(|m| m.name == model_name && m.provider.to_lowercase() == provider.to_lowercase());

        if let Some(m) = found_model {
            App::normalize_base_url(&m.base_url)
        } else {
            DEFAULT_BASE_URL.to_string()
        }
    };

    let model = Model::new(model_name.clone(), api_key.clone(), base_url.clone());

    println!("Running agent in command mode...");
    println!("Agent: {:?}", agent_type);
    println!("Model: {} ({})", model_name, provider);
    println!("Prompt: {}\n", prompt);

    let callback = |event: AgentEvent| match event {
        AgentEvent::Step { step, max_steps } => {
            println!("[Step {}/{}]", step, max_steps);
        }
        AgentEvent::ToolCall { tool_name, args } => {
            println!("[Tool Call] {} with args: {}", tool_name, args);
        }
        AgentEvent::ToolResult { result } => {
            println!("[Tool Result] {}", result);
        }
        AgentEvent::TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        } => {
            println!(
                "[Usage] prompt: {:?}, completion: {:?}, total: {:?}",
                prompt_tokens, completion_tokens, total_tokens
            );
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
