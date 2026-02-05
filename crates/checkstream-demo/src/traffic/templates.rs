use crate::models::{ChatMessage, ChatRequest};
use rand::prelude::*;

/// Request templates for traffic generation
pub struct RequestTemplates {
    general: Vec<ConversationTemplate>,
    coding: Vec<ConversationTemplate>,
    creative: Vec<ConversationTemplate>,
}

pub struct ConversationTemplate {
    pub system_prompt: Option<String>,
    pub user_prompts: Vec<String>,
}

impl RequestTemplates {
    pub fn new() -> Self {
        Self {
            general: vec![
                ConversationTemplate {
                    system_prompt: Some("You are a helpful assistant.".to_string()),
                    user_prompts: vec![
                        "What is the capital of France?".to_string(),
                        "Explain quantum computing in simple terms.".to_string(),
                        "What are the benefits of regular exercise?".to_string(),
                        "How does the internet work?".to_string(),
                        "What is climate change and why is it important?".to_string(),
                        "Can you explain the basics of nutrition?".to_string(),
                        "What are some tips for better sleep?".to_string(),
                        "How do vaccines work?".to_string(),
                    ],
                },
                ConversationTemplate {
                    system_prompt: Some("You are a knowledgeable tutor.".to_string()),
                    user_prompts: vec![
                        "Help me understand photosynthesis.".to_string(),
                        "What caused World War I?".to_string(),
                        "Explain the theory of relativity.".to_string(),
                        "How do black holes form?".to_string(),
                    ],
                },
            ],
            coding: vec![
                ConversationTemplate {
                    system_prompt: Some("You are a coding assistant.".to_string()),
                    user_prompts: vec![
                        "Write a function to reverse a string in Python.".to_string(),
                        "How do I implement a binary search tree?".to_string(),
                        "Explain the difference between REST and GraphQL.".to_string(),
                        "What is the best way to handle errors in Rust?".to_string(),
                        "Write a simple HTTP server in Node.js.".to_string(),
                        "How do I optimize database queries?".to_string(),
                        "Implement a sorting algorithm in JavaScript.".to_string(),
                        "What are best practices for API design?".to_string(),
                    ],
                },
            ],
            creative: vec![
                ConversationTemplate {
                    system_prompt: Some("You are a creative writing assistant.".to_string()),
                    user_prompts: vec![
                        "Write a short story about a robot learning to love.".to_string(),
                        "Create a poem about the ocean at sunset.".to_string(),
                        "Imagine a world where time flows backwards.".to_string(),
                        "Write a dialogue between the sun and the moon.".to_string(),
                        "Create a mystery story opening paragraph.".to_string(),
                    ],
                },
            ],
        }
    }

    /// Generate a random request based on template category
    pub fn generate(&self, category: &str) -> ChatRequest {
        let mut rng = rand::thread_rng();

        let templates = match category {
            "coding" => &self.coding,
            "creative" => &self.creative,
            _ => &self.general,
        };

        let template = &templates[rng.gen_range(0..templates.len())];
        let user_prompt = &template.user_prompts[rng.gen_range(0..template.user_prompts.len())];

        let mut messages = Vec::new();

        if let Some(system) = &template.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_prompt.clone(),
        });

        ChatRequest {
            model: "gpt-4".to_string(),
            messages,
            stream: false,
        }
    }

    /// Generate a random request from any category
    pub fn generate_random(&self) -> ChatRequest {
        let mut rng = rand::thread_rng();
        let categories = ["general", "coding", "creative"];
        let category = categories[rng.gen_range(0..categories.len())];
        self.generate(category)
    }
}

impl Default for RequestTemplates {
    fn default() -> Self {
        Self::new()
    }
}
