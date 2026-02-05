use crate::models::{
    ChatRequest, DemoEvent, DemoMode, DetectedIssue, PipelineStage, RequestAction,
    RequestRecord, RequestResult, TrafficConfig,
};
use crate::state::DemoAppState;
use crate::traffic::RequestTemplates;
use chrono::Utc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Traffic generator that sends requests through the demo pipeline
pub struct TrafficGenerator {
    state: Arc<DemoAppState>,
    templates: RequestTemplates,
}

impl TrafficGenerator {
    pub fn new(state: Arc<DemoAppState>) -> Self {
        Self {
            state,
            templates: RequestTemplates::new(),
        }
    }

    /// Run traffic generation until stopped
    pub async fn run(&self, config: TrafficConfig, mut stop_signal: oneshot::Receiver<()>) {
        let interval = Duration::from_secs_f64(1.0 / config.rate as f64);
        let start_time = Instant::now();

        loop {
            // Check if we should stop
            if stop_signal.try_recv().is_ok() {
                break;
            }

            // Check duration limit
            if let Some(duration) = config.duration_secs {
                if start_time.elapsed() > Duration::from_secs(duration) {
                    break;
                }
            }

            // Check if paused
            if !self.state.traffic_controller.is_running() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Generate and process request
            let request = self.generate_request(&config);
            let result = self.process_request(request).await;

            // Record metrics
            self.state.metrics.record(&result.result);

            // Publish event
            self.state
                .event_bus
                .publish(DemoEvent::RequestCompleted(result.result.clone()));

            // Store in history
            self.state.add_request_record(result);

            // Wait for next interval
            tokio::time::sleep(interval).await;
        }
    }

    fn generate_request(&self, config: &TrafficConfig) -> ChatRequest {
        if config.templates.is_empty() {
            self.templates.generate_random()
        } else {
            let category = &config.templates[rand::random::<usize>() % config.templates.len()];
            self.templates.generate(category)
        }
    }

    async fn process_request(&self, request: ChatRequest) -> RequestRecord {
        let id = Uuid::new_v4().to_string();
        let start_time = Instant::now();
        let timestamp = Utc::now();

        let config = self.state.config.read().clone();
        let mut issues_detected = Vec::new();
        let mut triggered_rules = Vec::new();
        let mut action = RequestAction::Pass;
        let mut pipeline_trace = Vec::new();

        // Phase 1: Ingress - check user input
        let ingress_start = Instant::now();
        let user_content = request
            .messages
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let (ingress_issues, ingress_rules, ingress_action) =
            self.evaluate_content(&user_content, "ingress");
        issues_detected.extend(ingress_issues.clone());
        triggered_rules.extend(ingress_rules.clone());
        if ingress_action != RequestAction::Pass {
            action = ingress_action;
        }

        pipeline_trace.push(PipelineStage {
            phase: "ingress".to_string(),
            duration_ms: ingress_start.elapsed().as_secs_f64() * 1000.0,
            classifiers_run: vec![],
            rules_evaluated: vec![],
        });

        // If blocked at ingress, return early
        if action == RequestAction::Block {
            return self.build_record(
                id,
                timestamp,
                request,
                None,
                action,
                start_time.elapsed().as_secs_f64() * 1000.0,
                "ingress".to_string(),
                issues_detected,
                triggered_rules,
                pipeline_trace,
            );
        }

        // Generate response based on mode
        let response = match config.mode {
            DemoMode::Mock => {
                let resp = self.state.mock_backend.chat_completion(&request);
                Some(resp)
            }
            DemoMode::Proxy => {
                // In proxy mode, we would forward to real backend
                // For now, use mock
                let resp = self.state.mock_backend.chat_completion(&request);
                Some(resp)
            }
        };

        // Phase 2: Midstream/Egress - check response
        if let Some(ref resp) = response {
            let egress_start = Instant::now();
            let (egress_issues, egress_rules, egress_action) =
                self.evaluate_content(&resp.content, "egress");
            issues_detected.extend(egress_issues);
            triggered_rules.extend(egress_rules);
            if egress_action != RequestAction::Pass && action == RequestAction::Pass {
                action = egress_action;
            }

            pipeline_trace.push(PipelineStage {
                phase: "egress".to_string(),
                duration_ms: egress_start.elapsed().as_secs_f64() * 1000.0,
                classifiers_run: vec![],
                rules_evaluated: vec![],
            });
        }

        self.build_record(
            id,
            timestamp,
            request,
            response,
            action,
            start_time.elapsed().as_secs_f64() * 1000.0,
            "egress".to_string(),
            issues_detected,
            triggered_rules,
            pipeline_trace,
        )
    }

