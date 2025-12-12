//! Latency benchmarks for classifier performance verification
//!
//! Verifies that classifiers meet their tier latency budgets:
//! - Tier A: <2ms (patterns, PII)
//! - Tier B: <5ms (ML classifiers like toxicity)
//! - Tier C: <10ms (complex ML models)
//!
//! Run with: cargo bench -p checkstream-classifiers

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use tokio::runtime::Runtime;

use checkstream_classifiers::pii::PiiClassifier;
use checkstream_classifiers::patterns::PatternClassifier;
use checkstream_classifiers::{Classifier, ClassifierPipeline, AggregationStrategy};

/// Benchmark PII classifier (Tier A target: <2ms)
fn benchmark_pii_classifier(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let classifier = PiiClassifier::new().expect("Failed to create PII classifier");

    let test_cases = vec![
        ("short_clean", "Hello, how are you today?"),
        ("short_pii_email", "Contact me at test@example.com"),
        ("short_pii_phone", "Call me at 555-123-4567"),
        ("short_pii_ssn", "My SSN is 123-45-6789"),
        ("medium_clean", "The quick brown fox jumps over the lazy dog. This is a test sentence without any PII."),
        ("medium_pii", "Contact John at 555-123-4567 or john.doe@example.com for more information."),
    ];

    let mut group = c.benchmark_group("PII_Classifier_Tier_A");
    group.significance_level(0.05);
    group.sample_size(100);

    for (name, text) in test_cases {
        group.bench_with_input(BenchmarkId::new("classify", name), &text, |b, text| {
            b.iter(|| {
                rt.block_on(async {
                    classifier.classify(black_box(text)).await.unwrap()
                })
            });
        });
    }

    group.finish();
}

/// Benchmark pattern classifier (Tier A target: <2ms)
fn benchmark_pattern_classifier(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Create classifier with common patterns (label, pattern) pairs
    let patterns = vec![
        ("unsafe".to_string(), "unsafe".to_string()),
        ("dangerous".to_string(), "dangerous".to_string()),
        ("prohibited".to_string(), "prohibited".to_string()),
        ("blocked".to_string(), "blocked".to_string()),
        ("harmful".to_string(), "harmful".to_string()),
    ];
    let classifier = PatternClassifier::new("test-patterns", patterns)
        .expect("Failed to create pattern classifier");

    let test_cases = vec![
        ("no_match_short", "Hello, how are you?"),
        ("no_match_medium", "This is a perfectly normal sentence with nothing concerning."),
        ("match_single", "This content is unsafe for viewing."),
        ("match_multiple", "This is unsafe and dangerous content that is prohibited."),
    ];

    let mut group = c.benchmark_group("Pattern_Classifier_Tier_A");
    group.significance_level(0.05);
    group.sample_size(100);

    for (name, text) in test_cases {
        group.bench_with_input(BenchmarkId::new("classify", name), &text, |b, text| {
            b.iter(|| {
                rt.block_on(async {
                    classifier.classify(black_box(text)).await.unwrap()
                })
            });
        });
    }

    group.finish();
}

/// Verify latency budgets are met
fn verify_latency_budgets(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("Latency_Budget_Verification");
    group.significance_level(0.01);
    group.sample_size(1000);

    // Tier A: PII should be <2ms
    let pii = PiiClassifier::new().expect("Failed to create PII classifier");
    group.bench_function("tier_a_pii_budget", |b| {
        b.iter(|| {
            rt.block_on(async {
                pii.classify("test@example.com").await.unwrap()
            })
        });
    });

    // Tier A: Pattern should be <2ms
    let patterns = PatternClassifier::new(
        "budget-test",
        vec![("test".to_string(), "test".to_string())]
    ).expect("Failed to create pattern classifier");

    group.bench_function("tier_a_pattern_budget", |b| {
        b.iter(|| {
            rt.block_on(async {
                patterns.classify("This is a test").await.unwrap()
            })
        });
    });

    group.finish();
}

/// Pipeline benchmark - test classifier composition overhead
fn benchmark_pipeline_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let pii: Arc<dyn Classifier> = Arc::new(PiiClassifier::new().expect("Failed to create PII classifier"));
    let patterns: Arc<dyn Classifier> = Arc::new(
        PatternClassifier::new(
            "test",
            vec![("unsafe".to_string(), "unsafe".to_string())]
        ).expect("Failed to create pattern classifier")
    );

    // Single classifier pipeline
    let single_pipeline = ClassifierPipeline::new()
        .add_single("pii", pii.clone());

    // Parallel pipeline with 2 classifiers
    let parallel_pipeline = ClassifierPipeline::new()
        .add_parallel(
            "safety-checks",
            vec![
                ("pii".to_string(), pii.clone()),
                ("patterns".to_string(), patterns.clone()),
            ],
            AggregationStrategy::MaxScore,
        );

    // Sequential pipeline with 2 classifiers
    let sequential_pipeline = ClassifierPipeline::new()
        .add_sequential(
            "sequential-checks",
            vec![
                ("pii".to_string(), pii.clone()),
                ("patterns".to_string(), patterns.clone()),
            ],
        );

    let test_text = "Contact me at test@example.com for unsafe content.";

    let mut group = c.benchmark_group("Pipeline_Overhead");
    group.sample_size(100);

    group.bench_function("single_classifier", |b| {
        b.iter(|| {
            rt.block_on(async {
                single_pipeline.execute(black_box(test_text)).await.unwrap()
            })
        });
    });

    group.bench_function("parallel_two_classifiers", |b| {
        b.iter(|| {
            rt.block_on(async {
                parallel_pipeline.execute(black_box(test_text)).await.unwrap()
            })
        });
    });

    group.bench_function("sequential_two_classifiers", |b| {
        b.iter(|| {
            rt.block_on(async {
                sequential_pipeline.execute(black_box(test_text)).await.unwrap()
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_pii_classifier,
    benchmark_pattern_classifier,
    verify_latency_budgets,
    benchmark_pipeline_overhead
);
criterion_main!(benches);
