//! Test loading and running a real HuggingFace model
//!
//! This example downloads a small DistilBERT sentiment classifier and tests it.
//! Run with: cargo run --example test_hf_model --features ml-models

use checkstream_classifiers::generic_loader::GenericModelLoader;
use checkstream_classifiers::model_config::ModelRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with DEBUG level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== HuggingFace Model Test ===\n");

    // Define model configuration for DistilBERT SST-2 sentiment classifier
    // This is a small (~260MB) model that works well for testing
    let config_yaml = r#"
version: "1.0"
models:
  sentiment:
    name: "distilbert-sst2"
    source:
      type: huggingface
      repo: "distilbert-base-uncased-finetuned-sst-2-english"
      revision: "main"
    architecture:
      type: distil-bert-sequence-classification
      num_labels: 2
      labels: ["negative", "positive"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#;

    // Parse the configuration
    let registry: ModelRegistry = serde_yaml::from_str(config_yaml)?;
    let loader = GenericModelLoader::new(registry);

    println!("Loading sentiment classifier from HuggingFace...");
    println!("(This may take a minute on first run to download the model)\n");

    let classifier = loader.load_classifier("sentiment").await?;

    println!("Model loaded successfully!\n");
    println!("Running inference tests:\n");

    // Test cases
    let test_cases = vec![
        ("I love this movie, it's absolutely fantastic!", "positive"),
        ("This is the worst experience I've ever had.", "negative"),
        ("The weather is nice today.", "neutral/positive"),
        ("I hate waiting in long lines.", "negative"),
        ("The food was okay, nothing special.", "neutral"),
        ("This product exceeded all my expectations!", "positive"),
        ("I'm so disappointed with the service.", "negative"),
        ("What a beautiful day to be alive!", "positive"),
    ];

    for (text, expected) in test_cases {
        let result = classifier.classify(text).await?;

        let correct = if expected.contains(&result.label.to_lowercase())
            || (expected == "neutral/positive" && result.label == "positive")
            || (expected == "neutral" && result.score < 0.7)
        {
            "✓"
        } else {
            "?"
        };

        println!("{} Input: \"{}\"", correct, text);
        println!("  Predicted: {} (score: {:.3})", result.label, result.score);

        if let Some(all_scores) = &result.metadata.all_scores {
            print!("  All scores: ");
            for (label, score) in all_scores {
                print!("{}={:.3} ", label, score);
            }
            println!();
        }

        println!("  Latency: {}µs", result.latency_us);
        println!();
    }

    println!("\n=== Test Complete ===");

    Ok(())
}
