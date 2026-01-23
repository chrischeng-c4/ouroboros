/// Stress test benchmark (30+ modules)
use ouroboros_talos_bundler::{Bundler, BundleOptions};
use ouroboros_talos_resolver::ResolveOptions;
use ouroboros_talos_transform::TransformOptions;
use ouroboros_talos_asset::AssetOptions;
use std::path::PathBuf;
use std::time::Instant;
use std::collections::HashSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔥 Talos Bundler - Stress Test (30+ modules)\n");

    std::fs::write(
        "/tmp/talos-example/src/index-stress.js",
        r#"import React from 'react';
import ReactDOM from 'react-dom/client';
import StressApp from './StressApp.jsx';

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(<StressApp />);
"#
    )?;

    let options = BundleOptions {
        entry: PathBuf::from("/tmp/talos-example/src/index-stress.js"),
        output_dir: PathBuf::from("/tmp/talos-example/dist"),
        source_maps: false,
        minify: false,
        externals: HashSet::new(),
        resolve_options: ResolveOptions::default(),
        transform_options: TransformOptions::default(),
        asset_options: AssetOptions::default(),
    };

    println!("📊 Configuration");
    println!("{}", "=".repeat(60));
    println!("Expected modules: 31+");
    println!("  • Entry + StressApp (2)");
    println!("  • Components (4): Header, Footer, Card, Button");
    println!("  • Hooks (2): useCounter, useToggle");
    println!("  • Utils (2): math, string");
    println!("  • Generated modules (20): module1-20.js");
    println!("  • Styles (1): styles.css");
    println!();

    // Cold start
    let start = Instant::now();
    let bundler = Bundler::new(options)?;
    let init_time = start.elapsed();

    let start = Instant::now();
    let entry = PathBuf::from("/tmp/talos-example/src/index-stress.js");
    let output = bundler.bundle(entry.clone()).await?;
    let cold_time = start.elapsed();

    println!("📊 Cold Start");
    println!("{}", "=".repeat(60));
    println!("⏱  Initialization: {:?}", init_time);
    println!("⏱  Cold bundle: {:?}", cold_time);
    println!("📦 Bundle size: {} KB", output.code.len() / 1024);
    println!();

    // Warm iterations
    println!("📊 Warm Iterations (1000 runs)");
    println!("{}", "=".repeat(60));

    let mut times = Vec::new();
    let iterations = 1000;

    let overall_start = Instant::now();
    for i in 1..=iterations {
        let start = Instant::now();
        bundler.bundle(entry.clone()).await?;
        times.push(start.elapsed());

        if i % 100 == 0 {
            print!("  Progress: {}/{}  \r", i, iterations);
        }
    }
    let overall_time = overall_start.elapsed();
    println!();

    times.sort();
    let avg = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    let min = times[0];
    let max = times[times.len() - 1];
    let p50 = times[times.len() / 2];
    let p95 = times[times.len() * 95 / 100];
    let p99 = times[times.len() * 99 / 100];
    let p999 = times[times.len() * 999 / 1000];

    println!("⏱  Average: {:?}", avg);
    println!("⏱  Median (P50): {:?}", p50);
    println!("⏱  P95: {:?}", p95);
    println!("⏱  P99: {:?}", p99);
    println!("⏱  P99.9: {:?}", p999);
    println!("⏱  Min: {:?}", min);
    println!("⏱  Max: {:?}", max);
    println!();

    // Throughput analysis
    println!("📈 Throughput Analysis");
    println!("{}", "=".repeat(60));

    let total_bundles = iterations as f64;
    let bundles_per_sec = total_bundles / overall_time.as_secs_f64();
    let avg_modules = 31.0;
    let modules_per_sec = bundles_per_sec * avg_modules;

    println!("Overall performance:");
    println!("  • Total time: {:?}", overall_time);
    println!("  • Bundles/sec: {:.1}", bundles_per_sec);
    println!("  • Modules/sec: {:.1}", modules_per_sec);
    println!("  • Per-module latency: {:?}", avg / avg_modules as u32);
    println!();

    println!("Latency distribution:");
    println!("  • Best case (min): {:?}", min);
    println!("  • Typical (P50): {:?}", p50);
    println!("  • Good (P95): {:?}", p95);
    println!("  • Acceptable (P99): {:?}", p99);
    println!("  • Edge case (P99.9): {:?}", p999);
    println!();

    // Scalability analysis
    println!("🎯 Scalability Analysis");
    println!("{}", "=".repeat(60));

    let small_app_time = 0.473; // ms from previous benchmark (3 modules)
    let large_app_time = p50.as_micros() as f64 / 1000.0; // This test (31 modules)
    let scaling_factor = large_app_time / small_app_time;
    let module_ratio = 31.0 / 3.0;

    println!("Scaling comparison:");
    println!("  • 3 modules → 31 modules (10.3x more)");
    println!("  • Time increase: {:.2}x", scaling_factor);
    println!("  • Efficiency: {:.1}% (100% = perfect linear scaling)",
        (module_ratio / scaling_factor) * 100.0
    );
    println!();

    println!("Performance targets:");
    println!("  • Cold build < 500ms: {:?} {}",
        cold_time,
        if cold_time.as_millis() < 500 { "✅" } else { "❌" }
    );
    println!("  • Hot rebuild < 100ms: {:?} {}",
        p50,
        if p50.as_millis() < 100 { "✅" } else { "❌" }
    );
    println!("  • P99 < 50ms: {:?} {}",
        p99,
        if p99.as_millis() < 50 { "✅" } else { "⚠️" }
    );

    Ok(())
}
