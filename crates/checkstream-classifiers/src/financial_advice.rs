//! Financial Advice Classifier (Tier A/C)
//!
//! Detects when LLM outputs cross from general information into
//! regulated financial advice territory. Designed for FCA Consumer Duty
//! and FINRA compliance.
//!
//! Classification categories:
//! - Information: Factual, educational content (safe)
//! - Guidance: General principles, not personalized (low risk)
//! - PersonalAdvice: Personalized recommendations (regulated)
//! - Suitability: Specific product suitability statements (high risk)
//!
//! Key FCA regulations covered:
//! - COBS 9A: Suitability requirements
//! - COBS 4: Fair, clear, and not misleading
//! - Consumer Duty: Acting in customers' best interests

use crate::classifier::{ClassificationMetadata, ClassificationResult, Classifier, ClassifierTier};
use aho_corasick::AhoCorasick;
use checkstream_core::Result;
use std::time::Instant;

/// Categories of financial content based on regulatory risk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdviceCategory {
    /// Factual, educational content - safe
    Information,
    /// General principles, not personalized - low risk
    Guidance,
    /// Personalized recommendations - regulated (FCA COBS 9A)
    PersonalAdvice,
    /// Specific product suitability statements - high risk
    Suitability,
    /// Risk guarantees or unrealistic claims - prohibited
    ProhibitedClaim,
}

impl AdviceCategory {
    /// Get the risk score for this category (higher = more regulatory risk)
    pub fn risk_score(&self) -> f32 {
        match self {
            Self::Information => 0.1,
            Self::Guidance => 0.3,
            Self::PersonalAdvice => 0.75,
            Self::Suitability => 0.90,
            Self::ProhibitedClaim => 0.98,
        }
    }

    /// Get a human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Information => "information",
            Self::Guidance => "guidance",
            Self::PersonalAdvice => "personal_advice",
            Self::Suitability => "suitability",
            Self::ProhibitedClaim => "prohibited_claim",
        }
    }

    /// Get FCA regulation reference
    pub fn fca_reference(&self) -> Option<&'static str> {
        match self {
            Self::Information => None,
            Self::Guidance => Some("FCA COBS 4"),
            Self::PersonalAdvice => Some("FCA COBS 9A"),
            Self::Suitability => Some("FCA COBS 9A.2"),
            Self::ProhibitedClaim => Some("FCA COBS 4.2 - Misleading"),
        }
    }
}

/// Pattern-based financial advice classifier
pub struct FinancialAdviceClassifier {
    name: String,
    /// Prohibited claims (highest risk)
    prohibited_claims: AhoCorasick,
    prohibited_claims_patterns: Vec<String>,
    /// Suitability statements
    suitability: AhoCorasick,
    suitability_patterns: Vec<String>,
    /// Personal advice indicators
    personal_advice: AhoCorasick,
    personal_advice_patterns: Vec<String>,
    /// Guidance indicators
    guidance: AhoCorasick,
    guidance_patterns: Vec<String>,
    /// Information indicators (safe)
    information: AhoCorasick,
    information_patterns: Vec<String>,
}

impl FinancialAdviceClassifier {
    /// Create a new financial advice classifier with default patterns
    pub fn new() -> Result<Self> {
        Self::with_name("financial-advice")
    }

