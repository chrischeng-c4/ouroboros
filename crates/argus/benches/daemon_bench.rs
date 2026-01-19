//! Benchmarks for Argus daemon performance
//!
//! Run with: cargo bench -p argus

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::PathBuf;

use argus::syntax::{Language, MultiParser};
use argus::types::{build_semantic_model, TypeChecker};

/// Sample Python code for benchmarking
const SIMPLE_PYTHON: &str = r#"
x: int = 42
y: str = "hello"
z: float = 3.14

def add(a: int, b: int) -> int:
    return a + b

def greet(name: str) -> str:
    return f"Hello, {name}!"

class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y

    def distance(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5
"#;

const COMPLEX_PYTHON: &str = r#"
from typing import List, Dict, Optional, TypeVar, Generic

T = TypeVar('T')

class Container(Generic[T]):
    def __init__(self, items: List[T]) -> None:
        self._items = items

    def get(self, index: int) -> Optional[T]:
        if 0 <= index < len(self._items):
            return self._items[index]
        return None

    def add(self, item: T) -> None:
        self._items.append(item)

    def map(self, fn: 'Callable[[T], T]') -> 'Container[T]':
        return Container([fn(item) for item in self._items])

def process_data(data: Dict[str, List[int]]) -> Dict[str, int]:
    result: Dict[str, int] = {}
    for key, values in data.items():
        if values:
            result[key] = sum(values) // len(values)
    return result

class UserService:
    def __init__(self, db: 'Database') -> None:
        self.db = db
        self.cache: Dict[int, 'User'] = {}

    def get_user(self, user_id: int) -> Optional['User']:
        if user_id in self.cache:
            return self.cache[user_id]
        user = self.db.find_user(user_id)
        if user:
            self.cache[user_id] = user
        return user

    def create_user(self, name: str, email: str) -> 'User':
        user = User(name=name, email=email)
        self.db.save_user(user)
        return user
"#;

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");

    group.bench_function("simple_python", |b| {
        let mut parser = MultiParser::new().unwrap();
        b.iter(|| {
            black_box(parser.parse(black_box(SIMPLE_PYTHON), Language::Python))
        });
    });

    group.bench_function("complex_python", |b| {
        let mut parser = MultiParser::new().unwrap();
        b.iter(|| {
            black_box(parser.parse(black_box(COMPLEX_PYTHON), Language::Python))
        });
    });

    group.finish();
}

fn bench_semantic_model(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_model");
    let mut parser = MultiParser::new().unwrap();

    group.bench_function("build_simple", |b| {
        let parsed = parser.parse(SIMPLE_PYTHON, Language::Python).unwrap();
        b.iter(|| {
            black_box(build_semantic_model(
                &parsed,
                black_box(SIMPLE_PYTHON),
                PathBuf::from("test.py"),
            ))
        });
    });

    group.bench_function("build_complex", |b| {
        let parsed = parser.parse(COMPLEX_PYTHON, Language::Python).unwrap();
        b.iter(|| {
            black_box(build_semantic_model(
                &parsed,
                black_box(COMPLEX_PYTHON),
                PathBuf::from("test.py"),
            ))
        });
    });

    group.finish();
}

fn bench_type_checking(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_checking");
    let mut parser = MultiParser::new().unwrap();

    group.bench_function("check_simple", |b| {
        let parsed = parser.parse(SIMPLE_PYTHON, Language::Python).unwrap();
        b.iter(|| {
            let mut checker = TypeChecker::new(black_box(SIMPLE_PYTHON));
            black_box(checker.check_file(&parsed))
        });
    });

    group.bench_function("check_complex", |b| {
        let parsed = parser.parse(COMPLEX_PYTHON, Language::Python).unwrap();
        b.iter(|| {
            let mut checker = TypeChecker::new(black_box(COMPLEX_PYTHON));
            black_box(checker.check_file(&parsed))
        });
    });

    group.finish();
}

fn bench_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookups");
    let mut parser = MultiParser::new().unwrap();

    let parsed = parser.parse(SIMPLE_PYTHON, Language::Python).unwrap();
    let model = build_semantic_model(&parsed, SIMPLE_PYTHON, PathBuf::from("test.py"));

    group.bench_function("type_at", |b| {
        b.iter(|| {
            // Look up type at various positions
            for line in 0..10 {
                for col in 0..20 {
                    black_box(model.type_at(line, col));
                }
            }
        });
    });

    group.bench_function("symbol_at", |b| {
        b.iter(|| {
            for line in 0..10 {
                for col in 0..20 {
                    black_box(model.symbol_at(line, col));
                }
            }
        });
    });

    group.bench_function("definition_at", |b| {
        b.iter(|| {
            for line in 0..10 {
                for col in 0..20 {
                    black_box(model.definition_at(line, col));
                }
            }
        });
    });

    group.finish();
}

fn bench_file_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_sizes");
    let mut parser = MultiParser::new().unwrap();

    // Generate different sized Python files
    for size in [10, 50, 100, 500].iter() {
        let code = generate_python_code(*size);
        group.bench_with_input(
            BenchmarkId::new("parse", size),
            &code,
            |b, code| {
                b.iter(|| {
                    black_box(parser.parse(black_box(code), Language::Python))
                });
            },
        );
    }

    for size in [10, 50, 100].iter() {
        let code = generate_python_code(*size);
        let parsed = parser.parse(&code, Language::Python).unwrap();
        group.bench_with_input(
            BenchmarkId::new("semantic_model", size),
            &(&parsed, &code),
            |b, (parsed, code)| {
                b.iter(|| {
                    black_box(build_semantic_model(
                        parsed,
                        black_box(code),
                        PathBuf::from("test.py"),
                    ))
                });
            },
        );
    }

    group.finish();
}

/// Generate Python code with n functions
fn generate_python_code(num_functions: usize) -> String {
    let mut code = String::new();
    code.push_str("from typing import List, Dict, Optional\n\n");

    for i in 0..num_functions {
        code.push_str(&format!(
            r#"
def function_{i}(arg{i}: int, items: List[str]) -> Optional[str]:
    """Function {i} docstring."""
    result = arg{i} * 2
    if result > 100:
        return items[0] if items else None
    return str(result)

"#,
            i = i
        ));
    }

    code
}

criterion_group!(
    benches,
    bench_parsing,
    bench_semantic_model,
    bench_type_checking,
    bench_lookups,
    bench_file_sizes,
);

criterion_main!(benches);
