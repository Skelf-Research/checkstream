use checkstream_classifiers::model_config::ModelRegistry;
use checkstream_classifiers::Classifier;
use checkstream_classifiers::ModelLoaderPlugin;
use checkstream_classifiers_ml_plugin::ExternalMlModelLoader;

fn external_ml_tests_enabled() -> bool {
    std::env::var("CHECKSTREAM_RUN_EXTERNAL_ML_TESTS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn distilbert_default_config() -> &'static str {
    r#"
version: "1.0"
models:
  sentiment:
    name: "distilbert-sst2-default"
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

fn local_bert_default_config() -> &'static str {
    r#"
version: "1.0"
models:
  toxicity:
    name: "toxic-bert-default"
    source:
      type: local
      path: "./models/toxic-bert"
    architecture:
      type: bert-sequence-classification
      num_labels: 6
      labels: ["toxic", "severe_toxic", "obscene", "threat", "insult", "identity_hate"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#
}

fn local_roberta_default_config() -> &'static str {
    r#"
version: "1.0"
models:
  roberta-toxicity:
    name: "roberta-toxicity-default"
    source:
      type: local
      path: "./models/toxic-bert"
    architecture:
      type: roberta-sequence-classification
      num_labels: 6
      labels: ["toxic", "severe_toxic", "obscene", "threat", "insult", "identity_hate"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#
}

fn local_missing_deberta_config() -> &'static str {
    r#"
version: "1.0"
models:
  deberta:
    name: "deberta-missing-local"
    source:
      type: local
      path: "./models/does-not-exist"
    architecture:
      type: deberta-sequence-classification
      num_labels: 2
      labels: ["safe", "injection"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#
}

fn local_missing_xlm_roberta_config() -> &'static str {
    r#"
version: "1.0"
models:
  xlm-roberta:
    name: "xlm-roberta-missing-local"
    source:
      type: local
      path: "./models/does-not-exist"
    architecture:
      type: xlm-roberta-sequence-classification
      num_labels: 2
      labels: ["negative", "positive"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#
}

fn local_missing_minilm_config() -> &'static str {
    r#"
version: "1.0"
models:
  mini-lm:
    name: "minilm-missing-local"
    source:
      type: local
      path: "./models/does-not-exist"
    architecture:
      type: mini-lm-sequence-classification
      num_labels: 2
      labels: ["negative", "positive"]
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#
}

fn local_missing_sentence_transformer_config() -> &'static str {
    r#"
version: "1.0"
models:
  sentence-transformer:
    name: "sentence-transformer-missing-local"
    source:
      type: local
      path: "./models/does-not-exist"
    architecture:
      type: sentence-transformer
      pooling: mean
    inference:
      device: "cpu"
      max_length: 512
      threshold: 0.5
"#
}

async fn assert_supported_architecture_reaches_source_validation(yaml: &str, model_name: &str) {
    let registry: ModelRegistry = serde_yaml::from_str(yaml).unwrap();
    let loader = ExternalMlModelLoader::from_registry(registry);
    let err = match loader.load_classifier(model_name).await {
        Ok(_) => panic!("Expected model loading to fail for missing local path"),
        Err(err) => err,
    };
    let message = err.to_string();

    assert!(
        message.contains("Model path does not exist"),
        "Expected source validation error, got: {}",
        message
    );
    assert!(
        !message.contains("Unsupported architecture"),
        "Architecture dispatch failed unexpectedly: {}",
        message
    );
}

async fn load_distilbert_default() -> Option<Box<dyn Classifier>> {
    if !external_ml_tests_enabled() {
        return None;
    }

    let registry: ModelRegistry = serde_yaml::from_str(distilbert_default_config()).unwrap();
    let loader = ExternalMlModelLoader::from_registry(registry);
    Some(
        loader
            .load_classifier("sentiment")
            .await
            .expect("Failed to load default DistilBERT sentiment model"),
    )
}

