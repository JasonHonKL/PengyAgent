pub mod coder_v2 {
    use crate::agent::agent::agent::Agent;
    use crate::model::model::model::Model;
    use crate::prompt::coder::coder_v2_system_prompt;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::docs_researcher::docs_researcher::DocsResearcherTool;
    use crate::tool::edit::edit::EditTool;
    use crate::tool::end::end::EndTool;
    use crate::tool::file_manager::file_manager::FileManagerTool;
    use crate::tool::find_replace::find_replace::FindReplaceTool;
    use crate::tool::grep::grep::GrepTool;
    use crate::tool::read_file::read_file::ReadFileTool;
    use crate::tool::summarizer::summarizer::SummarizerTool;
    use crate::tool::todo::todo::TodoTool;
    use crate::tool::tool::tool::ToolCall;
    use crate::tool::web::web::WebTool;

    /// Create a coder agent using the tools listed in the coder prompt.
    /// Tool order mirrors the prompt guidance:
    /// grep -> read_file -> find_replace -> edit -> file_manager -> docs_researcher -> todo
    /// -> web -> bash -> summarizer -> end.
    pub fn create_coder_v2_agent(
        model: Model,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        // Instantiate tools
        let grep_tool = GrepTool::new();
        let read_file_tool = ReadFileTool::new();
        let find_replace_tool = FindReplaceTool::new();
        let edit_tool = EditTool::new();
        let file_manager_tool = FileManagerTool::new();
        let docs_researcher_tool = DocsResearcherTool::new();
        let todo_tool = TodoTool::new();
        let web_tool = WebTool::new();
        let bash_tool = BashTool::new();
        let summarizer_tool = SummarizerTool::new();
        let end_tool = EndTool::new();

        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(grep_tool),
            Box::new(read_file_tool),
            Box::new(find_replace_tool),
            Box::new(edit_tool),
            Box::new(file_manager_tool),
            Box::new(docs_researcher_tool),
            Box::new(todo_tool),
            Box::new(web_tool),
            Box::new(bash_tool),
            Box::new(summarizer_tool),
            Box::new(end_tool),
        ];

        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let final_system_prompt =
            system_prompt.unwrap_or_else(|| coder_v2_system_prompt(&current_dir));

        Agent::new(model, tools, final_system_prompt, max_retry, max_step)
    }
}
