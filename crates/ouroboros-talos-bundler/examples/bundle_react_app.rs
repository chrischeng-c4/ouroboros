/// Example: Bundle a React application
use ouroboros_talos_bundler::{Bundler, BundleOptions};
use ouroboros_talos_resolver::ResolveOptions;
use ouroboros_talos_transform::TransformOptions;
use ouroboros_talos_asset::AssetOptions;
use std::path::PathBuf;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Testing Talos Bundler with React App...\n");

    // Create bundle options
    let options = BundleOptions {
        entry: PathBuf::from("/tmp/talos-example/src/index.js"),
        output_dir: PathBuf::from("/tmp/talos-example/dist"),
        source_maps: true,
        minify: false,
        resolve_options: ResolveOptions::default(),
        transform_options: TransformOptions::default(),
        asset_options: AssetOptions::default(),
        externals: HashSet::new(), // Bundle everything including node_modules
    };

    // Create bundler
    let bundler = Bundler::new(options)?;

    // Bundle
    println!("📦 Bundling React application...");
    let entry = PathBuf::from("/tmp/talos-example/src/index.js");

    match bundler.bundle(entry).await {
        Ok(output) => {
            println!("✅ Bundle successful!");
            println!("📊 Bundle size: {} bytes", output.code.len());
            println!("📝 Number of lines: {}", output.code.lines().count());

            // Write to file
            std::fs::create_dir_all("/tmp/talos-example/dist")?;
            std::fs::write("/tmp/talos-example/dist/bundle.js", &output.code)?;

            println!("✅ Written to: /tmp/talos-example/dist/bundle.js");

            // Show first few lines
            println!("\n📄 Bundle preview (first 20 lines):");
            println!("{}", "-".repeat(60));
            for (i, line) in output.code.lines().take(20).enumerate() {
                println!("{:3}: {}", i + 1, line);
            }
            println!("{}", "-".repeat(60));

            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Bundle failed: {}", e);
            eprintln!("Error details: {:?}", e);
            Err(e)
        }
    }
}
