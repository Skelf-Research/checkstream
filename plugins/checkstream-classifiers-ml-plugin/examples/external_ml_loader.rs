use checkstream_classifiers::dynamic_registry::DynamicRegistryBuilder;
use checkstream_classifiers_ml_plugin::ExternalMlModelLoader;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let loader = ExternalMlModelLoader::from_file("models/registry.yaml")?;

    let registry = DynamicRegistryBuilder::new()
        .with_loader(Arc::new(loader))
        .preload("toxicity")
        .build()
        .await?;

    let classifier = registry.get_classifier("toxicity").await?;
    let result = classifier.classify("You are a terrible person").await?;

    println!(
        "label={} score={:.3} latency_us={}",
        result.label, result.score, result.latency_us
    );

    Ok(())
}
