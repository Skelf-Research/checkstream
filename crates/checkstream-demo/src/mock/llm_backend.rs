use crate::mock::IssueInjector;
use crate::models::{ChatMessage, ChatRequest, ChatResponse, IssueConfig};
use parking_lot::RwLock;
use rand::prelude::*;

/// Mock LLM backend that generates configurable responses with issues
pub struct MockLlmBackend {
    injector: RwLock<IssueInjector>,
    issue_config: RwLock<IssueConfig>,
    templates: ResponseTemplates,
}

impl MockLlmBackend {
    pub fn new() -> Self {
        Self {
            injector: RwLock::new(IssueInjector::new()),
            issue_config: RwLock::new(IssueConfig::default()),
            templates: ResponseTemplates::new(),
        }
    }

    /// Update issue configuration
    pub fn set_issue_config(&self, config: IssueConfig) {
        *self.issue_config.write() = config;
    }

    /// Get current issue configuration
    pub fn get_issue_config(&self) -> IssueConfig {
        self.issue_config.read().clone()
    }

    /// Generate a chat completion response
    pub fn chat_completion(&self, request: &ChatRequest) -> ChatResponse {
        let base_response = self.templates.generate_response(&request.messages);
        let config = self.issue_config.read().clone();

        let content = if config.pii_enabled
            || config.toxicity_enabled
            || config.injection_enabled
            || config.financial_advice_enabled
        {
            self.injector.write().inject(&base_response, &config)
        } else {
            base_response
        };

        ChatResponse {
            content,
            finish_reason: "stop".to_string(),
            tokens_used: Some(rand::thread_rng().gen_range(50..500)),
        }
    }

    /// Generate streaming response chunks
    pub fn chat_completion_stream(&self, request: &ChatRequest) -> Vec<String> {
        let response = self.chat_completion(request);

        // Split response into chunks (simulating streaming)
        let words: Vec<&str> = response.content.split_whitespace().collect();
        let chunk_size = 3; // words per chunk

        words
            .chunks(chunk_size)
            .map(|chunk| chunk.join(" ") + " ")
            .collect()
    }
}

impl Default for MockLlmBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Response templates for different conversation types
pub struct ResponseTemplates {
    general_responses: Vec<&'static str>,
    coding_responses: Vec<&'static str>,
    creative_responses: Vec<&'static str>,
}

impl ResponseTemplates {
    pub fn new() -> Self {
        Self {
            general_responses: vec![
                "I'd be happy to help you with that. Let me explain the key concepts involved.",
                "That's an interesting question. Here's what I think about it.",
                "Based on my understanding, I can provide some insights on this topic.",
                "Let me break this down for you step by step.",
                "Great question! There are several aspects to consider here.",
            ],
            coding_responses: vec![
                "Here's a code example that demonstrates the concept. The function takes parameters and returns a result based on the logic.",
                "For this programming task, I recommend using a modular approach. Start by defining your data structures, then implement the core logic.",
                "The implementation involves several steps: first, parse the input data, then validate it, process the transformation, and finally output the result.",
                "This can be solved using a common algorithm pattern. Let me show you how to structure the solution effectively.",
            ],
            creative_responses: vec![
                "Let me craft something creative for you. The story begins in a small town where unexpected events unfold.",
                "Here's an imaginative take on your request. Picture a world where anything is possible.",
                "I'll create something unique for you. The scene opens with a sense of wonder and possibility.",
            ],
        }
    }

    pub fn generate_response(&self, messages: &[ChatMessage]) -> String {
        let mut rng = rand::thread_rng();

        // Detect conversation type from last user message
        let last_message = messages
            .iter()
            .rfind(|m| m.role == "user")
            .map(|m| m.content.to_lowercase())
            .unwrap_or_default();

        let responses = if last_message.contains("code")
            || last_message.contains("function")
            || last_message.contains("program")
            || last_message.contains("implement")
        {
            &self.coding_responses
        } else if last_message.contains("write")
            || last_message.contains("story")
            || last_message.contains("creative")
            || last_message.contains("imagine")
        {
            &self.creative_responses
        } else {
            &self.general_responses
        };

        // Add some variety by combining responses
        let base = responses[rng.gen_range(0..responses.len())];
        let extension = if rng.gen::<f32>() > 0.5 {
            " Additionally, I want to emphasize that this approach has proven effective in many scenarios. Feel free to ask if you need more clarification."
        } else {
            " I hope this helps! Let me know if you have any follow-up questions."
        };

        format!("{}{}", base, extension)
    }
}

impl Default for ResponseTemplates {
    fn default() -> Self {
        Self::new()
    }
}
