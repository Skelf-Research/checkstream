use checkstream_classifiers::generic_loader::GenericModelLoader;
use checkstream_classifiers::model_config::ModelRegistry;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Dynamic Model Loading Example");
    println!("==============================\n");

    // Load model registry from YAML
    println!("Loading model registry from models/registry.yaml...");
    let registry = ModelRegistry::from_file("models/registry.yaml")?;
    println!(
        "✓ Registry loaded: {} models available\n",
        registry.models.len()
    );

    // Create generic model loader
    let loader = GenericModelLoader::new(registry);

    // Load toxicity classifier dynamically
    println!("Loading 'toxicity' classifier from registry...");
    match loader.load_classifier("toxicity").await {
        Ok(classifier) => {
            println!("✓ Classifier loaded: {}", classifier.name());
            println!("  Tier: {:?}\n", classifier.tier());

            // Test with safe text
            println!("Testing with safe text:");
            let text = "This is a friendly and helpful message";
            let result = classifier.classify(text).await?;
            println!("  Input: \"{}\"", text);
            println!("  Label: {}", result.label);
            println!("  Score: {:.3}", result.score);
            println!("  Model: {:?}", result.metadata.model);
            println!("  Latency: {}µs\n", result.latency_us);

            // Test with toxic text
            println!("Testing with toxic text:");
            let text = "I hate you, you stupid idiot!";
            let result = classifier.classify(text).await?;
            println!("  Input: \"{}\"", text);
            println!("  Label: {}", result.label);
            println!("  Score: {:.3}", result.score);
            println!("  Model: {:?}", result.metadata.model);
            println!("  Latency: {}µs\n", result.latency_us);

            println!("✅ Dynamic model loading working!");
            println!("\n💡 Key Points:");
            println!("   - Model loaded from YAML configuration");
            println!("   - No custom code needed for standard BERT models");
            println!("   - Just edit models/registry.yaml to swap models");
        }
        Err(e) => {
            eprintln!("❌ Failed to load classifier: {}", e);
            eprintln!("\nNote: Make sure the model exists at ./models/toxic-bert/");
            eprintln!("Run: ./scripts/download_models.sh");
        }
    }

    Ok(())
}
