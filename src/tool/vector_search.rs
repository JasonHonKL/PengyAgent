pub mod vector_search {
    //! Perform semantic vector search across provided text files by chunking,
    //! embedding, and scoring content against a query.

    use crate::model::model::model::Model;
    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;
    use std::fs;
    use std::path::Path;

    /// Handles file chunking, embeddings, and similarity scoring for semantic
    /// search use cases.
    pub struct VectorSearchTool {
        tool: Tool,
        api_key: String,
        model_name: String,
        base_url: String,
    }

    impl VectorSearchTool {
        /// Create a new vector search tool with the provided model credentials
        /// and configuration.
        pub fn new(api_key: String, model_name: String, base_url: String) -> Self {
            let mut parameters = HashMap::new();

            // files parameter (required)
            let mut files_items = HashMap::new();
            files_items.insert("type".to_string(), "array".to_string());
            files_items.insert("item_type".to_string(), "string".to_string());
            parameters.insert("files".to_string(), Parameter {
                items: files_items,
                description: "List of file paths to search. Only text files can be searched directly. For PDF files, they must be converted to markdown first using the docs_reader tool.".to_string(),
                enum_values: None,
            });

            // chunk_size parameter (optional)
            let mut chunk_size_items = HashMap::new();
            chunk_size_items.insert("type".to_string(), "number".to_string());
            parameters.insert("chunk_size".to_string(), Parameter {
                items: chunk_size_items,
                description: "Number of words per chunk. Will be capped at 2000 for cost reasons. Defaults to 2000 if not provided.".to_string(),
                enum_values: None,
            });

            // query parameter (required)
            let mut query_items = HashMap::new();
            query_items.insert("type".to_string(), "string".to_string());
            parameters.insert("query".to_string(), Parameter {
                items: query_items,
                description: "The content to search for. This will be embedded and compared against document chunks.".to_string(),
                enum_values: None,
            });

            // top_k parameter (optional)
            let mut top_k_items = HashMap::new();
            top_k_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "top_k".to_string(),
                Parameter {
                    items: top_k_items,
                    description: "Number of top results to return. Defaults to 5 if not provided."
                        .to_string(),
                    enum_values: None,
                },
            );

            let tool = Tool {
                name: "vector_search".to_string(),
                description: "Perform semantic vector search across multiple text files. Takes a list of files, chunks them, embeds the query and chunks, then returns the top K most similar chunks. Only text files can be searched directly - PDF files must be converted to markdown first using docs_reader tool. Chunk size is automatically capped at 2000 words for cost reasons.".to_string(),
                parameters,
                required: vec!["files".to_string(), "query".to_string()],
            };

            Self {
                tool,
                api_key,
                model_name,
                base_url,
            }
        }

        /// Validate that the path points to a readable text file (not PDF) and
        /// return its contents.
        fn read_text_file(&self, file_path: &str) -> Result<String, Box<dyn Error>> {
            let path = Path::new(file_path);

            if !path.exists() {
                return Err(format!("File not found: {}", file_path).into());
            }

            // Check if it's a text file (simple check - could be improved)
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();

            // Warn about PDF files
            if extension == "pdf" {
                return Err("PDF files cannot be searched directly. Please convert to markdown first using the docs_reader tool.".into());
            }

            // Try to read as text
            let content = fs::read_to_string(path)?;
            Ok(content)
        }

        /// Split text into word-based chunks of at most `chunk_size_words`,
        /// returning both the chunk text and its starting index.
        fn chunk_text(&self, text: &str, chunk_size_words: usize) -> Vec<(String, usize)> {
            let words: Vec<&str> = text.split_whitespace().collect();
            let mut chunks = Vec::new();
            let mut start_idx = 0;

            while start_idx < words.len() {
                let end_idx = (start_idx + chunk_size_words).min(words.len());
                let chunk_words = &words[start_idx..end_idx];
                let chunk_text = chunk_words.join(" ");
                chunks.push((chunk_text, start_idx));
                start_idx = end_idx;
            }

            chunks
        }

        /// Compute cosine similarity between two vectors, returning 0.0 for
        /// mismatched lengths or zero norms.
        fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
            if a.len() != b.len() {
                return 0.0;
            }

            let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
            let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

            if norm_a == 0.0 || norm_b == 0.0 {
                return 0.0;
            }

            dot_product / (norm_a * norm_b)
        }

        /// Obtain an embedding for the provided text using the configured model,
        /// creating a runtime if one is not already available.
        fn embed_text(&self, text: &str) -> Result<Vec<f64>, Box<dyn Error>> {
            let model = Model::new(
                self.model_name.clone(),
                self.api_key.clone(),
                self.base_url.clone(),
            );

            // Use tokio runtime to run async code in sync context
            // Try to use current runtime handle first, otherwise create new runtime
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => handle
                    .block_on(model.completion_open_router_embedding(text.to_string()))
                    .map_err(|e| e as Box<dyn Error>),
                Err(_) => {
                    // Not in a tokio runtime, create a new one
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(model.completion_open_router_embedding(text.to_string()))
                        .map_err(|e| e as Box<dyn Error>)
                }
            }
        }

        /// Perform vector search over the provided files, returning the top
        /// matches with similarity scores.
        pub fn search(
            &self,
            files: Vec<String>,
            query: String,
            chunk_size: Option<usize>,
            top_k: Option<usize>,
        ) -> Result<String, Box<dyn Error>> {
            // Determine chunk size (min of 2000 and provided value)
            let chunk_size_words = chunk_size.map(|s| s.min(2000)).unwrap_or(2000);

            let top_k = top_k.unwrap_or(5);

            // Read and chunk all files
            let mut all_chunks: Vec<(String, String, usize)> = Vec::new(); // (file_path, chunk_text, chunk_index)

            for file_path in &files {
                match self.read_text_file(file_path) {
                    Ok(content) => {
                        let chunks = self.chunk_text(&content, chunk_size_words);
                        for (chunk_text, chunk_idx) in chunks {
                            all_chunks.push((file_path.clone(), chunk_text, chunk_idx));
                        }
                    }
                    Err(e) => {
                        // Continue with other files, but note the error
                        eprintln!("Warning: Failed to read file {}: {}", file_path, e);
                    }
                }
            }

            if all_chunks.is_empty() {
                return Err("No valid chunks found in any of the provided files.".into());
            }

            // Embed the query
            println!("Embedding query...");
            let query_embedding = self.embed_text(&query)?;

            // Embed all chunks
            println!("Embedding {} chunks...", all_chunks.len());
            let mut chunk_embeddings = Vec::new();
            for (file_path, chunk_text, chunk_idx) in &all_chunks {
                match self.embed_text(chunk_text) {
                    Ok(embedding) => {
                        chunk_embeddings.push((
                            file_path.clone(),
                            chunk_text.clone(),
                            *chunk_idx,
                            embedding,
                        ));
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to embed chunk from {}: {}", file_path, e);
                    }
                }
            }

            if chunk_embeddings.is_empty() {
                return Err("Failed to embed any chunks.".into());
            }

            // Calculate similarities
            println!("Calculating similarities...");
            let mut similarities: Vec<(String, String, usize, f64)> = chunk_embeddings
                .into_iter()
                .map(|(file_path, chunk_text, chunk_idx, embedding)| {
                    let similarity = Self::cosine_similarity(&query_embedding, &embedding);
                    (file_path, chunk_text, chunk_idx, similarity)
                })
                .collect();

            // Sort by similarity (descending)
            similarities.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

            // Take top K
            let top_results = similarities.into_iter().take(top_k).collect::<Vec<_>>();

            // Format results
            let mut result_lines = Vec::new();
            result_lines.push(format!("Top {} results for query: \"{}\"", top_k, query));
            result_lines.push("=".repeat(80));

            for (idx, (file_path, chunk_text, chunk_idx, similarity)) in
                top_results.iter().enumerate()
            {
                result_lines.push(format!(
                    "\n[Result {}] Similarity: {:.4}",
                    idx + 1,
                    similarity
                ));
                result_lines.push(format!(
                    "File: {} (chunk starting at word {})",
                    file_path, chunk_idx
                ));
                result_lines.push(format!("Content:\n{}", chunk_text));
                result_lines.push("-".repeat(80));
            }

            Ok(result_lines.join("\n"))
        }
    }

    impl ToolCall for VectorSearchTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            // Get required files parameter
            let files_array = args
                .get("files")
                .and_then(|v| v.as_array())
                .ok_or("Missing required parameter: files (must be an array of file paths)")?;

            let files: Vec<String> = files_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();

            if files.is_empty() {
                return Err("Files array cannot be empty.".into());
            }

            // Get required query parameter
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: query")?
                .to_string();

            // Get optional chunk_size parameter
            let chunk_size = args
                .get("chunk_size")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            // Get optional top_k parameter
            let top_k = args
                .get("top_k")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            // Execute the search
            self.search(files, query, chunk_size, top_k)
        }

        fn name(&self) -> &str {
            "vector_search"
        }
    }
}
