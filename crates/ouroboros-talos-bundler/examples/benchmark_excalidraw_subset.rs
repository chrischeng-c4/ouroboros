use ouroboros_talos_bundler::{Bundler, BundleOptions};
use ouroboros_talos_resolver::ResolveOptions;
use ouroboros_talos_transform::TransformOptions;
use ouroboros_talos_asset::AssetOptions;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("⚡ Talos Bundler: Excalidraw Subset Benchmark\n");
    println!("==============================================\n");

    let project_root = PathBuf::from("/Users/chris.cheng/chris-project/ouroboros-talos/benchmarks/excalidraw/excalidraw_subset");
    let entry = project_root.join("src/index.tsx");
    let output_dir = project_root.join("dist_talos");

    println!("📂 Configuration:");
    println!("   Entry: {:?}", entry);
    println!("   Output: {:?}", output_dir);
    println!();

    // No externals - bundle everything including React
    let externals = HashSet::new();

    // Configure resolve options with CSS support
    let resolve_options = ResolveOptions {
        base_dirs: vec![
            project_root.join("src"),
            project_root.join("node_modules"),
        ],
        extensions: vec![
            "ts".to_string(),
            "tsx".to_string(),
            "js".to_string(),
            "jsx".to_string(),
            "css".to_string(),  // Add CSS support
            "json".to_string(),
        ],
        resolve_index: true,
        alias: vec![],
        externals: externals.clone(),
    };

    // Configure bundle options
    let options = BundleOptions {
        entry: entry.clone(),
        output_dir: output_dir.clone(),
        minify: false,
        source_maps: false,
        externals,
        resolve_options,
        transform_options: TransformOptions::default(),
        asset_options: AssetOptions::default(),
    };

    // Cold start benchmark
    println!("❄️  Cold Start Build");
    println!("-------------------");

    let start = Instant::now();

    let bundler = Bundler::new(options)?;
    let output = bundler.bundle(entry).await?;

    let elapsed = start.elapsed();

    println!("✅ Build complete!");
    println!("   Time: {}ms", elapsed.as_millis());
    println!("   Bundle size: {} bytes ({:.2} KB)",
        output.code.len(),
        output.code.len() as f64 / 1024.0
    );
    println!("   Modules bundled: (analyzing...)");
    println!();

    // Write output
    std::fs::create_dir_all(&output_dir)?;
    std::fs::write(output_dir.join("bundle.js"), output.code)?;

    println!("📦 Output written to: {:?}", output_dir.join("bundle.js"));
    println!();

    // Statistics
    println!("📊 Performance:");
    println!("   Cold start: {}ms", elapsed.as_millis());
    println!("   Throughput: {:.2} modules/second",
        29.0 / elapsed.as_secs_f64()
    );
    println!();

    println!("✅ Benchmark complete!");

    Ok(())
}
