//! Classifier pipeline system for chaining and parallel execution
//!
//! This module provides a flexible pipeline system that allows:
//! - Parallel execution of multiple classifiers
//! - Sequential chaining of classifiers
//! - Conditional execution based on results
//! - Result aggregation and combination

use crate::{Classifier, ClassificationResult};
use checkstream_core::Result;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Instant;

/// A pipeline of classifiers that can be executed in parallel or sequentially
#[derive(Clone)]
pub struct ClassifierPipeline {
    stages: Vec<PipelineStage>,
}

/// A single stage in the pipeline
#[derive(Clone)]
pub enum PipelineStage {
    /// Execute a single classifier
    Single {
        name: String,
        classifier: Arc<dyn Classifier>,
    },

    /// Execute multiple classifiers in parallel
    Parallel {
        name: String,
        classifiers: Vec<(String, Arc<dyn Classifier>)>,
        aggregation: AggregationStrategy,
    },

    /// Execute classifiers sequentially (chain)
    Sequential {
        name: String,
        classifiers: Vec<(String, Arc<dyn Classifier>)>,
    },

    /// Conditional execution based on previous results
    Conditional {
        name: String,
        condition: Arc<dyn Fn(&[PipelineResult]) -> bool + Send + Sync>,
        classifier: Arc<dyn Classifier>,
    },
}

/// Strategy for aggregating results from parallel classifiers
#[derive(Debug, Clone, Copy)]
pub enum AggregationStrategy {
    /// Return all results
    All,

    /// Return the result with highest score
    MaxScore,

    /// Return the result with lowest score
    MinScore,

    /// Return first positive result (score > threshold)
    FirstPositive(f32),

    /// Require all classifiers to agree (all positive or all negative)
    Unanimous,

    /// Use weighted average of scores
    WeightedAverage,
}

/// Result from a pipeline stage
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Stage name
    pub stage_name: String,

    /// Classifier name
    pub classifier_name: String,

    /// Classification result
    pub result: ClassificationResult,

    /// Stage execution time
    pub stage_latency_us: u64,
}

/// Complete pipeline execution result
#[derive(Debug, Clone)]
pub struct PipelineExecutionResult {
    /// All stage results
    pub results: Vec<PipelineResult>,

    /// Total pipeline execution time
    pub total_latency_us: u64,

    /// Final aggregated decision
    pub final_decision: Option<ClassificationResult>,
}

impl ClassifierPipeline {
    /// Create a new empty pipeline
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    /// Add a single classifier stage
    pub fn add_single(mut self, name: impl Into<String>, classifier: Arc<dyn Classifier>) -> Self {
        self.stages.push(PipelineStage::Single {
            name: name.into(),
            classifier,
        });
        self
    }

    /// Add a parallel execution stage
    pub fn add_parallel(
        mut self,
        name: impl Into<String>,
        classifiers: Vec<(String, Arc<dyn Classifier>)>,
        aggregation: AggregationStrategy,
    ) -> Self {
        self.stages.push(PipelineStage::Parallel {
            name: name.into(),
            classifiers,
            aggregation,
        });
        self
    }

    /// Add a sequential chain stage
    pub fn add_sequential(
        mut self,
        name: impl Into<String>,
        classifiers: Vec<(String, Arc<dyn Classifier>)>,
    ) -> Self {
        self.stages.push(PipelineStage::Sequential {
            name: name.into(),
            classifiers,
        });
        self
    }

    /// Add a conditional stage
    pub fn add_conditional<F>(
        mut self,
        name: impl Into<String>,
        condition: F,
        classifier: Arc<dyn Classifier>,
    ) -> Self
    where
        F: Fn(&[PipelineResult]) -> bool + Send + Sync + 'static,
    {
        self.stages.push(PipelineStage::Conditional {
            name: name.into(),
            condition: Arc::new(condition),
            classifier,
        });
        self
    }

    /// Execute the entire pipeline
    pub async fn execute(&self, text: &str) -> Result<PipelineExecutionResult> {
        let start = Instant::now();
        let mut all_results = Vec::new();

        for stage in &self.stages {
            let stage_results = self.execute_stage(stage, text, &all_results).await?;
            all_results.extend(stage_results);
        }

        let final_decision = self.compute_final_decision(&all_results);

        Ok(PipelineExecutionResult {
            results: all_results,
            total_latency_us: start.elapsed().as_micros() as u64,
            final_decision,
        })
    }

    /// Execute a single stage
    async fn execute_stage(
        &self,
        stage: &PipelineStage,
        text: &str,
        previous_results: &[PipelineResult],
    ) -> Result<Vec<PipelineResult>> {
        match stage {
            PipelineStage::Single { name, classifier } => {
                self.execute_single(name, classifier, text).await
            }

            PipelineStage::Parallel {
                name,
                classifiers,
                aggregation,
            } => self.execute_parallel(name, classifiers, text, *aggregation).await,

            PipelineStage::Sequential { name, classifiers } => {
                self.execute_sequential(name, classifiers, text).await
            }

            PipelineStage::Conditional {
                name,
                condition,
                classifier,
            } => {
                if condition(previous_results) {
                    self.execute_single(name, classifier, text).await
                } else {
                    Ok(Vec::new()) // Skip this stage
                }
            }
        }
    }

