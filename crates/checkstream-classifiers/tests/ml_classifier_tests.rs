//! ML Classifier Integration Tests
//!
//! Tests for the ML-based classifiers using real models from HuggingFace.
//! These tests require the `ml-models` feature flag.

#![cfg(feature = "ml-models")]

use checkstream_classifiers::generic_loader::GenericModelLoader;
use checkstream_classifiers::model_config::ModelRegistry;
use checkstream_classifiers::Classifier;

/// Test configuration for DistilBERT sentiment classifier
fn sentiment_config() -> &'static str {
    r#"
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
"#
}

#[tokio::test]
async fn test_load_sentiment_model() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);

    let result = loader.load_classifier("sentiment").await;
    assert!(result.is_ok(), "Failed to load sentiment model: {:?}", result.err());
}

#[tokio::test]
async fn test_sentiment_positive() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let result = classifier.classify("I love this movie, it's absolutely fantastic!").await.unwrap();

    assert_eq!(result.label, "positive");
    assert!(result.score > 0.9, "Expected high positive score, got {}", result.score);
}

#[tokio::test]
async fn test_sentiment_negative() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let result = classifier.classify("This is the worst experience I've ever had.").await.unwrap();

    assert_eq!(result.label, "negative");
    // For negative sentiment, the "positive" score should be very low
    assert!(result.score < 0.1, "Expected low positive score for negative text, got {}", result.score);
}

#[tokio::test]
async fn test_sentiment_batch_classification() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let test_cases = vec![
        ("I absolutely love this product!", "positive"),
        ("Terrible service, never coming back.", "negative"),
        ("What a wonderful day!", "positive"),
        ("I hate waiting in long lines.", "negative"),
        ("This exceeded all my expectations!", "positive"),
        ("I'm so disappointed with the quality.", "negative"),
    ];

    for (text, expected_sentiment) in test_cases {
        let result = classifier.classify(text).await.unwrap();
        assert_eq!(
            result.label, expected_sentiment,
            "Text '{}' expected {}, got {} (score: {})",
            text, expected_sentiment, result.label, result.score
        );
    }
}

#[tokio::test]
async fn test_classifier_metadata() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let result = classifier.classify("Test input").await.unwrap();

    // Check that all_scores is populated
    assert!(result.metadata.all_scores.is_some(), "Expected all_scores metadata");

    let all_scores = result.metadata.all_scores.as_ref().unwrap();
    assert_eq!(all_scores.len(), 2, "Expected 2 labels (negative, positive)");

    // Scores should sum to approximately 1.0 (softmax output)
    let total: f32 = all_scores.iter().map(|(_, s)| s).sum();
    assert!((total - 1.0).abs() < 0.01, "Scores should sum to ~1.0, got {}", total);
}

#[tokio::test]
async fn test_classifier_latency() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let result = classifier.classify("Test input for latency measurement").await.unwrap();

    // Latency should be recorded and reasonable (< 1 second on CPU)
    assert!(result.latency_us > 0, "Latency should be recorded");
    assert!(result.latency_us < 1_000_000, "Latency should be < 1 second, got {}us", result.latency_us);
}

#[tokio::test]
async fn test_classifier_tier() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    // ML models should be Tier B
    assert_eq!(classifier.tier(), checkstream_classifiers::ClassifierTier::B);
}

#[tokio::test]
async fn test_long_input_truncation() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    // Create a very long input (should be truncated to max_length)
    let long_input = "I love this! ".repeat(1000);

    let result = classifier.classify(&long_input).await;
    assert!(result.is_ok(), "Should handle long inputs via truncation");
}

#[tokio::test]
async fn test_empty_input() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let result = classifier.classify("").await;
    // Empty input should still produce a result (tokenizer adds [CLS] and [SEP])
    assert!(result.is_ok(), "Should handle empty inputs");
}

#[tokio::test]
async fn test_special_characters() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);
    let classifier = loader.load_classifier("sentiment").await.unwrap();

    let inputs = vec![
        "I love this! @#$%^&*()",
        "Great product!!! 😊",
        "Best <html>ever</html>",
        "Love it\n\nSo much\ttabs",
    ];

    for input in inputs {
        let result = classifier.classify(input).await;
        assert!(result.is_ok(), "Should handle special characters in: {}", input);
    }
}

#[tokio::test]
async fn test_model_not_found() {
    let config = r#"
version: "1.0"
models:
  nonexistent:
    name: "fake-model"
    source:
      type: huggingface
      repo: "this-model-does-not-exist-12345"
      revision: "main"
    architecture:
      type: distil-bert-sequence-classification
      num_labels: 2
      labels: ["no", "yes"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#;

    let registry: ModelRegistry = serde_yaml::from_str(config).unwrap();
    let loader = GenericModelLoader::new(registry);

    let result = loader.load_classifier("nonexistent").await;
    assert!(result.is_err(), "Should fail for non-existent model");
}

#[tokio::test]
async fn test_model_name_not_in_registry() {
    let registry: ModelRegistry = serde_yaml::from_str(sentiment_config()).unwrap();
    let loader = GenericModelLoader::new(registry);

    let result = loader.load_classifier("not_in_registry").await;
    assert!(result.is_err(), "Should fail for model name not in registry");
}
