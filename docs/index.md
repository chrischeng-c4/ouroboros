# data-bridge

**High-performance MongoDB ORM for Python with Rust backend.**

data-bridge is a Beanie-compatible ORM that handles all BSON serialization and CPU-intensive tasks in Rust, offering significant performance improvements (1.4x - 5.4x faster).

---

## Documentation

Please select your language:

*   [English User Guide](en/user-guide.md)
*   [繁體中文使用指南 (Traditional Chinese)](zh-tw/user-guide.md)

---

## Features

*   **Fast**: Core engine written in Rust.
*   **Safe**: Type validation and memory safety.
*   **Compatible**: Drop-in replacement for Beanie (mostly).
*   **Async**: Built on `tokio` and `motor`.
