pub mod simple_agent {
    use crate::model::model::model::Model;
    use crate::agent::agent::agent::Agent;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::end::end::EndTool;
    use crate::tool::tool::tool::ToolCall;

    /// Creates a simple agent with the following tools:
    /// - bash: Execute bash commands in a persistent shell session
    /// - end: End the current agent run early with an optional reason
    /// 
    /// This is a minimal agent with only essential tools for basic tasks.
    pub fn create_simple_agent(
        model: Model,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        // Create tools
        let bash_tool = BashTool::new();
        let end_tool = EndTool::new();

        // Convert tools to Box<dyn ToolCall>
        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(bash_tool),
            Box::new(end_tool),
        ];

        // Get current working directory for system prompt
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        // Default system prompt if not provided
        let default_system_prompt = format!(
            "You are a simple assistant with basic capabilities. You can execute bash commands and end conversations when needed.

Available tools:
- bash: Execute bash commands in a persistent shell session. CRITICAL: Always use non-interactive flags (yolo mode) like '-y', '--yes', '--non-interactive' to avoid getting stuck on yes/no prompts during builds or installs. SECURITY: Never write to /tmp/ or system directories. Always use relative paths like './file.txt' or 'file.txt' in the current working directory. Use this to run commands, check file contents, list directories, and perform basic file operations.
- end: End the current agent run immediately. Use when the user explicitly asks to stop or wrap up. You may include a brief reason.

CRITICAL SECURITY RULE: When asked to create files, you MUST write files ONLY in the current working directory: {}. Use relative paths like './file.txt' or 'file.txt'. NEVER write to /tmp/, /var/, /usr/, or any other system directories. This is a strict security requirement - violating this rule is not allowed.

Keep your responses concise and focused on the task at hand.",
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


