use crate::prompt::chat::TODO_REMINDER;

pub fn research_prompt(user_request: &str, history: Option<&str>) -> String {
    let mut prompt = if user_request.trim().is_empty() {
        "Research the codebase and produce a concise report: architecture, key components, dependencies, risks, and implementation recommendations.".to_string()
    } else {
        format!(
            "Research the codebase to support this request:\n{}\n\nProduce a concise report covering architecture, key components, dependencies, risks, and actionable recommendations.\nPlanning: {}",
            user_request, TODO_REMINDER
        )
    };

    if let Some(h) = history {
        prompt.push_str("\n\nConversation history (for context, include only relevant points):\n");
        prompt.push_str(h);
    }
    prompt
}

pub fn implementation_prompt(
    user_request: &str,
    research_report: &str,
    history: Option<&str>,
) -> String {
    let mut prompt = format!(
        "Implement the requested change using the research insights.\nUser request:\n{}\n\nResearch report:\n{}\n\nPlanning: {}. Create a brief plan (≤3 steps) and then implement using the provided tools.",
        user_request, research_report, TODO_REMINDER
    );

    if let Some(h) = history {
        prompt.push_str("\n\nConversation history (use relevant context):\n");
        prompt.push_str(h);
    }
    prompt
}

pub fn testing_prompt(
    user_request: &str,
    research_report: &str,
    implementation_summary: &str,
    history: Option<&str>,
) -> String {
    let mut prompt = format!(
        "Test the implementation.\nUser request:\n{}\n\nResearch report:\n{}\n\nImplementation summary:\n{}\n\nCreate targeted tests (or commands) to validate correctness. Explain expected outcomes. Planning: {}.",
        user_request, research_report, implementation_summary, TODO_REMINDER
    );

    if let Some(h) = history {
        prompt.push_str("\n\nConversation history (use relevant context):\n");
        prompt.push_str(h);
    }
    prompt
}