    /// Create with a custom name
    pub fn with_name(name: impl Into<String>) -> Result<Self> {
        // Prohibited claims - risk guarantees, unrealistic returns
        let prohibited_claims_patterns = vec![
            "guaranteed return".to_string(),
            "guaranteed returns".to_string(),
            "guaranteed profit".to_string(),
            "guaranteed profits".to_string(),
            "guaranteed income".to_string(),
            "risk-free".to_string(),
            "risk free".to_string(),
            "no risk".to_string(),
            "zero risk".to_string(),
            "cannot lose".to_string(),
            "can't lose".to_string(),
            "will definitely".to_string(),
            "certain to increase".to_string(),
            "certain to grow".to_string(),
            "double your money".to_string(),
            "get rich quick".to_string(),
            "easy money".to_string(),
            "100% safe".to_string(),
            "completely safe investment".to_string(),
        ];

        // Suitability statements - personalized product recommendations
        let suitability_patterns = vec![
            "is suitable for you".to_string(),
            "is right for you".to_string(),
            "is perfect for you".to_string(),
            "is ideal for you".to_string(),
            "matches your needs".to_string(),
            "meets your requirements".to_string(),
            "based on your situation".to_string(),
            "based on your circumstances".to_string(),
            "given your risk profile".to_string(),
            "given your financial situation".to_string(),
            "for someone in your position".to_string(),
            "for your specific needs".to_string(),
            "this product suits you".to_string(),
            "this investment suits you".to_string(),
            "recommend this for you".to_string(),
            "you should choose this".to_string(),
            "best option for you".to_string(),
            "perfect fit for your".to_string(),
        ];

        // Personal advice indicators
        let personal_advice_patterns = vec![
            "you should invest".to_string(),
            "you should buy".to_string(),
            "you should sell".to_string(),
            "you should switch".to_string(),
            "you need to invest".to_string(),
            "i recommend".to_string(),
            "i would recommend".to_string(),
            "my recommendation".to_string(),
            "my advice would be".to_string(),
            "my advice is".to_string(),
            "i suggest you".to_string(),
            "i advise you".to_string(),
            "you must invest".to_string(),
            "you must buy".to_string(),
            "open an account with".to_string(),
            "transfer your pension to".to_string(),
            "consolidate your pensions".to_string(),
            "move your isa to".to_string(),
            "switch your provider to".to_string(),
            "put your money in".to_string(),
            "invest in this".to_string(),
        ];

        // Guidance indicators (general principles)
        let guidance_patterns = vec![
            "generally speaking".to_string(),
            "as a general rule".to_string(),
            "typically".to_string(),
            "in general".to_string(),
            "many people find".to_string(),
            "some investors prefer".to_string(),
            "options include".to_string(),
            "you might consider".to_string(),
            "you could consider".to_string(),
            "one option is".to_string(),
            "another option is".to_string(),
            "it may be worth".to_string(),
            "it might be worth".to_string(),
            "you may want to".to_string(),
            "factors to consider".to_string(),
            "things to think about".to_string(),
            "questions to ask yourself".to_string(),
            "speak to a financial adviser".to_string(),
            "consult a financial adviser".to_string(),
            "seek professional advice".to_string(),
        ];

        // Information indicators (educational, factual)
        let information_patterns = vec![
            "an isa is".to_string(),
            "a pension is".to_string(),
            "a sipp is".to_string(),
            "isas are".to_string(),
            "pensions are".to_string(),
            "stocks and shares".to_string(),
            "the difference between".to_string(),
            "how does a".to_string(),
            "what is a".to_string(),
            "defined as".to_string(),
            "this means that".to_string(),
            "for example".to_string(),
            "historically".to_string(),
            "tax rules".to_string(),
            "hmrc allows".to_string(),
            "the annual allowance".to_string(),
            "contribution limits".to_string(),
            "tax relief".to_string(),
            "capital gains tax".to_string(),
            "inheritance tax".to_string(),
        ];

        let prohibited_claims = Self::build_matcher(&prohibited_claims_patterns)?;
        let suitability = Self::build_matcher(&suitability_patterns)?;
        let personal_advice = Self::build_matcher(&personal_advice_patterns)?;
        let guidance = Self::build_matcher(&guidance_patterns)?;
        let information = Self::build_matcher(&information_patterns)?;

        Ok(Self {
            name: name.into(),
            prohibited_claims,
            prohibited_claims_patterns,
            suitability,
            suitability_patterns,
            personal_advice,
            personal_advice_patterns,
            guidance,
            guidance_patterns,
            information,
            information_patterns,
        })
    }

    /// Build an Aho-Corasick matcher from patterns
    fn build_matcher(patterns: &[String]) -> Result<AhoCorasick> {
        AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(patterns)
            .map_err(|e| {
                checkstream_core::Error::classifier(format!(
                    "Failed to build financial advice pattern matcher: {}",
                    e
                ))
            })
    }

    /// Detect the highest-risk category in the text
    fn detect_category(&self, text: &str) -> (AdviceCategory, Vec<(usize, usize, String)>) {
        let mut matches = Vec::new();

        // Check categories from highest risk to lowest
        // Prohibited claims
        for m in self.prohibited_claims.find_iter(text) {
            let pattern = &self.prohibited_claims_patterns[m.pattern().as_usize()];
            matches.push((m.start(), m.end(), pattern.clone()));
        }
        if !matches.is_empty() {
            return (AdviceCategory::ProhibitedClaim, matches);
        }

        // Suitability statements
        for m in self.suitability.find_iter(text) {
            let pattern = &self.suitability_patterns[m.pattern().as_usize()];
            matches.push((m.start(), m.end(), pattern.clone()));
        }
        if !matches.is_empty() {
            return (AdviceCategory::Suitability, matches);
        }

        // Personal advice
        for m in self.personal_advice.find_iter(text) {
            let pattern = &self.personal_advice_patterns[m.pattern().as_usize()];
            matches.push((m.start(), m.end(), pattern.clone()));
        }
        if !matches.is_empty() {
            return (AdviceCategory::PersonalAdvice, matches);
        }

        // Guidance (lower risk but still worth noting)
        for m in self.guidance.find_iter(text) {
            let pattern = &self.guidance_patterns[m.pattern().as_usize()];
            matches.push((m.start(), m.end(), pattern.clone()));
        }
        if !matches.is_empty() {
            return (AdviceCategory::Guidance, matches);
        }

        // Information (safe - but we note it for completeness)
        for m in self.information.find_iter(text) {
            let pattern = &self.information_patterns[m.pattern().as_usize()];
            matches.push((m.start(), m.end(), pattern.clone()));
        }
        if !matches.is_empty() {
            return (AdviceCategory::Information, matches);
        }

        // Default to information if no patterns match
        (AdviceCategory::Information, matches)
    }
}

