use checkstream_classifiers::toxicity::ToxicityClassifier;
use checkstream_classifiers::Classifier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Creating toxicity classifier...");
    let classifier = ToxicityClassifier::new()?;

    println!("\nTesting with safe text:");
    let result = classifier
        .classify("This is a nice friendly message")
        .await?;
    println!(
        "  Label: {}, Score: {:.3}, Model: {:?}, Latency: {}µs",
        result.label, result.score, result.metadata.model, result.latency_us
    );

    println!("\nTesting with toxic text:");
    let result = classifier.classify("I hate you, you stupid idiot!").await?;
    println!(
        "  Label: {}, Score: {:.3}, Model: {:?}, Latency: {}µs",
        result.label, result.score, result.metadata.model, result.latency_us
    );

    println!("\nTesting with mildly toxic text:");
    let result = classifier.classify("This is terrible and awful").await?;
    println!(
        "  Label: {}, Score: {:.3}, Model: {:?}, Latency: {}µs",
        result.label, result.score, result.metadata.model, result.latency_us
    );

    Ok(())
}
