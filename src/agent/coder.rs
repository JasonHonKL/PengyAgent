pub mod coder {
    use crate::agent::agent::agent::Agent;
    use crate::model::model::model::Model;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::docs_researcher::docs_researcher::DocsResearcherTool;
    use crate::tool::edit::edit::EditTool;
    use crate::tool::end::end::EndTool;
    use crate::tool::file_manager::file_manager::FileManagerTool;
    use crate::tool::grep::grep::GrepTool;
    use crate::tool::summarizer::summarizer::SummarizerTool;
    use crate::tool::todo::todo::TodoTool;
    use crate::tool::tool::tool::ToolCall;
    use crate::tool::web::web::WebTool;

    /// Creates a coding agent with the following tools:
    /// - file_manager: Create files or folders inside the workspace
    /// - bash: Execute bash commands in a persistent shell session (for short commands only)
    /// - docs_researcher: Manage documents in the 'pengy_docs' folder (create, read, search)
    /// - edit: Modify existing files using exact string replacements
    /// - grep: Search file contents using regular expressions
    /// - todo: Manage a todo list (read, insert, tick, delete tasks)
    /// - web: Fetch content from URLs using HTTP/HTTPS
    /// - summarizer: Condense the conversation when requested
    /// - end: End the current agent run early with an optional reason
    pub fn create_coder_agent(
        model: Model,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        // Create all tools
        let file_manager_tool = FileManagerTool::new();
        let bash_tool = BashTool::new();
        let docs_researcher_tool = DocsResearcherTool::new();
        let edit_tool = EditTool::new();
        let grep_tool = GrepTool::new();
        let todo_tool = TodoTool::new();
        let web_tool = WebTool::new();
        let summarizer_tool = SummarizerTool::new();
        let end_tool = EndTool::new();

        // Convert tools to Box<dyn ToolCall>
        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(file_manager_tool),
            Box::new(bash_tool),
            Box::new(docs_researcher_tool),
            Box::new(edit_tool),
            Box::new(grep_tool),
            Box::new(todo_tool),
            Box::new(web_tool),
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
            "You are a fast, pragmatic coding agent working in the workspace: {}.
Stay concise and choose the right tool instead of defaulting to bash.

SYSTEM FORMAT:
You are running in a terminal-based AI agent system (Pengy Agent). Your responses and tool calls are displayed in a TUI (Text User Interface) with:
- Tool calls shown as cards with status indicators (running/success/error)
- File edits displayed as side-by-side diffs (left: old, right: new)
- Modified files tracked in a sidebar showing line counts (+added -removed)
- Research notes live in the 'pengy_docs' folder; read/use them when relevant.
- All file operations must use absolute paths rooted in the workspace
- The system automatically tracks and displays file modifications

TOOLS (use the right tool for each task):
- file_manager: Create NEW files or folders. Use for: creating new files, creating directories, setting up project structure. Supports createParents and overwrite options.
- edit: Modify EXISTING files. Use for: updating code, changing content, fixing bugs. Requires exact string matching for replacements.
- grep: Search file contents with regex. Use for: finding code patterns, locating functions, searching for text across files.
- docs_researcher: Manage documents in 'pengy_docs' folder. Use for: creating documentation, reading docs, searching documentation.
- todo: Manage task list (read, insert, tick, delete). CRITICAL for multi-step work: read existing todos ONCE at the start, insert tasks before starting, tick as you complete them. Do NOT read the todo list multiple times in a row - read once, then work on tasks.
- web: Fetch content from URLs. Use for: downloading files, reading web pages, getting API documentation.
- bash: Run shell commands ONLY when necessary. Use for: running tests, quick checks, package installation (uv, pip, etc.), git operations. CRITICAL: Always use non-interactive flags (yolo mode) like '-y', '--yes', '--non-interactive' to avoid getting stuck on yes/no prompts during builds or installs. NEVER use bash to create/edit files - use file_manager/edit instead.
- summarizer: Condense conversation when it gets too long (rarely needed).
- end: End the agent run early if explicitly requested by user.

SAFETY RULES:
- Only touch paths under {}. Reject /tmp, /var, /usr, or anything outside the workspace.
- Use absolute paths rooted in the workspace when creating/editing files.

WORKFLOW:
1. For multi-step tasks: Read todo list → Insert tasks → Execute → Tick completed tasks
2. For file operations: Use file_manager for NEW files, edit for EXISTING files
3. For discovery: Use grep to search, not bash
4. For commands: Use bash only for package management, tests, git - NOT for file operations
5. Stay focused: Batch related operations, keep tool calls efficient",
            current_dir,
            current_dir
        );

        let final_system_prompt = system_prompt.unwrap_or(default_system_prompt);

        Agent::new(model, tools, final_system_prompt, max_retry, max_step)
    }
}
