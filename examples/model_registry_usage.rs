use checkstream_classifiers::model_config::ModelRegistry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("CheckStream Model Registry Example");
    println!("===================================\n");

    // Load model registry from YAML
    let registry = ModelRegistry::from_file("models/registry.yaml")?;

    println!("Registry version: {}", registry.version);
    println!("Available models: {}\n", registry.models.len());

    // List all models
    for (name, config) in &registry.models {
        println!("Model: {}", name);
        println!("  Name: {}", config.name);
        println!("  Version: {}", config.version);
        println!("  Description: {}", config.description);

        // Show source
        match &config.source {
            checkstream_classifiers::model_config::ModelSource::Local { path } => {
                println!("  Source: Local ({})", path.display());
            }
            checkstream_classifiers::model_config::ModelSource::HuggingFace { repo, revision } => {
                println!("  Source: HuggingFace ({} @ {})", repo, revision);
            }
            checkstream_classifiers::model_config::ModelSource::Builtin { implementation } => {
                println!("  Source: Built-in ({})", implementation);
            }
        }

        // Show architecture
        match &config.architecture {
            checkstream_classifiers::model_config::ArchitectureConfig::BertSequenceClassification { num_labels, labels } => {
                println!("  Architecture: BERT Sequence Classification");
                println!("  Num Labels: {}", num_labels);
                if !labels.is_empty() {
                    println!("  Labels: {:?}", labels);
                }
            }
            checkstream_classifiers::model_config::ArchitectureConfig::DistilBertSequenceClassification { num_labels, .. } => {
                println!("  Architecture: DistilBERT Sequence Classification");
                println!("  Num Labels: {}", num_labels);
            }
            _ => {
                println!("  Architecture: {:?}", config.architecture);
            }
        }

        // Show inference config
        println!("  Inference:");
        println!("    Device: {}", config.inference.device);
        println!("    Max Length: {}", config.inference.max_length);
        println!("    Threshold: {}", config.inference.threshold);

        println!();
    }

    // Example: Get specific model
    if let Some(toxicity_model) = registry.get_model("toxicity") {
        println!("✓ Found toxicity model: {}", toxicity_model.name);
        println!("  This model would be loaded dynamically when needed");
    }

    Ok(())
}
