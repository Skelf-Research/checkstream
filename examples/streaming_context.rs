//! Example: Streaming Classification with Context Windows
//!
//! This example demonstrates how to use streaming classifiers with different
//! context window configurations to see previous chunks.

use checkstream_classifiers::{
    ClassificationResult, Classifier, ClassifierTier, StreamingClassifier, StreamingConfig,
};
use std::sync::Arc;

/// Mock classifier that reports what text it saw
struct ContextAwareClassifier {
    name: String,
}

#[async_trait::async_trait]
impl Classifier for ContextAwareClassifier {
    async fn classify(&self, text: &str) -> checkstream_core::Result<ClassificationResult> {
        // Calculate score based on text length (just for demo)
        let score = (text.len() as f32 / 100.0).min(1.0);

        println!(
            "  {} saw: \"{}\" (length: {}, score: {:.2})",
            self.name,
            text,
            text.len(),
            score
        );

        Ok(ClassificationResult {
            label: if score > 0.5 {
                "long_context"
            } else {
                "short_context"
            }
            .to_string(),
            score,
            metadata: Default::default(),
            latency_us: 1000,
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tier(&self) -> ClassifierTier {
        ClassifierTier::B
    }
}

#[tokio::main]
async fn main() -> checkstream_core::Result<()> {
    println!("🔄 Streaming Classification with Context Windows\n");
    println!("═══════════════════════════════════════════════════════\n");

    let chunks = [
        "Hello,",
        "I'm interested",
        "in investing",
        "my savings.",
        "What do",
        "you recommend?",
    ];

    println!("Simulating stream with {} chunks:", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!("  Chunk {}: \"{}\"", i + 1, chunk);
    }
    println!();

    // Example 1: No context (only current chunk)
    println!("─────────────────────────────────────────────────────────");
    println!("Example 1: NO CONTEXT (only current chunk)");
    println!("─────────────────────────────────────────────────────────\n");

    let classifier = Arc::new(ContextAwareClassifier {
        name: "NoContext".to_string(),
    });

    let mut streaming = StreamingClassifier::new(classifier, StreamingConfig::no_context());

    for (i, chunk) in chunks.iter().enumerate() {
        println!("Processing chunk {}: \"{}\"", i + 1, chunk);
        let result = streaming.classify_chunk(chunk.to_string()).await?;
        println!(
            "  → Result: {} (score: {:.2})\n",
            result.label, result.score
        );
    }

    // Example 2: Last 3 chunks
    println!("─────────────────────────────────────────────────────────");
    println!("Example 2: LAST 3 CHUNKS");
    println!("─────────────────────────────────────────────────────────\n");

    let classifier = Arc::new(ContextAwareClassifier {
        name: "Last3".to_string(),
    });

    let mut streaming = StreamingClassifier::new(classifier, StreamingConfig::with_window(3));

    for (i, chunk) in chunks.iter().enumerate() {
        println!("Processing chunk {}: \"{}\"", i + 1, chunk);
        let result = streaming.classify_chunk(chunk.to_string()).await?;
        println!(
            "  → Result: {} (score: {:.2})\n",
            result.label, result.score
        );
    }

    // Example 3: Entire buffer
    println!("─────────────────────────────────────────────────────────");
    println!("Example 3: ENTIRE BUFFER (all previous chunks)");
    println!("─────────────────────────────────────────────────────────\n");

    let classifier = Arc::new(ContextAwareClassifier {
        name: "EntireBuffer".to_string(),
    });

    let mut streaming = StreamingClassifier::new(classifier, StreamingConfig::entire_buffer());

    for (i, chunk) in chunks.iter().enumerate() {
        println!("Processing chunk {}: \"{}\"", i + 1, chunk);
        let result = streaming.classify_chunk(chunk.to_string()).await?;
        println!(
            "  → Result: {} (score: {:.2})\n",
            result.label, result.score
        );
    }

    // Example 4: Comparison showing why context matters
    println!("═══════════════════════════════════════════════════════");
    println!("Example 4: WHY CONTEXT MATTERS");
    println!("═══════════════════════════════════════════════════════\n");

    let problematic_chunks = [
        "That sounds",
        "like a",
        "great investment!",
        "I recommend",
        "putting all",
        "your money",
        "into it!",
    ];

    println!("Stream with context-dependent risk:");
    for (i, chunk) in problematic_chunks.iter().enumerate() {
        println!("  Chunk {}: \"{}\"", i + 1, chunk);
    }
    println!();

    // Without context
    println!("WITHOUT CONTEXT (each chunk alone):");
    let classifier = Arc::new(ContextAwareClassifier {
        name: "NoContext".to_string(),
    });
    let mut no_context = StreamingClassifier::new(classifier, StreamingConfig::no_context());

    for (i, chunk) in problematic_chunks.iter().enumerate() {
        let result = no_context.classify_chunk(chunk.to_string()).await?;
        println!(
            "  Chunk {}: \"{}\" → {}",
            i + 1,
            chunk,
            if result.score > 0.3 {
                "⚠️ FLAG"
            } else {
                "✓ OK"
            }
        );
    }
    println!();

    // With context
    println!("WITH CONTEXT (sees previous chunks):");
    let classifier = Arc::new(ContextAwareClassifier {
        name: "WithContext".to_string(),
    });
    let mut with_context = StreamingClassifier::new(classifier, StreamingConfig::entire_buffer());

    for (i, chunk) in problematic_chunks.iter().enumerate() {
        let result = with_context.classify_chunk(chunk.to_string()).await?;
        println!(
            "  Chunk {}: \"{}\" → {}",
            i + 1,
            chunk,
            if result.score > 0.5 {
                "⚠️ FLAG"
            } else {
                "✓ OK"
            }
        );
    }
    println!();

    // Example 5: Use cases
    println!("═══════════════════════════════════════════════════════");
    println!("Example 5: CONFIGURATION USE CASES");
    println!("═══════════════════════════════════════════════════════\n");

    println!("1️⃣  NO CONTEXT (context_chunks = 1)");
    println!("   Use for: Fast, independent chunk checks");
    println!("   Examples:");
    println!("     • PII detection (SSN, credit card in current chunk)");
    println!("     • Profanity filter (single words)");
    println!("     • Token-level toxicity");
    println!("   Latency: Lowest (only current chunk)\n");

    println!("2️⃣  SLIDING WINDOW (context_chunks = 3-5)");
    println!("   Use for: Local context-aware detection");
    println!("   Examples:");
    println!("     • Sentence-level toxicity");
    println!("     • Short-term conversation flow");
    println!("     • Recent topic detection");
    println!("   Latency: Low (fixed window size)\n");

    println!("3️⃣  ENTIRE BUFFER (context_chunks = 0)");
    println!("   Use for: Full conversation context");
    println!("   Examples:");
    println!("     • Advice vs. information (needs full context)");
    println!("     • Jailbreak attempts (multi-turn attacks)");
    println!("     • Compliance (full conversation history)");
    println!("   Latency: Higher (grows with conversation)\n");

    // Example 6: Phase-specific configurations
    println!("═══════════════════════════════════════════════════════");
    println!("Example 6: PHASE-SPECIFIC CONFIGURATIONS");
    println!("═══════════════════════════════════════════════════════\n");

    println!("Phase 1 (Ingress):");
    println!("  Config: N/A (checks entire prompt at once)");
    println!("  No streaming yet\n");

    println!("Phase 2 (Midstream - Fast Path):");
    println!("  Config: NO CONTEXT or SMALL WINDOW (1-3 chunks)");
    println!("  Reason: Must be ultra-fast (<5ms)");
    println!("  Trade-off: Speed over context\n");

    println!("Phase 2 (Midstream - Thorough Path):");
    println!("  Config: LARGER WINDOW (5-10 chunks)");
    println!("  Reason: More context for better detection");
    println!("  Trade-off: Slightly higher latency but better accuracy\n");

    println!("Phase 3 (Egress):");
    println!("  Config: ENTIRE BUFFER");
    println!("  Reason: Full conversation analysis");
    println!("  Can take time (async, not blocking user)\n");

    println!("\n✨ Example complete!");
    println!("\n💡 Key Takeaway:");
    println!("   The right context window depends on:");
    println!("   • What you're detecting (token vs. conversation)");
    println!("   • Your latency budget");
    println!("   • The phase of processing");

    Ok(())
}