    fn evaluate_content(
        &self,
        content: &str,
        _phase: &str,
    ) -> (Vec<DetectedIssue>, Vec<String>, RequestAction) {
        let mut issues = Vec::new();
        let mut rules = Vec::new();
        let mut action = RequestAction::Pass;

        // PII detection (simple patterns)
        let pii_patterns = [
            (r"\d{3}-\d{2}-\d{4}", "ssn", "pii_detector"),
            (r"\d{4}\s?\d{4}\s?\d{4}\s?\d{4}", "credit_card", "pii_detector"),
            (r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", "email", "pii_detector"),
            (r"\(\d{3}\)\s?\d{3}-\d{4}", "phone", "pii_detector"),
        ];

        for (pattern, issue_type, classifier) in pii_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(matched) = re.find(content) {
                    issues.push(DetectedIssue {
                        issue_type: issue_type.to_string(),
                        classifier: classifier.to_string(),
                        score: 0.95,
                        matched_text: Some(matched.as_str().to_string()),
                        span: Some((matched.start(), matched.end())),
                    });
                    rules.push("pii-detection".to_string());
                    action = RequestAction::Redact;
                }
            }
        }

        // Prompt injection detection
        let injection_patterns = [
            r"(?i)ignore\s+(all\s+)?(previous|prior)\s+instructions?",
            r"(?i)disregard\s+(the\s+)?(above|previous)",
            r"(?i)forget\s+(what|everything)",
            r"(?i)system\s*:\s*override",
            r"(?i)\[?admin\s*mode\]?",
        ];

        for pattern in injection_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(matched) = re.find(content) {
                    issues.push(DetectedIssue {
                        issue_type: "prompt_injection".to_string(),
                        classifier: "prompt_injection_detector".to_string(),
                        score: 0.9,
                        matched_text: Some(matched.as_str().to_string()),
                        span: Some((matched.start(), matched.end())),
                    });
                    rules.push("prompt-injection-defense".to_string());
                    action = RequestAction::Block;
                    break;
                }
            }
        }

        // Toxicity detection (keyword-based for demo)
        let toxic_keywords = [
            "stupid", "idiot", "moron", "dumb", "incompetent", "worthless", "garbage", "trash",
            "fool", "pathetic",
        ];

        let content_lower = content.to_lowercase();
        let toxic_count = toxic_keywords
            .iter()
            .filter(|&kw| content_lower.contains(kw))
            .count();

        if toxic_count > 0 {
            let score = (toxic_count as f32 * 0.2).min(1.0);
            issues.push(DetectedIssue {
                issue_type: "toxicity".to_string(),
                classifier: "toxicity_detector".to_string(),
                score,
                matched_text: None,
                span: None,
            });

            if score > 0.8 {
                rules.push("toxicity-filter".to_string());
                if action == RequestAction::Pass {
                    action = RequestAction::Block;
                }
            }
        }

        // Financial advice detection
        let financial_patterns = [
            r"(?i)you\s+should\s+(definitely\s+)?invest",
            r"(?i)i\s+(recommend|advise)\s+(you\s+)?(put|invest|buy)",
            r"(?i)buy\s+\w+\s+stock",
            r"(?i)guaranteed\s+to\s+(go\s+up|increase|profit)",
        ];

        for pattern in financial_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(matched) = re.find(content) {
                    issues.push(DetectedIssue {
                        issue_type: "financial_advice".to_string(),
                        classifier: "financial_advice_detector".to_string(),
                        score: 0.85,
                        matched_text: Some(matched.as_str().to_string()),
                        span: Some((matched.start(), matched.end())),
                    });
                    rules.push("financial-advice-detection".to_string());
                    // Don't block, just flag for demo
                    break;
                }
            }
        }

        (issues, rules, action)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_record(
        &self,
        id: String,
        timestamp: chrono::DateTime<Utc>,
        request: ChatRequest,
        response: Option<crate::models::ChatResponse>,
        action: RequestAction,
        latency_ms: f64,
        phase: String,
        issues_detected: Vec<DetectedIssue>,
        triggered_rules: Vec<String>,
        pipeline_trace: Vec<PipelineStage>,
    ) -> RequestRecord {
        let request_preview = request
            .messages
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.chars().take(100).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ");

        let response_preview = response
            .as_ref()
            .map(|r| r.content.chars().take(100).collect());

        RequestRecord {
            id: id.clone(),
            timestamp,
            request,
            response,
            result: RequestResult {
                id,
                timestamp,
                action,
                latency_ms,
                phase,
                issues_detected,
                triggered_rules,
                request_preview,
                response_preview,
            },
            pipeline_trace,
        }
    }
}
