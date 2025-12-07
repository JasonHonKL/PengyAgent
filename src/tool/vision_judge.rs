pub mod vision_judge {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use serde_json;
    use std::error::Error;
    use crate::tool::tool::tool::{ToolCall, Tool, Parameter};

    pub struct VisionJudgeTool {
        tool: Tool,
    }

    impl VisionJudgeTool {
        pub fn new() -> Self {
            let mut parameters = HashMap::new();
            
            // image_path parameter (optional)
            let mut image_path_items = HashMap::new();
            image_path_items.insert("type".to_string(), "string".to_string());
            parameters.insert("image_path".to_string(), Parameter {
                items: image_path_items,
                description: "Path to an image file to read. If provided, this image will be analyzed.".to_string(),
                enum_values: None,
            });

            // screen_cap parameter (optional boolean)
            let mut screen_cap_items = HashMap::new();
            screen_cap_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert("screen_cap".to_string(), Parameter {
                items: screen_cap_items,
                description: "If true, capture a screenshot of the current screen. If false or not provided, use image_path instead.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "vision_judge".to_string(),
                description: "Read images from a file path or capture a screenshot. Returns the image path (as data URL) for vision analysis. The image will be automatically summarized in the next step if this tool was called previously.".to_string(),
                parameters,
                required: vec![],
            };

            Self { tool }
        }

        fn image_to_data_url(image_path: &str) -> Result<String, Box<dyn Error>> {
            // Read the image file
            let image_data = fs::read(image_path)?;
            
            // Determine MIME type from file extension
            let mime_type = Path::new(image_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| match ext.to_lowercase().as_str() {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "image/png", // default
                })
                .unwrap_or("image/png");
            
            // Encode to base64
            use base64::Engine;
            let base64_data = base64::engine::general_purpose::STANDARD.encode(&image_data);
            
            // Return as data URL
            Ok(format!("data:{};base64,{}", mime_type, base64_data))
        }

        fn capture_screenshot() -> Result<String, Box<dyn Error>> {
            // Use screenshots crate to capture screen
            let screens = screenshots::Screen::all()?;
            
            if screens.is_empty() {
                return Err("No screens available".into());
            }
            
            // Capture the first screen
            let screen = screens[0];
            let image = screen.capture()?;
            
            // Use the to_png() method to get PNG bytes directly
            let png_bytes = image.to_png(None)?;
            
            // Encode to base64
            use base64::Engine;
            let base64_data = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            
            // Return as data URL
            Ok(format!("data:image/png;base64,{}", base64_data))
        }

        fn execute_vision_judge(&self, image_path: Option<&str>, screen_cap: bool) -> Result<String, Box<dyn Error>> {
            let data_url = if screen_cap {
                // Capture screenshot
                Self::capture_screenshot()?
            } else if let Some(path) = image_path {
                // Read from file path
                if !Path::new(path).exists() {
                    return Err(format!("Image file does not exist: {}", path).into());
                }
                Self::image_to_data_url(path)?
            } else {
                return Err("Either image_path or screen_cap must be provided".into());
            };
            
            // Return the data URL so it can be used in the next vision completion call
            Ok(data_url)
        }
    }

    impl ToolCall for VisionJudgeTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;
            
            // Get optional parameters
            let image_path = args.get("image_path")
                .and_then(|v| v.as_str());

            let screen_cap = args.get("screen_cap")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Execute the vision judge (screenshots is sync, so we can call directly)
            match self.execute_vision_judge(image_path, screen_cap) {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Failed to execute vision_judge: {}", e).into())
            }
        }

        fn name(&self) -> &str {
            "vision_judge"
        }
    }
}

