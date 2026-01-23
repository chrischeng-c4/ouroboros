/// Performance benchmark for larger application (10+ modules)
use ouroboros_talos_bundler::{Bundler, BundleOptions};
use ouroboros_talos_resolver::ResolveOptions;
use ouroboros_talos_transform::TransformOptions;
use ouroboros_talos_asset::AssetOptions;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("⚡ Talos Bundler - Large Application Benchmark\n");

    // Create entry point that uses LargeApp
    std::fs::write(
        "/tmp/talos-example/src/index-large.js",
        r#"import React from 'react';
import ReactDOM from 'react-dom/client';
import LargeApp from './LargeApp.jsx';

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(<LargeApp />);
"#
    )?;

    let options = BundleOptions {
        entry: PathBuf::from("/tmp/talos-example/src/index-large.js"),
        output_dir: PathBuf::from("/tmp/talos-example/dist"),
        source_maps: true,
        minify: false,
        externals: HashSet::new(),
        resolve_options: ResolveOptions::default(),
        transform_options: TransformOptions::default(),
        asset_options: AssetOptions::default(),
    };

    println!("📊 Benchmarking with realistic application");
    println!("{}", "=".repeat(60));
    println!("Application structure:");
    println!("  • Entry: index-large.js");
    println!("  • Main component: LargeApp.jsx");
    println!("  • Components: Header, Footer, Card, Button (4)");
    println!("  • Hooks: useCounter, useToggle (2)");
    println!("  • Utils: math, string (2)");
    println!("  • Styles: styles.css");
    println!("  • Expected modules: 10+");
    println!();

    // Cold start
    println!("📊 Cold Start");
    println!("{}", "=".repeat(60));

    let start = Instant::now();
    let bundler = Bundler::new(options.clone())?;
    let init_time = start.elapsed();

    let start = Instant::now();
    let entry = PathBuf::from("/tmp/talos-example/src/index-large.js");
    let output = bundler.bundle(entry.clone()).await?;
    let cold_time = start.elapsed();

    println!("⏱  Initialization: {:?}", init_time);
    println!("⏱  Cold bundle: {:?}", cold_time);
    println!("📦 Bundle size: {} KB", output.code.len() / 1024);
    println!();

    // Hot rebuild
    println!("📊 Hot Rebuild (100 iterations)");
    println!("{}", "=".repeat(60));

    let mut times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        bundler.bundle(entry.clone()).await?;
        times.push(start.elapsed());
    }

    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    let min = times.iter().min().unwrap();
    let max = times.iter().max().unwrap();
    let p50 = times[times.len() / 2];
    let p95 = times[times.len() * 95 / 100];
    let p99 = times[times.len() * 99 / 100];

    println!("⏱  Average: {:?}", avg);
    println!("⏱  Min: {:?}", min);
    println!("⏱  Max: {:?}", max);
    println!("⏱  P50: {:?}", p50);
    println!("⏱  P95: {:?}", p95);
    println!("⏱  P99: {:?}", p99);
    println!();

    // Performance analysis
    println!("📈 Performance Analysis");
    println!("{}", "=".repeat(60));

    let module_count = 11; // Estimated from app structure
    let cold_modules_per_ms = module_count as f64 / cold_time.as_micros() as f64 * 1000.0;
    let hot_modules_per_ms = module_count as f64 / avg.as_micros() as f64 * 1000.0;

    println!("Cold build:");
    println!("  • Total time: {:?}", cold_time);
    println!("  • Throughput: {:.1} modules/ms", cold_modules_per_ms);
    println!("  • Per module: {:?}", cold_time / module_count);
    println!();

    println!("Hot rebuild:");
    println!("  • Average time: {:?}", avg);
    println!("  • Throughput: {:.1} modules/ms", hot_modules_per_ms);
    println!("  • Per module: {:?}", avg / module_count);
    println!("  • Speedup vs cold: {:.2}x", cold_time.as_micros() as f64 / avg.as_micros() as f64);
    println!();

    // Comparison with targets
    println!("🎯 Target Comparison");
    println!("{}", "=".repeat(60));
    println!("Phase 1 targets (10 modules):");
    println!("  • Cold: < 500ms");
    println!("  • Incremental: < 100ms");
    println!("  • HMR propagation: < 50ms");
    println!();
    println!("Actual performance:");
    println!("  • Cold: {:?} {}",
        cold_time,
        if cold_time.as_millis() < 500 { "✅" } else { "❌" }
    );
    println!("  • Hot: {:?} {}",
        avg,
        if avg.as_millis() < 100 { "✅" } else { "⚠️" }
    );
    println!("  • P99: {:?} {}",
        p99,
        if p99.as_millis() < 50 { "✅" } else { "⚠️" }
    );

    Ok(())
}
