pub mod code_researcher {
    use crate::model::model::model::Model;
    use crate::agent::agent::agent::Agent;
    use crate::tool::bash::bash::BashTool;
    use crate::tool::docs_researcher::docs_researcher::DocsResearcherTool;
    use crate::tool::docs_reader::docs_reader::DocsReaderTool;
    use crate::tool::edit::edit::EditTool;
    use crate::tool::grep::grep::GrepTool;
    use crate::tool::todo::todo::TodoTool;
    use crate::tool::vector_search::vector_search::VectorSearchTool;
    use crate::tool::web::web::WebTool;
    use crate::tool::summarizer::summarizer::SummarizerTool;
    use crate::tool::tool::tool::ToolCall;

    /// Creates a code researcher agent with the following tools:
    /// - grep: Search file contents using regular expressions
    /// - bash: Execute bash commands in a persistent shell session
    /// - docs_researcher: Manage documents in the 'pengy_docs' folder (create, read, search)
    /// - docs_reader: Read text content from PDF documents
    /// - edit: Modify existing files using exact string replacements
    /// - todo: Manage a todo list (read, insert, tick, delete tasks)
    /// - vector_search: Perform semantic vector search across multiple text files
    /// - web: Fetch content from URLs using HTTP/HTTPS
    /// 
    /// This agent is designed to research codebases and generate research reports.
    /// The vector_search tool requires API credentials for embedding generation.
    pub fn create_code_researcher_agent(
        model: Model,
        api_key: String,
        base_url: String,
        embedding_model: Option<String>,
        system_prompt: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
    ) -> Agent {
        // Create all tools
        let grep_tool = GrepTool::new();
        let bash_tool = BashTool::new();
        let docs_researcher_tool = DocsResearcherTool::new();
        let docs_reader_tool = DocsReaderTool::new();
        let edit_tool = EditTool::new();
        let todo_tool = TodoTool::new();
        
        // Vector search tool requires API credentials
        let embedding_model_name = embedding_model.unwrap_or_else(|| "openai/text-embedding-3-small".to_string());
        let vector_search_tool = VectorSearchTool::new(
            api_key.clone(),
            embedding_model_name,
            base_url.clone(),
        );
        
        let web_tool = WebTool::new();
        let summarizer_tool = SummarizerTool::new();

        // Convert tools to Box<dyn ToolCall>
        let tools: Vec<Box<dyn ToolCall>> = vec![
            Box::new(grep_tool),
            Box::new(bash_tool),
            Box::new(docs_researcher_tool),
            Box::new(docs_reader_tool),
            Box::new(edit_tool),
            Box::new(todo_tool),
            Box::new(vector_search_tool),
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
            "You are a code researcher assistant specialized in analyzing codebases and generating comprehensive research reports. Your goal is to thoroughly understand codebases, document findings, and create detailed research reports.

Available tools:
- grep: Search file contents using regular expressions with ripgrep integration. Searches for patterns in files and returns matching lines with file paths and line numbers. Use this to find specific code patterns, functions, classes, or keywords across the codebase.
- bash: Execute bash commands in a persistent shell session. SECURITY: Never write to /tmp/ or system directories. Always use relative paths like './file.txt' or 'file.txt' in the current working directory. Use this to run build commands, tests, or explore the project structure.
- docs_researcher: Manage documents in the 'pengy_docs' folder. Use 'create' to create a new document, 'read' to read an entire document, or 'search' to search for content in a document with context lines. Use this to store and retrieve research findings.
- docs_reader: Read text content from PDF documents. The PDF is converted to text (via markdown if possible), and the output is limited by the specified number of lines or words. Use this to read documentation, research papers, or PDF-based resources.
- edit: Modify existing files using exact string replacements with 9 fallback strategies for robust matching. Use this to create or update research reports and documentation files.
- todo: Manage a todo list. Use 'read' action to view all tasks, or 'modify' action with 'tick', 'insert', or 'delete' operations to update the list. Use this to track research tasks and findings.
- vector_search: Perform semantic vector search across multiple text files. Takes a list of files, chunks them, embeds the query and chunks, then returns the top K most similar chunks. Only text files can be searched directly - PDF files must be converted to markdown first using docs_reader tool. Use this to find semantically similar code or documentation across the codebase.
- web: Fetch content from a URL using HTTP/HTTPS. Returns the HTML or text content of the webpage. Useful for searching the web, reading documentation, or accessing online resources.

RESEARCH WORKFLOW:
1. Start by exploring the codebase structure using bash commands (ls, find, tree, etc.)
2. Use grep to search for key patterns, functions, classes, or concepts
3. Use vector_search to find semantically related code across multiple files
4. Read important files to understand the architecture and implementation
5. Use docs_reader to process any PDF documentation
6. Use web to fetch external documentation or resources if needed
7. Use docs_researcher to store findings and intermediate research notes
8. Use todo to track research progress and findings
9. Use edit to create comprehensive research reports documenting your findings

RESEARCH REPORT STRUCTURE:
When generating a research report, include:
- Executive Summary: High-level overview of the codebase
- Architecture Overview: System design, components, and relationships
- Key Components: Important modules, classes, and functions
- Code Patterns: Common patterns, conventions, and best practices
- Dependencies: External libraries and their purposes
- Testing Strategy: How the codebase is tested
- Documentation: Available documentation and its quality
- Findings: Interesting discoveries, potential issues, or recommendations

CRITICAL SECURITY RULE: When asked to create files, you MUST write files ONLY in the current working directory: {}. Use relative paths like './research_report.md' or just 'research_report.md'. NEVER write to /tmp/, /var/, /usr/, or any other system directories. This is a strict security requirement - violating this rule is not allowed.",
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


