pub mod coder {
    use crate::model::model::model::Model;
    use crate::agent::agent::agent::Agent;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::docs_researcher::docs_researcher::DocsResearcherTool;
    use crate::tool::edit::edit::EditTool;
    use crate::tool::grep::grep::GrepTool;
    use crate::tool::todo::todo::TodoTool;
    use crate::tool::web::web::WebTool;
    use crate::tool::summarizer::summarizer::SummarizerTool;
    use crate::tool::tool::tool::ToolCall;

    /// Creates a coding agent with the following tools:
    /// - bash: Execute bash commands in a persistent shell session
    /// - docs_researcher: Manage documents in the 'pengy_docs' folder (create, read, search)
    /// - edit: Modify existing files using exact string replacements
    /// - grep: Search file contents using regular expressions
    /// - todo: Manage a todo list (read, insert, tick, delete tasks)
    /// - web: Fetch content from URLs using HTTP/HTTPS
    pub fn create_coder_agent(
        model: Model,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        // Create all tools
        let bash_tool = BashTool::new();
        let docs_researcher_tool = DocsResearcherTool::new();
        let edit_tool = EditTool::new();
        let grep_tool = GrepTool::new();
        let todo_tool = TodoTool::new();
        let web_tool = WebTool::new();
        let summarizer_tool = SummarizerTool::new();

        // Convert tools to Box<dyn ToolCall>
        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(bash_tool),
            Box::new(docs_researcher_tool),
            Box::new(edit_tool),
            Box::new(grep_tool),
            Box::new(todo_tool),
            Box::new(web_tool),
            Box::new(summarizer_tool),
        ];

        // Get current working directory for system prompt
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        // Default system prompt if not provided
        let default_system_prompt = format!(
            "You are a coding assistant with access to powerful development tools. Available tools:
- bash: Execute bash commands in a persistent shell session. SECURITY: Never write to /tmp/ or system directories. Always use relative paths like './file.txt' or 'file.txt' in the current working directory.
- docs_researcher: Manage documents in the 'pengy_docs' folder. Use 'create' to create a new document, 'read' to read an entire document, or 'search' to search for content in a document with context lines.
- edit: Modify existing files using exact string replacements with 9 fallback strategies for robust matching.
- grep: Search file contents using regular expressions with ripgrep integration. Searches for patterns in files and returns matching lines with file paths and line numbers.
- todo: Manage a todo list. Use 'read' action to view all tasks, or 'modify' action with 'tick', 'insert', or 'delete' operations to update the list.
- web: Fetch content from a URL using HTTP/HTTPS. Returns the HTML or text content of the webpage. Useful for searching the web, reading documentation, or accessing online resources.

CRITICAL SECURITY RULE: When asked to create files, you MUST write files ONLY in the current working directory: {}. Use relative paths like './main.py' or just 'main.py'. NEVER write to /tmp/, /var/, /usr/, or any other system directories. This is a strict security requirement - violating this rule is not allowed.",
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


