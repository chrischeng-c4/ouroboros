//! Performance benchmarks for Argus (M5.6)
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use argus::syntax::{MultiParser, Language};
use argus::types::{
    RefactoringEngine, RefactorRequest, RefactorKind, RefactorOptions, Span,
    SemanticSearchEngine, SearchQuery, SearchKind, SearchScope,
    IncrementalAnalyzer, IncrementalConfig, ChangeKind, ContentHash,
};
use std::path::PathBuf;

// ============================================================================
// Parsing Benchmarks
// ============================================================================

fn bench_parse_python(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    let small_code = r#"
def hello():
    print("Hello, World!")
"#;

    let medium_code = r#"
class Calculator:
    def __init__(self):
        self.result = 0

    def add(self, x: int, y: int) -> int:
        return x + y

    def subtract(self, x: int, y: int) -> int:
        return x - y

    def multiply(self, x: int, y: int) -> int:
        return x * y

    def divide(self, x: int, y: int) -> float:
        if y == 0:
            raise ValueError("Division by zero")
        return x / y
"#;

    let large_code = medium_code.repeat(50); // ~500 lines

    group.bench_function("python_small", |b| {
        let mut parser = MultiParser::new().unwrap();
        b.iter(|| {
            parser.parse(black_box(small_code), Language::Python)
        });
    });

    group.bench_function("python_medium", |b| {
        let mut parser = MultiParser::new().unwrap();
        b.iter(|| {
            parser.parse(black_box(medium_code), Language::Python)
        });
    });

    group.bench_function("python_large", |b| {
        let mut parser = MultiParser::new().unwrap();
        b.iter(|| {
            parser.parse(black_box(&large_code), Language::Python)
        });
    });

    group.finish();
}

fn bench_parse_typescript(c: &mut Criterion) {
    let code = r#"
interface User {
    name: string;
    age: number;
}

function greet(user: User): string {
    return `Hello, ${user.name}!`;
}
"#;

    c.bench_function("parse_typescript", |b| {
        let mut parser = MultiParser::new().unwrap();
        b.iter(|| {
            parser.parse(black_box(code), Language::TypeScript)
        });
    });
}

// ============================================================================
// Refactoring Benchmarks
// ============================================================================

fn bench_refactoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("refactoring");

    let code = r#"
def calculate_total(items):
    result = sum(item.price for item in items)
    return result
"#;

    group.bench_function("rename_symbol", |b| {
        let mut engine = RefactoringEngine::new();
        let request = RefactorRequest {
            kind: RefactorKind::Rename {
                new_name: "total".to_string(),
            },
            file: PathBuf::from("test.py"),
            span: Span::new(0, 6), // "result"
            options: RefactorOptions::default(),
        };

        b.iter(|| {
            engine.execute(black_box(&request), black_box(code))
        });
    });

    group.bench_function("extract_variable", |b| {
        let mut engine = RefactoringEngine::new();
        let request = RefactorRequest {
            kind: RefactorKind::ExtractVariable {
                name: "item_prices".to_string(),
            },
            file: PathBuf::from("test.py"),
            span: Span::new(40, 75), // sum expression
            options: RefactorOptions::default(),
        };

        b.iter(|| {
            engine.execute(black_box(&request), black_box(code))
        });
    });

    group.finish();
}

// ============================================================================
// Semantic Search Benchmarks
// ============================================================================

fn bench_semantic_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    // Pre-populate index
    let engine = SemanticSearchEngine::new();
    for i in 0..100 {
        let _file = PathBuf::from(format!("file_{}.py", i));
        // Simulate indexed symbols (would come from actual parsing)
        // This is a simplified benchmark - real usage would parse files
    }

    group.bench_function("search_usages", |b| {
        let query = SearchQuery {
            kind: SearchKind::Usages {
                symbol: "calculate_total".to_string(),
                file: PathBuf::from("main.py"),
            },
            scope: SearchScope::Project,
            max_results: 100,
        };

        b.iter(|| {
            engine.search(black_box(&query))
        });
    });

    group.finish();
}

// ============================================================================
// Incremental Analysis Benchmarks
// ============================================================================

fn bench_incremental_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental");

    let code = r#"
import os
import sys

def process_data(data):
    return [x * 2 for x in data]
"#;

    group.bench_function("first_analysis", |b| {
        b.iter(|| {
            let config = IncrementalConfig::default();
            let mut analyzer = IncrementalAnalyzer::new(config);

            let file = PathBuf::from("test.py");
            let hash = ContentHash::from_content(black_box(code));

            analyzer.file_changed(file.clone(), ChangeKind::Created, hash);
            analyzer.analyze(vec![file])
        });
    });

    group.bench_function("cache_hit", |b| {
        let config = IncrementalConfig::default();
        let mut analyzer = IncrementalAnalyzer::new(config);

        let file = PathBuf::from("test.py");
        let hash = ContentHash::from_content(code);

        // First analysis to populate cache
        analyzer.file_changed(file.clone(), ChangeKind::Created, hash.clone());
        analyzer.analyze(vec![file.clone()]);

        b.iter(|| {
            // Second analysis should hit cache
            analyzer.analyze(black_box(vec![file.clone()]))
        });
    });

    group.bench_function("cache_invalidation", |b| {
        b.iter(|| {
            let config = IncrementalConfig::default();
            let mut analyzer = IncrementalAnalyzer::new(config);

            let file = PathBuf::from("test.py");

            // First analysis
            let hash1 = ContentHash::from_content(code);
            analyzer.file_changed(file.clone(), ChangeKind::Created, hash1);
            analyzer.analyze(vec![file.clone()]);

            // Simulate change
            let modified_code = "import os\nimport sys\nimport json\n";
            let hash2 = ContentHash::from_content(black_box(modified_code));
            analyzer.file_changed(file.clone(), ChangeKind::Modified, hash2);

            // Reanalyze
            analyzer.analyze(vec![file])
        });
    });

    group.finish();
}

// ============================================================================
// Batch Processing Benchmarks
// ============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let files: Vec<(PathBuf, String)> = (0..size)
                .map(|i| {
                    (
                        PathBuf::from(format!("file_{}.py", i)),
                        format!("def func_{}(): pass\n", i),
                    )
                })
                .collect();

            b.iter(|| {
                let mut parser = MultiParser::new().unwrap();
                let mut count = 0;

                for (_, code) in &files {
                    if parser.parse(black_box(code), Language::Python).is_some() {
                        count += 1;
                    }
                }

                count
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    parsing,
    bench_parse_python,
    bench_parse_typescript
);

criterion_group!(
    refactoring,
    bench_refactoring
);

criterion_group!(
    search,
    bench_semantic_search
);

criterion_group!(
    incremental,
    bench_incremental_analysis
);

criterion_group!(
    batch,
    bench_batch_operations
);

criterion_main!(
    parsing,
    refactoring,
    search,
    incremental,
    batch
);
