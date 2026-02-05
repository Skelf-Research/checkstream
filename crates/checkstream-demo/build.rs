use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Get the profile (debug/release)
    let profile = env::var("PROFILE").unwrap_or_default();

    // Only build frontend in release mode or if BUILD_WEB is set
    let build_web = env::var("BUILD_WEB").is_ok() || profile == "release";

    // Get the manifest directory (where Cargo.toml is)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let web_dir = Path::new(&manifest_dir).join("web");

    if build_web && web_dir.exists() {
        println!("cargo:warning=Building frontend...");

        // Check if node_modules exists, if not run npm install
        let node_modules = web_dir.join("node_modules");
        if !node_modules.exists() {
            println!("cargo:warning=Running npm install...");
            let status = Command::new("npm")
                .args(["install"])
                .current_dir(&web_dir)
                .status();

            match status {
                Ok(s) if s.success() => println!("cargo:warning=npm install succeeded"),
                Ok(s) => {
                    println!("cargo:warning=npm install failed with status: {}", s);
                    // Don't fail the build, just warn
                }
                Err(e) => {
                    println!("cargo:warning=Failed to run npm install: {}", e);
                    println!("cargo:warning=Make sure Node.js and npm are installed");
                }
            }
        }

        // Run npm build
        println!("cargo:warning=Running npm run build...");
        let status = Command::new("npm")
            .args(["run", "build"])
            .current_dir(&web_dir)
            .status();

        match status {
            Ok(s) if s.success() => println!("cargo:warning=Frontend build succeeded"),
            Ok(s) => {
                println!("cargo:warning=Frontend build failed with status: {}", s);
            }
            Err(e) => {
                println!("cargo:warning=Failed to run npm build: {}", e);
                println!("cargo:warning=The demo will use fallback HTML");
            }
        }
    } else if !web_dir.exists() {
        println!("cargo:warning=Web directory not found at {:?}", web_dir);
    } else {
        println!("cargo:warning=Skipping frontend build (debug mode)");
        println!("cargo:warning=Set BUILD_WEB=1 or build with --release to include frontend");
    }

    // Rerun if web sources change
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/vite.config.ts");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-env-changed=BUILD_WEB");
}
