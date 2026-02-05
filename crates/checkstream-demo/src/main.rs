use checkstream_demo::cli::{Cli, Commands};
use checkstream_demo::models::{DemoConfig, IssueConfig};
use checkstream_demo::server::run_server;
use clap::Parser;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            port,
            address,
            mode,
            backend,
            policy,
            classifiers,
            verbose,
        } => {
            // Initialize logging
            init_logging(verbose);

            let config = DemoConfig {
                mode,
                backend_url: backend,
                policy_path: policy,
                classifiers_path: classifiers,
                issue_config: IssueConfig::default(),
                ..Default::default()
            };

            let addr: SocketAddr = format!("{}:{}", address, port).parse()?;

            println!();
            println!("  ╔═══════════════════════════════════════════════════════════╗");
            println!("  ║                                                           ║");
            println!("  ║   ██████╗██╗  ██╗███████╗ ██████╗██╗  ██╗                 ║");
            println!("  ║  ██╔════╝██║  ██║██╔════╝██╔════╝██║ ██╔╝                 ║");
            println!("  ║  ██║     ███████║█████╗  ██║     █████╔╝                  ║");
            println!("  ║  ██║     ██╔══██║██╔══╝  ██║     ██╔═██╗                  ║");
            println!("  ║  ╚██████╗██║  ██║███████╗╚██████╗██║  ██╗                 ║");
            println!("  ║   ╚═════╝╚═╝  ╚═╝╚══════╝ ╚═════╝╚═╝  ╚═╝                 ║");
            println!("  ║                                                           ║");
            println!("  ║   ███████╗████████╗██████╗ ███████╗ █████╗ ███╗   ███╗   ║");
            println!("  ║   ██╔════╝╚══██╔══╝██╔══██╗██╔════╝██╔══██╗████╗ ████║   ║");
            println!("  ║   ███████╗   ██║   ██████╔╝█████╗  ███████║██╔████╔██║   ║");
            println!("  ║   ╚════██║   ██║   ██╔══██╗██╔══╝  ██╔══██║██║╚██╔╝██║   ║");
            println!("  ║   ███████║   ██║   ██║  ██║███████╗██║  ██║██║ ╚═╝ ██║   ║");
            println!("  ║   ╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝   ║");
            println!("  ║                                                           ║");
            println!("  ║               Interactive Demo & Visualizer               ║");
            println!("  ╚═══════════════════════════════════════════════════════════╝");
            println!();
            println!("  Mode:    {:?}", mode);
            println!("  Policy:  {}", config.policy_path);
            println!();
            println!("  Open http://{} in your browser", addr);
            println!();

            run_server(config, addr).await?;
        }

        Commands::GenerateTraffic {
            target,
            rate,
            duration,
            issues,
            verbose,
        } => {
            init_logging(verbose);

            println!("Generating traffic to {}", target);
            println!("  Rate: {} req/s", rate);
            println!("  Duration: {}s", if duration == 0 { "infinite".to_string() } else { duration.to_string() });
            println!("  Issues: {}", issues);
            println!();

            // Parse issue types
            let issue_types: Vec<&str> = issues.split(',').map(|s| s.trim()).collect();

            let issue_config = IssueConfig {
                pii_enabled: issue_types.contains(&"pii"),
                toxicity_enabled: issue_types.contains(&"toxicity"),
                injection_enabled: issue_types.contains(&"injection"),
                financial_advice_enabled: issue_types.contains(&"financial"),
                ..Default::default()
            };

            // Start traffic generation via HTTP
            let client = reqwest::Client::new();

            let start_response = client
                .post(format!("{}/api/traffic/start", target))
                .json(&serde_json::json!({
                    "rate": rate,
                    "duration_secs": if duration == 0 { None } else { Some(duration) },
                    "issue_config": issue_config
                }))
                .send()
                .await?;

            if start_response.status().is_success() {
                println!("Traffic generation started!");

                if duration > 0 {
                    println!("Waiting for {} seconds...", duration);
                    tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;

                    // Stop traffic
                    client
                        .post(format!("{}/api/traffic/stop", target))
                        .send()
                        .await?;

                    println!("Traffic generation completed.");
                } else {
                    println!("Press Ctrl+C to stop.");
                    tokio::signal::ctrl_c().await?;
                    client
                        .post(format!("{}/api/traffic/stop", target))
                        .send()
                        .await?;
                }
            } else {
                eprintln!(
                    "Failed to start traffic: {}",
                    start_response.text().await?
                );
            }
        }
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    let filter = if verbose {
        "checkstream_demo=debug,tower_http=debug"
    } else {
        "checkstream_demo=info,tower_http=warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
