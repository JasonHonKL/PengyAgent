pub mod coder {
    use crate::agent::agent::agent::Agent;
    use crate::model::model::model::Model;
    use crate::prompt::coder::coder_system_prompt;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::docs_researcher::docs_researcher::DocsResearcherTool;
    use crate::tool::edit::edit::EditTool;
    use crate::tool::end::end::EndTool;
    use crate::tool::file_manager::file_manager::FileManagerTool;
    use crate::tool::find_replace::find_replace::FindReplaceTool;
    use crate::tool::grep::grep::GrepTool;
    use crate::tool::summarizer::summarizer::SummarizerTool;
    use crate::tool::think::think::ThinkTool;
    use crate::tool::todo::todo::TodoTool;
    use crate::tool::tool::tool::ToolCall;
    use crate::tool::web::web::WebTool;

    /// Creates a coding agent with the following tools:
    /// - file_manager: Create files or folders inside the workspace
    /// - bash: Execute bash commands in a persistent shell session (for short commands only)
    /// - docs_researcher: Manage documents in the 'pengy_docs' folder (create, read, search)
    /// - edit: Modify existing files using exact string replacements
    /// - find_replace: Find and replace exact text within a file
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
        let find_replace_tool = FindReplaceTool::new();
        let grep_tool = GrepTool::new();
        let todo_tool = TodoTool::new();
        let web_tool = WebTool::new();
        let summarizer_tool = SummarizerTool::new();
        let think_tool = ThinkTool::new();
        let end_tool = EndTool::new();

        // Convert tools to Box<dyn ToolCall>
        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(file_manager_tool),
            Box::new(bash_tool),
            Box::new(docs_researcher_tool),
            Box::new(edit_tool),
            Box::new(find_replace_tool),
            Box::new(grep_tool),
            Box::new(todo_tool),
            Box::new(web_tool),
            Box::new(summarizer_tool),
            Box::new(think_tool),
            Box::new(end_tool),
        ];

        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let final_system_prompt =
            system_prompt.unwrap_or_else(|| coder_system_prompt(&current_dir));

        Agent::new(model, tools, final_system_prompt, max_retry, max_step)
    }
}
