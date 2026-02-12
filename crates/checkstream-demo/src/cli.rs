use crate::models::DemoMode;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "checkstream-demo")]
#[command(
    author,
    version,
    about = "Interactive CheckStream demo and visualization"
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the demo server with web UI
    Start {
        /// Listen port
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Listen address
        #[arg(short, long, default_value = "127.0.0.1")]
        address: String,

        /// Mode: mock or proxy
        #[arg(short, long, default_value = "mock", value_parser = parse_mode)]
        mode: DemoMode,

        /// Backend URL (for proxy mode)
        #[arg(short, long)]
        backend: Option<String>,

        /// Policy file path
        #[arg(long, default_value = "./policies/default.yaml")]
        policy: String,

        /// Classifiers config path
        #[arg(long, default_value = "./classifiers.yaml")]
        classifiers: String,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },

    /// Generate traffic without starting the web UI
    GenerateTraffic {
        /// Target URL (demo server or real proxy)
        #[arg(short, long, default_value = "http://127.0.0.1:3000")]
        target: String,

        /// Requests per second
        #[arg(short, long, default_value = "10")]
        rate: u32,

        /// Duration in seconds (0 = infinite)
        #[arg(short, long, default_value = "60")]
        duration: u64,

        /// Issue types to inject (comma-separated: pii,toxicity,injection,financial)
        #[arg(long, default_value = "pii,toxicity")]
        issues: String,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
}

fn parse_mode(s: &str) -> Result<DemoMode, String> {
    s.parse()
}
