/// Performance benchmark for Talos bundler
use ouroboros_talos_bundler::{Bundler, BundleOptions};
use ouroboros_talos_resolver::ResolveOptions;
use ouroboros_talos_transform::TransformOptions;
use ouroboros_talos_asset::AssetOptions;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("⚡ Talos Bundler Performance Benchmark\n");

    let options = BundleOptions {
        entry: PathBuf::from("/tmp/talos-example/src/index.js"),
        output_dir: PathBuf::from("/tmp/talos-example/dist"),
        source_maps: true,
        minify: false,
        resolve_options: ResolveOptions::default(),
        transform_options: TransformOptions::default(),
        asset_options: AssetOptions::default(),
        externals: HashSet::new(),
    };

    // Benchmark 1: Cold start (first bundle)
    println!("📊 Benchmark 1: Cold Start");
    println!("{}", "=".repeat(60));

    let start = Instant::now();
    let bundler = Bundler::new(options.clone())?;
    let init_time = start.elapsed();
    println!("⏱  Bundler initialization: {:?}", init_time);

    let start = Instant::now();
    let entry = PathBuf::from("/tmp/talos-example/src/index.js");
    let output = bundler.bundle(entry.clone()).await?;
    let cold_bundle_time = start.elapsed();

    println!("⏱  Cold bundle time: {:?}", cold_bundle_time);
    println!("📦 Bundle size: {} bytes", output.code.len());
    println!("📊 Modules processed: 3");
    println!();

    // Benchmark 2: Hot rebuild (with cache)
    println!("📊 Benchmark 2: Hot Rebuild (Cached)");
    println!("{}", "=".repeat(60));

    let start = Instant::now();
    let output2 = bundler.bundle(entry.clone()).await?;
    let hot_bundle_time = start.elapsed();

    println!("⏱  Hot bundle time: {:?}", hot_bundle_time);
    println!("🚀 Speedup: {:.2}x", cold_bundle_time.as_secs_f64() / hot_bundle_time.as_secs_f64());
    println!();

    // Benchmark 3: Multiple runs to get average
    println!("📊 Benchmark 3: Average Performance (10 runs)");
    println!("{}", "=".repeat(60));

    let mut times = Vec::new();
    for i in 1..=10 {
        let start = Instant::now();
        bundler.bundle(entry.clone()).await?;
        let elapsed = start.elapsed();
        times.push(elapsed);
        print!("  Run {}: {:?}\r", i, elapsed);
    }
    println!();

    let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    let min_time = times.iter().min().unwrap();
    let max_time = times.iter().max().unwrap();

    println!("⏱  Average: {:?}", avg_time);
    println!("⏱  Min: {:?}", min_time);
    println!("⏱  Max: {:?}", max_time);
    println!();

    // Performance summary
    println!("📈 Performance Summary");
    println!("{}", "=".repeat(60));
    println!("Target (3 modules):");
    println!("  • Cold build: < 500ms");
    println!("  • Hot rebuild: < 100ms");
    println!();
    println!("Actual:");
    println!("  • Cold build: {:?} {}",
        cold_bundle_time,
        if cold_bundle_time.as_millis() < 500 { "✅" } else { "❌" }
    );
    println!("  • Hot rebuild: {:?} {}",
        hot_bundle_time,
        if hot_bundle_time.as_millis() < 100 { "✅" } else { "⚠️ " }
    );
    println!("  • Average: {:?}", avg_time);
    println!();

    // Throughput calculation
    let modules_per_sec_cold = 3.0 / cold_bundle_time.as_secs_f64();
    let modules_per_sec_hot = 3.0 / hot_bundle_time.as_secs_f64();

    println!("🔥 Throughput:");
    println!("  • Cold: {:.1} modules/sec", modules_per_sec_cold);
    println!("  • Hot: {:.1} modules/sec", modules_per_sec_hot);
    println!();

    // Memory usage estimate
    println!("💾 Bundle Efficiency:");
    println!("  • Output: {} bytes", output.code.len());
    println!("  • Per module: {} bytes avg", output.code.len() / 3);

    Ok(())
}