    /// Execute a single classifier
    async fn execute_single(
        &self,
        stage_name: &str,
        classifier: &Arc<dyn Classifier>,
        text: &str,
    ) -> Result<Vec<PipelineResult>> {
        let stage_start = Instant::now();
        let result = classifier.classify(text).await?;

        Ok(vec![PipelineResult {
            stage_name: stage_name.to_string(),
            classifier_name: classifier.name().to_string(),
            result,
            stage_latency_us: stage_start.elapsed().as_micros() as u64,
        }])
    }

    /// Execute multiple classifiers in parallel
    async fn execute_parallel(
        &self,
        stage_name: &str,
        classifiers: &[(String, Arc<dyn Classifier>)],
        text: &str,
        aggregation: AggregationStrategy,
    ) -> Result<Vec<PipelineResult>> {
        let stage_start = Instant::now();

        // Execute all classifiers concurrently
        let futures: Vec<_> = classifiers
            .iter()
            .map(|(name, classifier)| {
                let text = text.to_string();
                let name = name.clone();
                let classifier = Arc::clone(classifier);

                async move {
                    let result = classifier.classify(&text).await?;
                    Ok::<_, checkstream_core::Error>((name, result))
                }
            })
            .collect();

        let results = join_all(futures).await;

        let stage_latency = stage_start.elapsed().as_micros() as u64;

        // Convert to PipelineResults
        let mut pipeline_results = Vec::new();
        for result in results {
            let (classifier_name, classification_result) = result?;
            pipeline_results.push(PipelineResult {
                stage_name: stage_name.to_string(),
                classifier_name,
                result: classification_result,
                stage_latency_us: stage_latency,
            });
        }

        // Apply aggregation strategy
        self.apply_aggregation(&mut pipeline_results, aggregation);

        Ok(pipeline_results)
    }

    /// Execute classifiers sequentially
    async fn execute_sequential(
        &self,
        stage_name: &str,
        classifiers: &[(String, Arc<dyn Classifier>)],
        text: &str,
    ) -> Result<Vec<PipelineResult>> {
        let stage_start = Instant::now();
        let mut results = Vec::new();

        for (name, classifier) in classifiers {
            let result = classifier.classify(text).await?;
            results.push(PipelineResult {
                stage_name: stage_name.to_string(),
                classifier_name: name.clone(),
                result,
                stage_latency_us: stage_start.elapsed().as_micros() as u64,
            });
        }

        Ok(results)
    }

    /// Apply aggregation strategy to results
    fn apply_aggregation(&self, results: &mut Vec<PipelineResult>, strategy: AggregationStrategy) {
        match strategy {
            AggregationStrategy::All => {
                // Keep all results
            }

            AggregationStrategy::MaxScore => {
                if let Some(max_result) = results.iter().max_by(|a, b| {
                    a.result.score.partial_cmp(&b.result.score).unwrap()
                }) {
                    let max_result = max_result.clone();
                    results.clear();
                    results.push(max_result);
                }
            }

            AggregationStrategy::MinScore => {
                if let Some(min_result) = results.iter().min_by(|a, b| {
                    a.result.score.partial_cmp(&b.result.score).unwrap()
                }) {
                    let min_result = min_result.clone();
                    results.clear();
                    results.push(min_result);
                }
            }

            AggregationStrategy::FirstPositive(threshold) => {
                if let Some(positive) = results.iter().find(|r| r.result.score >= threshold) {
                    let positive = positive.clone();
                    results.clear();
                    results.push(positive);
                }
            }

            AggregationStrategy::Unanimous => {
                // Check if all results agree (all positive or all negative)
                let threshold = 0.5;
                let all_positive = results.iter().all(|r| r.result.score >= threshold);
                let all_negative = results.iter().all(|r| r.result.score < threshold);

                if !all_positive && !all_negative {
                    // Not unanimous, keep all for inspection
                    // Could also return empty or error
                }
            }

            AggregationStrategy::WeightedAverage => {
                if !results.is_empty() {
                    let avg_score: f32 = results.iter().map(|r| r.result.score).sum::<f32>()
                        / results.len() as f32;

                    // Create an aggregated result
                    let mut aggregated = results[0].clone();
                    aggregated.result.score = avg_score;
                    aggregated.result.label = if avg_score >= 0.5 {
                        "positive"
                    } else {
                        "negative"
                    }
                    .to_string();

                    results.clear();
                    results.push(aggregated);
                }
            }
        }
    }

    /// Compute final decision from all results
    fn compute_final_decision(&self, results: &[PipelineResult]) -> Option<ClassificationResult> {
        if results.is_empty() {
            return None;
        }

        // Use the last result as the final decision
        // Could be made more sophisticated with custom logic
        results.last().map(|r| r.result.clone())
    }

