pub mod github_control {
    use std::error::Error;
    use std::process::Command;

    /// View details of a specific pull request
    ///
    /// # Arguments
    /// * `pr_number` - The PR number to view
    /// * `repo` - Optional repository in format "owner/repo". If None, uses current repo
    ///
    /// # Returns
    /// JSON string containing PR details
    pub fn view_pr(pr_number: u64, repo: Option<&str>) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::new("gh");
        cmd.arg("pr");
        cmd.arg("view");
        cmd.arg(pr_number.to_string());
        cmd.arg("--json");
        cmd.arg("number,title,body,state,author,createdAt,updatedAt,headRefName,baseRefName,url,mergeable,isDraft,labels,reviewDecision");

        if let Some(repo) = repo {
            cmd.arg("--repo");
            cmd.arg(repo);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to view PR: {}", error_msg).into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }

    /// List all pull requests
    ///
    /// # Arguments
    /// * `state` - Filter by state: "open", "closed", or "all" (default: "open")
    /// * `repo` - Optional repository in format "owner/repo". If None, uses current repo
    /// * `limit` - Optional limit on number of PRs to return
    ///
    /// # Returns
    /// JSON string containing array of PR details
    pub fn list_prs(
        state: Option<&str>,
        repo: Option<&str>,
        limit: Option<u32>,
    ) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::new("gh");
        cmd.arg("pr");
        cmd.arg("list");
        cmd.arg("--json");
        cmd.arg("number,title,state,author,createdAt,updatedAt,headRefName,baseRefName,url,isDraft,labels");

        if let Some(state) = state {
            cmd.arg("--state");
            cmd.arg(state);
        } else {
            cmd.arg("--state");
            cmd.arg("open");
        }

        if let Some(repo) = repo {
            cmd.arg("--repo");
            cmd.arg(repo);
        }

        if let Some(limit) = limit {
            cmd.arg("--limit");
            cmd.arg(limit.to_string());
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to list PRs: {}", error_msg).into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }

    /// View details of a specific issue
    ///
    /// # Arguments
    /// * `issue_number` - The issue number to view
    /// * `repo` - Optional repository in format "owner/repo". If None, uses current repo
    ///
    /// # Returns
    /// JSON string containing issue details
    pub fn view_issue(issue_number: u64, repo: Option<&str>) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue");
        cmd.arg("view");
        cmd.arg(issue_number.to_string());
        cmd.arg("--json");
        cmd.arg("number,title,body,state,author,createdAt,updatedAt,url,labels,assignees,comments,closed");

        if let Some(repo) = repo {
            cmd.arg("--repo");
            cmd.arg(repo);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to view issue: {}", error_msg).into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }

    /// List all issues
    ///
    /// # Arguments
    /// * `state` - Filter by state: "open", "closed", or "all" (default: "open")
    /// * `repo` - Optional repository in format "owner/repo". If None, uses current repo
    /// * `limit` - Optional limit on number of issues to return
    ///
    /// # Returns
    /// JSON string containing array of issue details
    pub fn list_issues(
        state: Option<&str>,
        repo: Option<&str>,
        limit: Option<u32>,
    ) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue");
        cmd.arg("list");
        cmd.arg("--json");
        cmd.arg("number,title,state,author,createdAt,updatedAt,url,labels,assignees");

        if let Some(state) = state {
            cmd.arg("--state");
            cmd.arg(state);
        } else {
            cmd.arg("--state");
            cmd.arg("open");
        }

        if let Some(repo) = repo {
            cmd.arg("--repo");
            cmd.arg(repo);
        }

        if let Some(limit) = limit {
            cmd.arg("--limit");
            cmd.arg(limit.to_string());
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to list issues: {}", error_msg).into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }

    /// Create a new issue
    ///
    /// # Arguments
    /// * `title` - The title of the issue
    /// * `body` - The body/description of the issue
    /// * `repo` - Optional repository in format "owner/repo". If None, uses current repo
    /// * `labels` - Optional vector of label names to add to the issue
    ///
    /// # Returns
    /// JSON string containing the created issue details
    pub fn create_issue(
        title: &str,
        body: &str,
        repo: Option<&str>,
        labels: Option<Vec<&str>>,
    ) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue");
        cmd.arg("create");
        cmd.arg("--title");
        cmd.arg(title);
        cmd.arg("--body");
        cmd.arg(body);
        cmd.arg("--json");
        cmd.arg("number,title,body,state,author,createdAt,url,labels");

        if let Some(repo) = repo {
            cmd.arg("--repo");
            cmd.arg(repo);
        }

        if let Some(labels) = labels {
            if !labels.is_empty() {
                cmd.arg("--label");
                cmd.arg(labels.join(","));
            }
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create issue: {}", error_msg).into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }

    /// Create a new pull request
    ///
    /// # Arguments
    /// * `title` - The title of the PR
    /// * `body` - The body/description of the PR
    /// * `head` - The branch to merge from (e.g., "feature-branch" or "owner:feature-branch")
    /// * `base` - The branch to merge into (default: "main" or "master")
    /// * `repo` - Optional repository in format "owner/repo". If None, uses current repo
    /// * `draft` - Whether to create as a draft PR (default: false)
    ///
    /// # Returns
    /// JSON string containing the created PR details
    pub fn create_pr(
        title: &str,
        body: &str,
        head: &str,
        base: Option<&str>,
        repo: Option<&str>,
        draft: Option<bool>,
    ) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::new("gh");
        cmd.arg("pr");
        cmd.arg("create");
        cmd.arg("--title");
        cmd.arg(title);
        cmd.arg("--body");
        cmd.arg(body);
        cmd.arg("--head");
        cmd.arg(head);
        cmd.arg("--json");
        cmd.arg(
            "number,title,body,state,author,createdAt,headRefName,baseRefName,url,isDraft,labels",
        );

        if let Some(base) = base {
            cmd.arg("--base");
            cmd.arg(base);
        }

        if let Some(repo) = repo {
            cmd.arg("--repo");
            cmd.arg(repo);
        }

        if draft.unwrap_or(false) {
            cmd.arg("--draft");
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create PR: {}", error_msg).into());
        }

        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout)
    }
}
