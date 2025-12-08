//! Tools available to the agent runtime, exposing capabilities like shell
//! execution, file management, documentation helpers, and web access.
//! Each submodule wraps a concrete tool and implements the shared `ToolCall`
//! trait to provide a consistent interface for invocation.

pub mod bash;
pub mod docs_reader;
pub mod docs_researcher;
pub mod edit;
pub mod end;
pub mod file_manager;
pub mod github_tool;
pub mod grep;
pub mod summarizer;
pub mod todo;
pub mod tool;
pub mod vector_search;
pub mod vision_judge;
pub mod web;
