use checkstream_classifiers::dynamic_registry::DynamicRegistryBuilder;
use checkstream_classifiers::pii::PiiClassifier;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Full Dynamic Pipeline Example");
    println!("==============================\n");

    // Build registry with both ML models (from YAML) and built-in classifiers
    println!("Building dynamic classifier registry...");
    let registry = DynamicRegistryBuilder::new()
        .with_model_registry("models/registry.yaml")
        .with_builtin("pii", Arc::new(PiiClassifier::new()?))
        .preload("toxicity") // Preload toxicity model
        .build()
        .await?;

    println!("✓ Registry built\n");

    // List available models
    println!("Available models:");
    for model in registry.available_models() {
        println!("  - {}", model);
    }
    println!();

    // Test 1: Use built-in PII classifier
    println!("Test 1: Built-in PII Classifier");
    println!("--------------------------------");
    let pii_classifier = registry.get_classifier("pii").await?;
    let text = "My email is john@example.com and SSN is 123-45-6789";
    let result = pii_classifier.classify(text).await?;
    println!("Input: \"{}\"", text);
    println!("Label: {}", result.label);
    println!("Score: {:.3}", result.score);
    println!("Latency: {}µs\n", result.latency_us);

    // Test 2: Use dynamically loaded toxicity classifier
    println!("Test 2: Dynamic ML Toxicity Classifier");
    println!("---------------------------------------");
    let toxicity_classifier = registry.get_classifier("toxicity").await?;
    let text = "You're so stupid and worthless!";
    let result = toxicity_classifier.classify(text).await?;
    println!("Input: \"{}\"", text);
    println!("Label: {}", result.label);
    println!("Score: {:.3}", result.score);
    println!("Model: {:?}", result.metadata.model);
    println!("Latency: {}µs\n", result.latency_us);

    // Test 3: Get same classifier again (should use cache)
    println!("Test 3: Cached Classifier (Instant Load)");
    println!("-----------------------------------------");
    let start = std::time::Instant::now();
    let _toxicity_classifier2 = registry.get_classifier("toxicity").await?;
    let load_time = start.elapsed();
    println!("Load time: {:?} (should be <1ms from cache)", load_time);

    println!("✅ Full dynamic pipeline working!");
    println!("\n💡 Key Benefits:");
    println!("   ✓ Mix built-in (pattern) and ML (dynamic) classifiers");
    println!("   ✓ Lazy loading - models load on first use");
    println!("   ✓ Automatic caching - instant on subsequent access");
    println!("   ✓ No code changes to add new models");
    println!("   ✓ All configuration in YAML files");

    Ok(())
}
