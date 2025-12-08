pub mod chat_agent {
    use crate::{
        agent::agent::agent::Agent,
        model::model::model::Model,
        tool::{
            docs_reader::docs_reader::DocsReaderTool,
            end::end::EndTool,
            grep::grep::GrepTool,
            summarizer::{self, summarizer::SummarizerTool},
            todo,
            tool::tool::ToolCall,
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

        todo!("Implement chat agent");
    }
}