#[async_trait::async_trait]
impl Classifier for FinancialAdviceClassifier {
    async fn classify(&self, text: &str) -> Result<ClassificationResult> {
        let start = Instant::now();

        let (category, matches) = self.detect_category(text);
        let score = category.risk_score();
        let label = category.label().to_string();

        let mut extra = Vec::new();
        for (_, _, pattern) in &matches {
            extra.push(("matched_pattern".to_string(), pattern.clone()));
        }
        extra.push(("category".to_string(), label.clone()));
        if let Some(fca_ref) = category.fca_reference() {
            extra.push(("fca_reference".to_string(), fca_ref.to_string()));
        }
        let metadata = ClassificationMetadata {
            spans: matches.iter().map(|(s, e, _)| (*s, *e)).collect(),
            extra,
            ..Default::default()
        };

        Ok(ClassificationResult {
            label,
            score,
            metadata,
            latency_us: start.elapsed().as_micros() as u64,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::A // Pattern-based, fast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_information_content() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("An ISA is a tax-efficient savings account. The annual allowance is £20,000.")
            .await
            .unwrap();
        assert_eq!(result.label, "information");
        assert!(result.score < 0.2);
    }

    #[tokio::test]
    async fn test_guidance_content() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify(
                "Generally speaking, you might consider diversifying your portfolio. \
             Options include stocks, bonds, and property.",
            )
            .await
            .unwrap();
        assert_eq!(result.label, "guidance");
        assert!(result.score > 0.2 && result.score < 0.5);
    }

    #[tokio::test]
    async fn test_personal_advice() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify(
                "Based on what you've told me, I recommend you invest in a stocks and shares ISA. \
             You should buy index funds.",
            )
            .await
            .unwrap();
        assert_eq!(result.label, "personal_advice");
        assert!(result.score > 0.7);
    }

    #[tokio::test]
    async fn test_suitability_statement() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify(
                "Based on your situation, this pension is right for you and matches your needs.",
            )
            .await
            .unwrap();
        assert_eq!(result.label, "suitability");
        assert!(result.score > 0.85);
    }

    #[tokio::test]
    async fn test_prohibited_claim() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify(
                "This investment offers guaranteed returns with zero risk. \
             You cannot lose money!",
            )
            .await
            .unwrap();
        assert_eq!(result.label, "prohibited_claim");
        assert!(result.score > 0.95);
    }

    #[tokio::test]
    async fn test_prohibited_claim_case_insensitive() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("GUARANTEED PROFIT with NO RISK")
            .await
            .unwrap();
        assert_eq!(result.label, "prohibited_claim");
    }

    #[tokio::test]
    async fn test_fca_reference_in_metadata() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("You should invest in this fund")
            .await
            .unwrap();
        assert!(result
            .metadata
            .extra
            .iter()
            .any(|(k, v)| k == "fca_reference" && v.contains("COBS")));
    }

    #[tokio::test]
    async fn test_transfer_pension_advice() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("You should transfer your pension to this provider")
            .await
            .unwrap();
        assert_eq!(result.label, "personal_advice");
        assert!(result.score > 0.7);
    }

    #[tokio::test]
    async fn test_seek_advice_is_guidance() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("You may want to speak to a financial adviser before making this decision.")
            .await
            .unwrap();
        assert_eq!(result.label, "guidance");
    }

    #[tokio::test]
    async fn test_tier_is_a() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        assert_eq!(classifier.tier(), ClassifierTier::A);
    }

    #[tokio::test]
    async fn test_latency_within_tier_budget() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("Tax rules can change. The value of investments can go down as well as up.")
            .await
            .unwrap();
        // Tier A budget is 2000us (2ms)
        assert!(
            result.latency_us < 2000,
            "Latency {}us exceeds Tier A budget",
            result.latency_us
        );
    }

    #[tokio::test]
    async fn test_highest_risk_takes_precedence() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        // Contains both guidance and prohibited claim
        let result = classifier
            .classify("Generally speaking, this is a guaranteed return investment")
            .await
            .unwrap();
        // Prohibited claim should take precedence
        assert_eq!(result.label, "prohibited_claim");
    }

    #[tokio::test]
    async fn test_matched_patterns_in_metadata() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("I recommend you should buy this")
            .await
            .unwrap();
        assert!(!result.metadata.spans.is_empty());
        assert!(result
            .metadata
            .extra
            .iter()
            .any(|(k, _)| k == "matched_pattern"));
    }

    #[tokio::test]
    async fn test_clean_non_financial_text() {
        let classifier = FinancialAdviceClassifier::new().unwrap();
        let result = classifier
            .classify("The weather today is sunny with a high of 22 degrees.")
            .await
            .unwrap();
        // Should default to information (safe) for non-financial content
        assert_eq!(result.label, "information");
        assert!(result.score < 0.2);
    }
}
