pub mod issue_agent {
    use crate::model::model::model::Model;
    use crate::agent::agent::agent::Agent;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::edit::edit::EditTool;
    use crate::tool::todo::todo::TodoTool;
    use crate::tool::github_tool::github_tool::GithubTool;
    use crate::tool::summarizer::summarizer::SummarizerTool;
    use crate::tool::end::end::EndTool;
    use crate::tool::tool::tool::ToolCall;

    /// Creates an issue-focused agent responsible for finding and reporting issues.
    /// This agent should:
    /// - Investigate potential issues and document findings
    /// - Create a temporary branch for investigation, then clean it up
    /// - Publish confirmed issues to GitHub via the github tool
    /// - Confirm when no issues are found
    pub fn create_issue_agent(
        model: Model,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        // Create all tools
        let todo_tool = TodoTool::new();
        let bash_tool = BashTool::new();
        let edit_tool = EditTool::new();
        let github_tool = GithubTool::new();
        let summarizer_tool = SummarizerTool::new();
        let end_tool = EndTool::new();

        // Convert tools to Box<dyn ToolCall>
        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(todo_tool),
            Box::new(bash_tool),
            Box::new(edit_tool),
            Box::new(github_tool),
            Box::new(summarizer_tool),
            Box::new(end_tool),
        ];

        // Get current working directory for system prompt
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        // Default system prompt if not provided
        let default_system_prompt = format!(
            "You are an issue-finding assistant. Your job is to investigate and report issues without committing code.

Available tools:
- todo: Track investigation tasks and checkpoints. Read ONCE at the start, then insert/tick/delete as needed. Do NOT read multiple times in a row.
- bash: Run shell commands in the repo. CRITICAL: Always use non-interactive flags (yolo mode) like '-y', '--yes', '--non-interactive' to avoid getting stuck on yes/no prompts during builds or installs. Use this to inspect git status, branches, logs, and to create/delete temporary branches. SECURITY: Never write to /tmp/ or system directories. Use relative paths under the current working directory.
- edit: Update local files only if needed for reproduction notes or logs (avoid committing).
- github: Create issues on GitHub. Use the action 'create_issue' with a clear title and detailed body (include expected vs actual, steps to reproduce, logs, environment).
- summarizer: Summarize the conversation when asked.
- end: End the run early when requested.

Branch workflow (mandatory):
1) Record the current branch (e.g., via 'git rev-parse --abbrev-ref HEAD').
2) Create and switch to a temporary branch for investigation (e.g., 'git checkout -b issue/<slug>').
3) Do your investigation.
4) After investigation, always switch back to the original branch and delete the temporary branch (e.g., 'git checkout <orig>' then 'git branch -D issue/<slug>'). Never leave the temp branch around.

Issue policy:
- If no issue is found, clearly state that no issue was identified and still clean up branches.
- If an issue is found, publish it via the github tool using 'create_issue'. Provide concise title and detailed body with steps, expected vs actual, scope, and logs. Do not open pull requests or make commits.

CRITICAL SECURITY RULE: When asked to create files, you MUST write files ONLY in the current working directory: {}. Use relative paths like './notes.md'. NEVER write to /tmp/, /var/, /usr/, or other system directories.",
            current_dir
        );

        let final_system_prompt = system_prompt.unwrap_or(default_system_prompt);

        Agent::new(
            model,
            tools,
            final_system_prompt,
            max_retry,
            max_step,
        )
    }
}


