//! Tools available to the agent runtime, exposing capabilities like shell
//! execution, file management, documentation helpers, and web access.
//! Each submodule wraps a concrete tool and implements the shared `ToolCall`
//! trait to provide a consistent interface for invocation.

pub mod bash;
pub mod codebase_search;
pub mod docs_reader;
pub mod docs_researcher;
pub mod edit_file;
pub mod edit;
pub mod end;
pub mod file_search;
pub mod file_manager;
pub mod find_replace;
pub mod github_tool;
pub mod grep;
pub mod grep_search;
pub mod list_dir;
pub mod read_file;
pub mod run_terminal_cmd;
pub mod summarizer;
pub mod think;
pub mod todo;
pub mod tool;
pub mod vector_search;
pub mod vision_judge;
pub mod web;
pub mod web_search;
pub mod delete_file;
pub mod diff_history;
pub mod reapply;
pub mod multi_tool_use;