    /// Get number of stages in pipeline
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

impl Default for ClassifierPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing pipelines fluently
pub struct PipelineBuilder {
    pipeline: ClassifierPipeline,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self {
            pipeline: ClassifierPipeline::new(),
        }
    }

    /// Add a single classifier
    pub fn single(mut self, name: impl Into<String>, classifier: Arc<dyn Classifier>) -> Self {
        self.pipeline = self.pipeline.add_single(name, classifier);
        self
    }

    /// Add parallel classifiers
    pub fn parallel(
        mut self,
        name: impl Into<String>,
        classifiers: Vec<(String, Arc<dyn Classifier>)>,
        aggregation: AggregationStrategy,
    ) -> Self {
        self.pipeline = self.pipeline.add_parallel(name, classifiers, aggregation);
        self
    }

    /// Add sequential classifiers
    pub fn sequential(
        mut self,
        name: impl Into<String>,
        classifiers: Vec<(String, Arc<dyn Classifier>)>,
    ) -> Self {
        self.pipeline = self.pipeline.add_sequential(name, classifiers);
        self
    }

    /// Add conditional classifier
    pub fn conditional<F>(
        mut self,
        name: impl Into<String>,
        condition: F,
        classifier: Arc<dyn Classifier>,
    ) -> Self
    where
        F: Fn(&[PipelineResult]) -> bool + Send + Sync + 'static,
    {
        self.pipeline = self.pipeline.add_conditional(name, condition, classifier);
        self
    }

    /// Build the pipeline
    pub fn build(self) -> ClassifierPipeline {
        self.pipeline
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classifier::{ClassificationMetadata, ClassifierTier};

    // Mock classifier for testing
    struct MockClassifier {
        name: String,
        score: f32,
    }

    #[async_trait::async_trait]
    impl Classifier for MockClassifier {
        async fn classify(&self, _text: &str) -> Result<ClassificationResult> {
            Ok(ClassificationResult {
                label: if self.score >= 0.5 { "positive" } else { "negative" }.to_string(),
                score: self.score,
                metadata: ClassificationMetadata::default(),
                latency_us: 1000,
            })
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn tier(&self) -> ClassifierTier {
            ClassifierTier::A
        }
    }

    #[tokio::test]
    async fn test_single_stage() {
        let classifier = Arc::new(MockClassifier {
            name: "test".to_string(),
            score: 0.8,
        });

        let pipeline = ClassifierPipeline::new().add_single("stage1", classifier);

        let result = pipeline.execute("test text").await.unwrap();

        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].result.score, 0.8);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let classifiers = vec![
            (
                "classifier1".to_string(),
                Arc::new(MockClassifier {
                    name: "c1".to_string(),
                    score: 0.6,
                }) as Arc<dyn Classifier>,
            ),
            (
                "classifier2".to_string(),
                Arc::new(MockClassifier {
                    name: "c2".to_string(),
                    score: 0.9,
                }) as Arc<dyn Classifier>,
            ),
        ];

        let pipeline = ClassifierPipeline::new().add_parallel(
            "parallel_stage",
            classifiers,
            AggregationStrategy::All,
        );

        let result = pipeline.execute("test text").await.unwrap();

        assert_eq!(result.results.len(), 2);
    }

    #[tokio::test]
    async fn test_max_score_aggregation() {
        let classifiers = vec![
            (
                "classifier1".to_string(),
                Arc::new(MockClassifier {
                    name: "c1".to_string(),
                    score: 0.6,
                }) as Arc<dyn Classifier>,
            ),
            (
                "classifier2".to_string(),
                Arc::new(MockClassifier {
                    name: "c2".to_string(),
                    score: 0.9,
                }) as Arc<dyn Classifier>,
            ),
        ];

        let pipeline = ClassifierPipeline::new().add_parallel(
            "parallel_stage",
            classifiers,
            AggregationStrategy::MaxScore,
        );

        let result = pipeline.execute("test text").await.unwrap();

        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].result.score, 0.9);
    }

    #[tokio::test]
    async fn test_conditional_execution() {
        let initial_classifier = Arc::new(MockClassifier {
            name: "initial".to_string(),
            score: 0.8,
        });

        let conditional_classifier = Arc::new(MockClassifier {
            name: "conditional".to_string(),
            score: 0.5,
        });

        let pipeline = ClassifierPipeline::new()
            .add_single("stage1", initial_classifier)
            .add_conditional(
                "conditional_stage",
                |results: &[PipelineResult]| {
                    results.iter().any(|r| r.result.score > 0.7)
                },
                conditional_classifier,
            );

        let result = pipeline.execute("test text").await.unwrap();

        // Should execute both stages since condition is met
        assert_eq!(result.results.len(), 2);
    }

    #[tokio::test]
    async fn test_pipeline_builder() {
        let c1 = Arc::new(MockClassifier {
            name: "c1".to_string(),
            score: 0.6,
        });

        let pipeline = PipelineBuilder::new()
            .single("stage1", c1)
            .build();

        let result = pipeline.execute("test").await.unwrap();
        assert_eq!(result.results.len(), 1);
    }
}
