//! Example: Using Classifier Pipelines
//!
//! This example demonstrates how to:
//! 1. Load classifier configuration from YAML
//! 2. Build pipelines from configuration
//! 3. Execute pipelines with real text
//! 4. Analyze results and timing

use checkstream_classifiers::{
    build_pipeline_from_config, load_config, ClassificationResult, Classifier, ClassifierTier,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock classifier for demonstration
struct DemoClassifier {
    name: String,
    base_score: f32,
    tier: ClassifierTier,
}

impl DemoClassifier {
    fn new(name: &str, base_score: f32, tier: ClassifierTier) -> Self {
        Self {
            name: name.to_string(),
            base_score,
            tier,
        }
    }
}

#[async_trait::async_trait]
impl Classifier for DemoClassifier {
    async fn classify(&self, text: &str) -> checkstream_core::Result<ClassificationResult> {
        // Simulate different scoring based on text content
        let score = if text.to_lowercase().contains("toxic") {
            0.9
        } else if text.to_lowercase().contains("suspicious") {
            0.6
        } else {
            self.base_score
        };

        // Simulate processing time based on tier
        let latency_us = match self.tier {
            ClassifierTier::A => 1500, // <2ms
            ClassifierTier::B => 4000, // <5ms
            ClassifierTier::C => 8000, // <10ms
        };

        Ok(ClassificationResult {
            label: if score >= 0.5 { "positive" } else { "negative" }.to_string(),
            score,
            metadata: Default::default(),
            latency_us,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        self.tier
    }
}

#[tokio::main]
async fn main() -> checkstream_core::Result<()> {
    println!("🚀 CheckStream Classifier Pipeline Example\n");

    // Step 1: Create demo classifiers
    println!("📦 Setting up classifiers...");
    let mut classifiers: HashMap<String, Arc<dyn Classifier>> = HashMap::new();

    classifiers.insert(
        "toxicity".to_string(),
        Arc::new(DemoClassifier::new("toxicity", 0.2, ClassifierTier::B)),
    );
    classifiers.insert(
        "toxicity-distilled".to_string(),
        Arc::new(DemoClassifier::new(
            "toxicity-distilled",
            0.2,
            ClassifierTier::A,
        )),
    );
    classifiers.insert(
        "sentiment".to_string(),
        Arc::new(DemoClassifier::new("sentiment", 0.3, ClassifierTier::B)),
    );
    classifiers.insert(
        "prompt-injection".to_string(),
        Arc::new(DemoClassifier::new(
            "prompt-injection",
            0.1,
            ClassifierTier::B,
        )),
    );
    classifiers.insert(
        "financial-advice".to_string(),
        Arc::new(DemoClassifier::new(
            "financial-advice",
            0.15,
            ClassifierTier::C,
        )),
    );
    classifiers.insert(
        "readability".to_string(),
        Arc::new(DemoClassifier::new("readability", 0.5, ClassifierTier::C)),
    );

    println!("✓ Created {} classifiers\n", classifiers.len());

    // Step 2: Load configuration
    println!("📄 Loading classifier configuration...");
    let config = load_config("./classifiers.yaml")?;
    println!("✓ Loaded configuration");
    println!("  - Models: {:?}", config.model_names());
    println!("  - Pipelines: {:?}\n", config.pipeline_names());

    // Step 3: Build and test different pipelines
    let test_cases = vec![
        ("basic-safety", "This is a normal message", "Clean input"),
        (
            "basic-safety",
            "This is a toxic message that should be flagged",
            "Toxic content",
        ),
        (
            "advanced-safety",
            "This looks suspicious",
            "Suspicious content",
        ),
        ("fast-triage", "Another toxic message", "Fast detection"),
    ];

    for (pipeline_name, text, description) in test_cases {
        println!("─────────────────────────────────────────────────");
        println!("🔍 Testing Pipeline: {}", pipeline_name);
        println!("📝 Input: \"{}\"", text);
        println!("💡 Scenario: {}", description);

        // Get pipeline configuration
        let pipeline_spec = config.get_pipeline(pipeline_name).ok_or_else(|| {
            checkstream_core::Error::config(format!("Pipeline '{}' not found", pipeline_name))
        })?;

        // Build pipeline from configuration
        let pipeline = build_pipeline_from_config(pipeline_spec, &classifiers)?;

        // Execute pipeline
        let result = pipeline.execute(text).await?;

        // Display results
        println!("\n📊 Results:");
        println!(
            "  Total latency: {}μs ({:.2}ms)",
            result.total_latency_us,
            result.total_latency_us as f64 / 1000.0
        );
        println!("  Stages executed: {}", result.results.len());

        for stage_result in &result.results {
            println!(
                "    ├─ {}: {} (score: {:.2}, {}μs)",
                stage_result.stage_name,
                stage_result.classifier_name,
                stage_result.result.score,
                stage_result.stage_latency_us
            );
        }

        if let Some(decision) = &result.final_decision {
            println!(
                "\n  🎯 Final Decision: {} (score: {:.2})",
                decision.label, decision.score
            );

            if decision.score >= 0.5 {
                println!("  ⚠️  Action: FLAG for review");
            } else {
                println!("  ✅ Action: PASS");
            }
        }

        println!();
    }

    println!("─────────────────────────────────────────────────");
    println!("\n✨ Pipeline Comparison\n");

    // Compare different pipeline strategies
    let comparison_text = "This is a suspicious and somewhat toxic message";
    println!("Testing text: \"{}\"\n", comparison_text);

    let pipelines_to_compare = vec![
        "basic-safety",
        "advanced-safety",
        "fast-triage",
        "weighted-analysis",
    ];

    println!("Pipeline             | Latency  | Stages | Final Score | Decision");
    println!("---------------------|----------|--------|-------------|----------");

    for pipeline_name in pipelines_to_compare {
        if let Some(pipeline_spec) = config.get_pipeline(pipeline_name) {
            let pipeline = build_pipeline_from_config(pipeline_spec, &classifiers)?;
            let result = pipeline.execute(comparison_text).await?;

            let latency_ms = result.total_latency_us as f64 / 1000.0;
            let stages = result.results.len();
            let (score, decision) = if let Some(d) = &result.final_decision {
                (d.score, d.label.as_str())
            } else {
                (0.0, "none")
            };

            println!(
                "{:<20} | {:>6.2}ms | {:>6} | {:>11.2} | {}",
                pipeline_name, latency_ms, stages, score, decision
            );
        }
    }

    println!("\n💡 Key Insights:");
    println!("  - Parallel execution keeps latency low (max of concurrent classifiers)");
    println!("  - Conditional stages save compute on clean inputs");
    println!("  - Different aggregation strategies produce different results");
    println!("  - All pipelines stay within CheckStream's <10ms target");

    println!("\n🎉 Example complete!");

    Ok(())
}
