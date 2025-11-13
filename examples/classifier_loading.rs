//! Example: Loading and using Candle models from configuration
//!
//! This example shows how to:
//! 1. Load classifier configuration from YAML
//! 2. Initialize model registry
//! 3. Use models for classification
//!
//! Run with: cargo run --example classifier_loading

use checkstream_classifiers::{init_registry_from_file, load_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("CheckStream Classifier Loading Example\n");

    // 1. Load classifier configuration
    println!("Loading classifier configuration from classifiers.yaml...");
    let config = load_config("./classifiers.yaml")?;

    println!("Configuration loaded:");
    println!("  - Models defined: {}", config.models.len());
    println!("  - Default device: {:?}", config.default_device);
    println!("  - Default quantize: {}", config.default_quantize);
    println!("  - Models directory: {:?}\n", config.models_dir);

    // 2. List configured models
    println!("Configured models:");
    for (name, spec) in &config.models {
        println!("  - {}", name);
        match &spec.source {
            checkstream_classifiers::ModelSourceSpec::Local { path } => {
                println!("      Source: Local file ({:?})", path);
            }
            checkstream_classifiers::ModelSourceSpec::HuggingFace {
                repo_id,
                filename,
                ..
            } => {
                println!("      Source: Hugging Face");
                println!("      Repo: {}", repo_id);
                println!("      File: {}", filename);
            }
        }
        if let Some(tier) = &spec.tier {
            println!("      Tier: {}", tier);
        }
    }
    println!();

    // 3. Initialize model registry
    // Note: This will actually download and load models, which may take time
    // Uncomment the following to actually load models:

    /*
    println!("Initializing model registry...");
    println!("(This may take a while for first-time downloads)\n");

    let registry = init_registry_from_file("./classifiers.yaml")?;

    println!("Model registry initialized!");
    println!("Loaded models: {:?}\n", registry.model_names());

    // 4. Retrieve and use a model
    if let Some(model) = registry.get("toxicity") {
        println!("Retrieved 'toxicity' model:");
        println!("  Device: {:?}", model.device());
        println!("  Has tokenizer: {}", model.has_tokenizer());
        println!("  Metadata: {:?}", model.metadata());

        // You can now use this model for inference
        // See other examples for actual classification
    }
    */

    println!("Example complete!");
    println!("\nTo actually load models, uncomment the loading section in the code.");
    println!("Note: First-time loading will download models from Hugging Face.");

    Ok(())
}
