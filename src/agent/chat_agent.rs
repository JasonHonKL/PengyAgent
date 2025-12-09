pub mod chat_agent {
    use crate::{
        agent::agent::agent::Agent,
        model::model::model::Model,
        prompt::chat::chat_system_prompt,
        tool::{
            docs_reader::docs_reader::DocsReaderTool, end::end::EndTool, grep::grep::GrepTool,
            summarizer::summarizer::SummarizerTool, tool::tool::ToolCall,
        },
    };

    pub fn create_chat_agent(
        model: Model,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        let grep_tool = GrepTool::new();
        let docs_researcher_tool = DocsReaderTool::new();
        let summarizer_tool = SummarizerTool::new();
        let end_tool = EndTool::new();

        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(grep_tool),
            Box::new(docs_researcher_tool),
            Box::new(summarizer_tool),
            Box::new(end_tool),
        ];

        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();
        let default_prompt = chat_system_prompt(&current_dir);
        let final_system_prompt = system_prompt.unwrap_or(default_prompt);

        // Read-only agent: tools above cannot modify files. This agent is for discussion and code navigation only.
        Agent::new(model, tools, final_system_prompt, max_retry, max_step)
    }
}