#[tokio::test]
async fn test_default_distilbert_sentiment_positive() {
    let Some(classifier) = load_distilbert_default().await else {
        return;
    };

    let result = classifier
        .classify("I absolutely love this and it exceeded my expectations.")
        .await
        .unwrap();

    assert_eq!(result.label, "positive");
    assert!(
        result.score > 0.8,
        "Expected high positive confidence, got {}",
        result.score
    );
}

#[tokio::test]
async fn test_default_distilbert_sentiment_negative() {
    let Some(classifier) = load_distilbert_default().await else {
        return;
    };

    let result = classifier
        .classify("This is the worst service I have ever received.")
        .await
        .unwrap();

    assert_eq!(result.label, "negative");
    assert!(
        result.score < 0.2,
        "Expected low positive confidence, got {}",
        result.score
    );
}

#[tokio::test]
async fn test_default_distilbert_metadata_scores() {
    let Some(classifier) = load_distilbert_default().await else {
        return;
    };

    let result = classifier.classify("Neutral test sentence").await.unwrap();
    let all_scores = result
        .metadata
        .all_scores
        .as_ref()
        .expect("Expected all_scores metadata");

    assert_eq!(all_scores.len(), 2, "Expected binary score outputs");
}

#[tokio::test]
async fn test_default_local_bert_toxicity_loads() {
    if !external_ml_tests_enabled() {
        return;
    }

    if !std::path::Path::new("./models/toxic-bert").exists() {
        return;
    }

    let registry: ModelRegistry = serde_yaml::from_str(local_bert_default_config()).unwrap();
    let loader = ExternalMlModelLoader::from_registry(registry);
    let classifier = loader
        .load_classifier("toxicity")
        .await
        .expect("Failed to load local default BERT toxicity model");

    let result = classifier
        .classify("You are awful and disgusting.")
        .await
        .unwrap();

    assert!(
        result.metadata.model.is_some(),
        "Expected model metadata to be populated"
    );
    assert!(
        result
            .metadata
            .all_scores
            .as_ref()
            .is_some_and(|scores: &Vec<(String, f32)>| !scores.is_empty()),
        "Expected non-empty class scores"
    );
}

#[tokio::test]
async fn test_default_local_roberta_toxicity_loads() {
    if !external_ml_tests_enabled() {
        return;
    }

    if !std::path::Path::new("./models/toxic-bert").exists() {
        return;
    }

    let registry: ModelRegistry = serde_yaml::from_str(local_roberta_default_config()).unwrap();
    let loader = ExternalMlModelLoader::from_registry(registry);
    let classifier = loader
        .load_classifier("roberta-toxicity")
        .await
        .expect("Failed to load local default RoBERTa toxicity model");

    let result = classifier
        .classify("You are awful and disgusting.")
        .await
        .unwrap();
    let scores = result
        .metadata
        .all_scores
        .as_ref()
        .expect("Expected class scores for RoBERTa path");

    assert_eq!(scores.len(), 6, "Expected 6 class scores");
}

#[tokio::test]
async fn test_deberta_architecture_dispatch_is_supported() {
    assert_supported_architecture_reaches_source_validation(
        local_missing_deberta_config(),
        "deberta",
    )
    .await;
}

#[tokio::test]
async fn test_xlm_roberta_architecture_dispatch_is_supported() {
    assert_supported_architecture_reaches_source_validation(
        local_missing_xlm_roberta_config(),
        "xlm-roberta",
    )
    .await;
}

#[tokio::test]
async fn test_minilm_architecture_dispatch_is_supported() {
    assert_supported_architecture_reaches_source_validation(
        local_missing_minilm_config(),
        "mini-lm",
    )
    .await;
}

#[tokio::test]
async fn test_sentence_transformer_architecture_dispatch_is_supported() {
    assert_supported_architecture_reaches_source_validation(
        local_missing_sentence_transformer_config(),
        "sentence-transformer",
    )
    .await;
}
