pub mod pengy_agent {
    use crate::agent::agent::agent::AgentEvent;
    use crate::agent::code_researcher::code_researcher::create_code_researcher_agent;
    use crate::agent::coder::coder::create_coder_agent;
    use crate::agent::test_agent::test_agent::create_test_agent;
    use crate::model::model::model::{Message, Model, Role};
    use crate::prompt::pengy::{implementation_prompt, research_prompt, testing_prompt};

    /// Helper function to extract the final response from an agent's messages
    fn extract_final_response(messages: &[Message]) -> Option<String> {
        // Look for the last assistant message that doesn't start with "Tool call:"
        for msg in messages.iter().rev() {
            if matches!(msg.role, Role::Assistant) && !msg.content.starts_with("Tool call:") {
                return Some(msg.content.clone());
            }
        }
        None
    }

    /// Creates and runs a Pengy Agent that orchestrates three agents sequentially:
    /// 1. Code Researcher - researches the codebase and generates a research report
    /// 2. Coder - implements code based on the research report
    /// 3. Test Agent - tests the implemented code
    ///
    /// This is a meta-agent that coordinates the full development workflow.
    pub async fn run_pengy_agent<F>(
        model: Model,
        api_key: String,
        base_url: String,
        embedding_model: Option<String>,
        user_request: String,
        conversation_history: Option<String>,
        max_retry: Option<u32>,
        max_step: Option<u32>,
        callback: F,
    ) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static + Clone,
    {
        callback(AgentEvent::Thinking {
            content: "=== PENGY AGENT: Starting Orchestration ===".to_string(),
        });

        // Step 1: Run Code Researcher Agent
        callback(AgentEvent::Thinking {
            content: "=== PHASE 1: Code Research ===".to_string(),
        });
        let research_prompt = research_prompt(&user_request, conversation_history.as_deref());

        let mut researcher_agent = create_code_researcher_agent(
            model.clone(),
            api_key.clone(),
            base_url.clone(),
            embedding_model.clone(),
            None, // Use default system prompt
            max_retry,
            max_step,
        );

        researcher_agent
            .run(research_prompt, callback.clone())
            .await;

        // Extract research report
        let research_report = extract_final_response(researcher_agent.get_messages())
            .unwrap_or_else(|| "Research completed but no final report was generated.".to_string());

        callback(AgentEvent::Thinking {
            content: format!("Research Report Generated:\n{}", research_report),
        });

        // Step 2: Run Coder Agent with research report
        callback(AgentEvent::Thinking {
            content: "=== PHASE 2: Code Implementation ===".to_string(),
        });
        let implementation_prompt =
            implementation_prompt(&user_request, &research_report, conversation_history.as_deref());

        let mut coder_agent = create_coder_agent(
            model.clone(),
            None, // Use default system prompt
            max_retry,
            max_step,
        );

        coder_agent
            .run(implementation_prompt, callback.clone())
            .await;

        // Extract implementation summary
        let implementation_summary = extract_final_response(coder_agent.get_messages())
            .unwrap_or_else(|| "Code implementation completed.".to_string());

        callback(AgentEvent::Thinking {
            content: format!("Implementation Summary:\n{}", implementation_summary),
        });

        // Step 3: Run Test Agent
        callback(AgentEvent::Thinking {
            content: "=== PHASE 3: Testing ===".to_string(),
        });
        let testing_prompt = testing_prompt(
            &user_request,
            &research_report,
            &implementation_summary,
            conversation_history.as_deref(),
        );

        let mut test_agent = create_test_agent(
            model.clone(),
            None, // Use default system prompt
            max_retry,
            max_step,
        );

        test_agent.run(testing_prompt, callback.clone()).await;

        // Extract test results
        let test_results = extract_final_response(test_agent.get_messages())
            .unwrap_or_else(|| "Testing completed.".to_string());

        callback(AgentEvent::Thinking {
            content: format!("Test Results:\n{}", test_results),
        });

        // Compile final summary
        let final_summary = format!(
            "=== PENGY AGENT: Complete Workflow Summary ===\n\n\
            PHASE 1 - RESEARCH:\n{}\n\n\
            PHASE 2 - IMPLEMENTATION:\n{}\n\n\
            PHASE 3 - TESTING:\n{}\n\n\
            === Workflow Complete ===",
            research_report, implementation_summary, test_results
        );

        Ok(final_summary)
    }
}
