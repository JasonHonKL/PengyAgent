pub mod docs_reader {
    //! Read PDF documents and return text snippets with optional line/word
    //! limits. Uses multiple backends (pandoc, pdf_extract, pdftotext) to
    //! maximize extraction success and adds truncation metadata for the caller.

    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::path::PathBuf;
    use std::process::Command;

    /// Extracts text from PDFs and returns limited snippets for downstream
    /// processing.
    pub struct DocsReaderTool {
        tool: Tool,
    }

    impl DocsReaderTool {
        /// Create the tool definition describing accepted parameters and
        /// defaults.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            // file_name parameter (required)
            let mut file_name_items = HashMap::new();
            file_name_items.insert("type".to_string(), "string".to_string());
            parameters.insert(
                "file_name".to_string(),
                Parameter {
                    items: file_name_items,
                    description:
                        "The path to the PDF file to read. Can be a relative or absolute path."
                            .to_string(),
                    enum_values: None,
                },
            );

            // limit_type parameter (optional)
            let mut limit_type_items = HashMap::new();
            limit_type_items.insert("type".to_string(), "string".to_string());
            parameters.insert("limit_type".to_string(), Parameter {
                items: limit_type_items,
                description: "The type of limit to apply: 'lines' to limit by number of lines, or 'words' to limit by number of words. Defaults to 'lines'.".to_string(),
                enum_values: Some(vec!["lines".to_string(), "words".to_string()]),
            });

            // limit parameter (optional)
            let mut limit_items = HashMap::new();
            limit_items.insert("type".to_string(), "number".to_string());
            parameters.insert("limit".to_string(), Parameter {
                items: limit_items,
                description: "The maximum number of lines or words to return. Defaults to 1000 lines if limit_type is 'lines', or 5000 words if limit_type is 'words'. This helps avoid returning files that are too large.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "docs_reader".to_string(),
                description: "Read text content from a PDF document. The PDF is converted to text (via markdown if possible), and the output is limited by the specified number of lines or words to avoid returning files that are too large. This is useful for reading PDF documents that need to be processed as text.".to_string(),
                parameters,
                required: vec!["file_name".to_string()],
            };

            Self { tool }
        }

        /// Try several extraction strategies to convert a PDF into text,
        /// preferring pandoc, then `pdf_extract`, and finally `pdftotext`.
        fn convert_pdf_to_text(&self, file_path: &PathBuf) -> Result<String, Box<dyn Error>> {
            // First, try to use pandoc if available (converts PDF to markdown, then we can use as text)
            if let Ok(output) = Command::new("pandoc")
                .arg(file_path.as_os_str())
                .arg("-t")
                .arg("markdown")
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !text.trim().is_empty() {
                        return Ok(text);
                    }
                }
            }

            // Fallback: try pdf-extract library
            match pdf_extract::extract_text(file_path.as_path()) {
                Ok(text) => {
                    if !text.trim().is_empty() {
                        return Ok(text);
                    }
                }
                Err(e) => {
                    // If pdf-extract fails, try pdftotext command as another fallback
                    if let Ok(output) = Command::new("pdftotext")
                        .arg(file_path.as_os_str())
                        .arg("-")
                        .output()
                    {
                        if output.status.success() {
                            let text = String::from_utf8_lossy(&output.stdout).to_string();
                            if !text.trim().is_empty() {
                                return Ok(text);
                            }
                        }
                    }
                    return Err(format!("Failed to extract text from PDF: {}. Tried pandoc, pdf-extract library, and pdftotext command.", e).into());
                }
            }

            Err("Failed to extract text from PDF: All conversion methods failed or returned empty content.".into())
        }

        /// Apply either word-count or line-count truncation to the extracted
        /// text.
        fn limit_text(&self, text: &str, limit_type: &str, limit: usize) -> String {
            match limit_type {
                "words" => {
                    let words: Vec<&str> = text.split_whitespace().collect();
                    let limited_words: Vec<&str> = words.into_iter().take(limit).collect();
                    limited_words.join(" ")
                }
                "lines" | _ => {
                    let lines: Vec<&str> = text.lines().collect();
                    let limited_lines: Vec<&str> = lines.into_iter().take(limit).collect();
                    limited_lines.join("\n")
                }
            }
        }

        /// Load a PDF from disk, convert it to text, and return a truncated
        /// result with metadata describing any truncation applied.
        fn read_pdf(
            &self,
            file_name: &str,
            limit_type: Option<&str>,
            limit: Option<usize>,
        ) -> Result<String, Box<dyn Error>> {
            // Resolve file path
            let file_path = if PathBuf::from(file_name).is_absolute() {
                PathBuf::from(file_name)
            } else {
                std::env::current_dir()?.join(file_name)
            };

            // Check if file exists
            if !file_path.exists() {
                return Err(format!("File '{}' not found.", file_name).into());
            }

            // Check if it's a PDF file
            if let Some(ext) = file_path.extension() {
                if ext.to_string_lossy().to_lowercase() != "pdf" {
                    return Err(format!(
                        "File '{}' is not a PDF file (extension: {:?})",
                        file_name, ext
                    )
                    .into());
                }
            } else {
                return Err(format!(
                    "File '{}' has no extension. Please specify a PDF file.",
                    file_name
                )
                .into());
            }

            // Convert PDF to text
            let full_text = self.convert_pdf_to_text(&file_path)?;

            // Apply limits
            let limit_type = limit_type.unwrap_or("lines");
            let default_limit = if limit_type == "words" { 5000 } else { 1000 };
            let limit = limit.unwrap_or(default_limit);

            let limited_text = self.limit_text(&full_text, limit_type, limit);

            // Add metadata about truncation
            let total_lines = full_text.lines().count();
            let total_words = full_text.split_whitespace().count();
            let returned_lines = limited_text.lines().count();
            let returned_words = limited_text.split_whitespace().count();

            let mut result = limited_text;
            if limit_type == "lines" && returned_lines < total_lines {
                result.push_str(&format!(
                    "\n\n[Note: Document truncated from {} lines to {} lines]",
                    total_lines, returned_lines
                ));
            } else if limit_type == "words" && returned_words < total_words {
                result.push_str(&format!(
                    "\n\n[Note: Document truncated from {} words to {} words]",
                    total_words, returned_words
                ));
            }

            Ok(result)
        }
    }

    impl ToolCall for DocsReaderTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        /// Parse arguments and run the PDF reader with optional limits.
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            // Get required file_name parameter
            let file_name = args
                .get("file_name")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: file_name")?;

            // Get optional limit_type parameter
            let limit_type = args.get("limit_type").and_then(|v| v.as_str());

            // Get optional limit parameter
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            self.read_pdf(file_name, limit_type, limit)
        }

        fn name(&self) -> &str {
            "docs_reader"
        }
    }
}
