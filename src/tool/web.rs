pub mod web {
    use std::collections::HashMap;
    use serde_json;
    use std::error::Error;
    use crate::tool::tool::tool::{ToolCall, Tool, Parameter};

    pub struct WebTool {
        tool: Tool,
        client: reqwest::Client,
    }

    impl WebTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            
            // url parameter (required)
            let mut url_items = HashMap::new();
            url_items.insert("type".to_string(), "string".to_string());
            parameters.insert("url".to_string(), Parameter {
                items: url_items,
                description: "The URL to fetch. Must be a valid HTTP or HTTPS URL.".to_string(),
                enum_values: None,
            });

            // timeout parameter (optional)
            let mut timeout_items = HashMap::new();
            timeout_items.insert("type".to_string(), "number".to_string());
            parameters.insert("timeout".to_string(), Parameter {
                items: timeout_items,
                description: "Request timeout in seconds (default: 30).".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "web".to_string(),
                description: "Fetch content from a URL using HTTP/HTTPS. Returns the HTML or text content of the webpage. Useful for searching the web, reading documentation, or accessing online resources.".to_string(),
                parameters,
                required: vec!["url".to_string()],
            };

            // Create HTTP client with default timeout
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .build()
                .expect("Failed to create HTTP client");

            Self { tool, client }
        }

        async fn fetch_url(&self, url: &str, timeout_secs: Option<u64>) -> Result<String, Box<dyn Error>> {
            // Validate URL
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(format!("Invalid URL: {}. URL must start with http:// or https://", url).into());
            }

            // Create request with optional custom timeout
            let mut request = self.client.get(url);
            
            if let Some(timeout) = timeout_secs {
                request = request.timeout(std::time::Duration::from_secs(timeout));
            }

            // Send request
            let response = request.send().await?;

            // Check status
            if !response.status().is_success() {
                return Err(format!("HTTP error: {} - {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown")).into());
            }

            // Get content type
            let content_type = response.headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("text/html")
                .to_string();

            // Read response body
            let body = response.text().await?;

            // If it's HTML, try to extract meaningful text (basic extraction)
            if content_type.contains("text/html") {
                let cleaned = Self::extract_text_from_html(&body);
                Ok(format!("Content-Type: {}\n\n{}", content_type, cleaned))
            } else {
                // For non-HTML content, return as-is
                Ok(format!("Content-Type: {}\n\n{}", content_type, body))
            }
        }

        fn extract_text_from_html(html: &str) -> String {
            // Basic HTML text extraction - remove script and style tags, decode entities
            let mut result = String::new();
            let mut in_script = false;
            let mut in_style = false;
            let mut chars = html.chars().peekable();
            
            // Simple state machine to remove script/style tags and extract text
            while let Some(ch) = chars.next() {
                match ch {
                    '<' => {
                        let mut tag_name = String::new();
                        // Read tag name
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch == '>' || next_ch.is_whitespace() {
                                break;
                            }
                            if let Some(c) = chars.next() {
                                tag_name.push(c.to_ascii_lowercase());
                            }
                        }
                        
                        if tag_name == "script" {
                            in_script = true;
                        } else if tag_name == "/script" {
                            in_script = false;
                        } else if tag_name == "style" {
                            in_style = true;
                        } else if tag_name == "/style" {
                            in_style = false;
                        }
                        
                        // Skip to closing >
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch == '>' {
                                chars.next();
                                break;
                            }
                            chars.next();
                        }
                    }
                    _ => {
                        if !in_script && !in_style {
                            result.push(ch);
                        }
                    }
                }
            }
            
            // Decode common HTML entities
            let text = result.replace("&nbsp;", " ")
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&apos;", "'")
                .replace("&mdash;", "—")
                .replace("&ndash;", "–");
            
            // Normalize whitespace - replace multiple whitespace with single space
            let mut normalized = String::new();
            let mut prev_was_whitespace = false;
            for ch in text.chars() {
                if ch.is_whitespace() {
                    if !prev_was_whitespace {
                        normalized.push(' ');
                        prev_was_whitespace = true;
                    }
                } else {
                    normalized.push(ch);
                    prev_was_whitespace = false;
                }
            }
            
            // Trim and limit length (prevent huge outputs)
            normalized.trim().chars().take(50000).collect()
        }
    }

    impl ToolCall for WebTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            
            // Get required URL parameter
            let url = args.get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: url")?;

            // Get optional timeout parameter
            let timeout = args.get("timeout")
                .and_then(|v| v.as_u64());

            // Since reqwest is async, we need to use a runtime
            // Create a new runtime for this call
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create async runtime: {}", e))?;
            
            // Execute the async fetch
            match rt.block_on(self.fetch_url(url, timeout)) {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Failed to fetch URL: {}", e).into())
            }
        }

        fn name(&self) -> &str {
            "web"
        }
    }
}

