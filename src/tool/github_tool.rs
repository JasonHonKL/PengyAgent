pub mod github_tool {
    //! Wrapper around `gh` CLI operations for viewing and creating issues or
    //! pull requests, with a unified tool schema for agent consumption.

    use crate::tool::tool::tool::{Parameter, Tool, ToolCall};
    use crate::util::github_control::github_control;
    use serde_json;
    use std::collections::HashMap;
    use std::error::Error;

    /// Exposes a subset of GitHub actions (list/view/create) via the tool
    /// interface.
    pub struct GithubTool {
        tool: Tool,
    }

    impl GithubTool {
        /// Define the GitHub tool parameters and supported actions.
        pub fn new() -> Self {
            let mut parameters = HashMap::new();

            // action parameter
            let mut action_items = HashMap::new();
            action_items.insert("type".to_string(), "string".to_string());
            parameters.insert("action".to_string(), Parameter {
                items: action_items,
                description: "The GitHub action to perform: 'view_pr', 'list_prs', 'view_issue', 'list_issues', 'create_issue', or 'create_pr'.".to_string(),
                enum_values: Some(vec![
                    "view_pr".to_string(),
                    "list_prs".to_string(),
                    "view_issue".to_string(),
                    "list_issues".to_string(),
                    "create_issue".to_string(),
                    "create_pr".to_string(),
                ]),
            });

            // pr_number parameter (for view_pr)
            let mut pr_number_items = HashMap::new();
            pr_number_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "pr_number".to_string(),
                Parameter {
                    items: pr_number_items,
                    description: "The PR number to view. Required for 'view_pr' action."
                        .to_string(),
                    enum_values: None,
                },
            );

            // issue_number parameter (for view_issue)
            let mut issue_number_items = HashMap::new();
            issue_number_items.insert("type".to_string(), "number".to_string());
            parameters.insert(
                "issue_number".to_string(),
                Parameter {
                    items: issue_number_items,
                    description: "The issue number to view. Required for 'view_issue' action."
                        .to_string(),
                    enum_values: None,
                },
            );

            // state parameter (for list_prs and list_issues)
            let mut state_items = HashMap::new();
            state_items.insert("type".to_string(), "string".to_string());
            parameters.insert("state".to_string(), Parameter {
                items: state_items,
                description: "Filter by state: 'open', 'closed', or 'all'. Used for 'list_prs' and 'list_issues' actions. Defaults to 'open' if not specified.".to_string(),
                enum_values: Some(vec!["open".to_string(), "closed".to_string(), "all".to_string()]),
            });

            // limit parameter (for list_prs and list_issues)
            let mut limit_items = HashMap::new();
            limit_items.insert("type".to_string(), "number".to_string());
            parameters.insert("limit".to_string(), Parameter {
                items: limit_items,
                description: "Optional limit on number of results to return. Used for 'list_prs' and 'list_issues' actions.".to_string(),
                enum_values: None,
            });

            // repo parameter (optional for all actions)
            let mut repo_items = HashMap::new();
            repo_items.insert("type".to_string(), "string".to_string());
            parameters.insert("repo".to_string(), Parameter {
                items: repo_items,
                description: "Optional repository in format 'owner/repo'. If not provided, uses the current repository.".to_string(),
                enum_values: None,
            });

            // title parameter (for create_issue and create_pr)
            let mut title_items = HashMap::new();
            title_items.insert("type".to_string(), "string".to_string());
            parameters.insert("title".to_string(), Parameter {
                items: title_items,
                description: "The title of the issue or PR. Required for 'create_issue' and 'create_pr' actions.".to_string(),
                enum_values: None,
            });

            // body parameter (for create_issue and create_pr)
            let mut body_items = HashMap::new();
            body_items.insert("type".to_string(), "string".to_string());
            parameters.insert("body".to_string(), Parameter {
                items: body_items,
                description: "The body/description of the issue or PR. Required for 'create_issue' and 'create_pr' actions.".to_string(),
                enum_values: None,
            });

            // labels parameter (for create_issue)
            let mut labels_items = HashMap::new();
            labels_items.insert("type".to_string(), "array".to_string());
            labels_items.insert("item_type".to_string(), "string".to_string());
            parameters.insert("labels".to_string(), Parameter {
                items: labels_items,
                description: "Optional array of label names to add to the issue. Used for 'create_issue' action.".to_string(),
                enum_values: None,
            });

            // head parameter (for create_pr)
            let mut head_items = HashMap::new();
            head_items.insert("type".to_string(), "string".to_string());
            parameters.insert("head".to_string(), Parameter {
                items: head_items,
                description: "The branch to merge from (e.g., 'feature-branch' or 'owner:feature-branch'). Required for 'create_pr' action.".to_string(),
                enum_values: None,
            });

            // base parameter (for create_pr)
            let mut base_items = HashMap::new();
            base_items.insert("type".to_string(), "string".to_string());
            parameters.insert("base".to_string(), Parameter {
                items: base_items,
                description: "The branch to merge into (e.g., 'main' or 'master'). Optional for 'create_pr' action, defaults to the repository's default branch.".to_string(),
                enum_values: None,
            });

            // draft parameter (for create_pr)
            let mut draft_items = HashMap::new();
            draft_items.insert("type".to_string(), "boolean".to_string());
            parameters.insert("draft".to_string(), Parameter {
                items: draft_items,
                description: "Whether to create as a draft PR. Optional for 'create_pr' action, defaults to false.".to_string(),
                enum_values: None,
            });

            let tool = Tool {
                name: "github".to_string(),
                description: "Interact with GitHub repositories using the GitHub CLI (gh). Supports viewing and listing PRs and issues, as well as creating new PRs and issues. All operations return JSON data for easy parsing.".to_string(),
                parameters,
                required: vec!["action".to_string()],
            };

            Self { tool }
        }
    }

    impl ToolCall for GithubTool {
        fn get_json(&self) -> Result<serde_json::Value, serde_json::Error> {
            self.tool.get_json()
        }

        /// Route parsed arguments to the requested GitHub action and return the
        /// CLI output or error.
        fn run(&self, arguments: &str) -> Result<String, Box<dyn Error>> {
            // Parse arguments JSON
            let args: serde_json::Value = serde_json::from_str(arguments)?;

            // Get the action
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .ok_or("Missing required parameter: action")?;

            // Get optional repo parameter
            let repo = args.get("repo").and_then(|v| v.as_str());

            // Route to appropriate handler based on action
            match action {
                "view_pr" => {
                    let pr_number = args.get("pr_number")
                        .and_then(|v| v.as_u64())
                        .ok_or("Missing required parameter: pr_number (required for view_pr action)")?;

                    match github_control::view_pr(pr_number, repo) {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to view PR: {}", e).into()),
                    }
                }
                "list_prs" => {
                    let state = args.get("state")
                        .and_then(|v| v.as_str());
                    let limit = args.get("limit")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    
                    match github_control::list_prs(state, repo, limit) {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to list PRs: {}", e).into()),
                    }
                }
                "view_issue" => {
                    let issue_number = args.get("issue_number")
                        .and_then(|v| v.as_u64())
                        .ok_or("Missing required parameter: issue_number (required for view_issue action)")?;
                    
                    match github_control::view_issue(issue_number, repo) {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to view issue: {}", e).into()),
                    }
                }
                "list_issues" => {
                    let state = args.get("state")
                        .and_then(|v| v.as_str());
                    let limit = args.get("limit")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32);
                    
                    match github_control::list_issues(state, repo, limit) {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to list issues: {}", e).into()),
                    }
                }
                "create_issue" => {
                    let title = args.get("title")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: title (required for create_issue action)")?;
                    let body = args.get("body")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: body (required for create_issue action)")?;
                    
                    let labels = args.get("labels")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<&str>>()
                        });
                    
                    match github_control::create_issue(title, body, repo, labels) {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to create issue: {}", e).into()),
                    }
                }
                "create_pr" => {
                    let title = args.get("title")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: title (required for create_pr action)")?;
                    let body = args.get("body")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: body (required for create_pr action)")?;
                    let head = args.get("head")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing required parameter: head (required for create_pr action)")?;
                    
                    let base = args.get("base")
                        .and_then(|v| v.as_str());
                    let draft = args.get("draft")
                        .and_then(|v| v.as_bool());
                    
                    match github_control::create_pr(title, body, head, base, repo, draft) {
                        Ok(result) => Ok(result),
                        Err(e) => Err(format!("Failed to create PR: {}", e).into()),
                    }
                }
                _ => Err(format!("Unknown action: {}. Supported actions: view_pr, list_prs, view_issue, list_issues, create_issue, create_pr", action).into()),
            }
        }

        fn name(&self) -> &str {
            "github"
        }
    }
}
