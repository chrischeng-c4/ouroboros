"""
PyLoop vs Asyncio Performance Benchmark Suite

Comprehensive benchmarks comparing data-bridge-pyloop performance against
Python's standard asyncio event loop implementation.

Benchmark Categories:
1. Callback Scheduling Throughput (call_soon)
2. Timer Scheduling Performance (call_later with multiple timers)
3. Event Loop Overhead (empty iteration cost)

Each benchmark measures:
- Absolute performance (ops/sec, latency)
- Relative performance (speedup vs asyncio)
- Statistical consistency (warmup phase)

Usage:
    python benchmarks/pyloop/bench_event_loop.py
"""

import time
import sys
from typing import Dict, Any, Callable


def bench_pyloop_call_soon(iterations: int = 10000) -> Dict[str, Any]:
    """
    Benchmark PyLoop.call_soon throughput.

    Tests the core callback scheduling mechanism by measuring how many
    callbacks can be scheduled and executed per second. This is a fundamental
    operation that affects overall event loop performance.

    Args:
        iterations: Number of callbacks to schedule and execute

    Returns:
        Dictionary containing performance metrics:
        - duration: Total execution time in seconds
        - iterations: Number of iterations performed
        - ops_per_sec: Callbacks per second
        - us_per_op: Microseconds per callback
    """
    from data_bridge.pyloop import PyLoop

    loop = PyLoop()
    counter = [0]

    def callback():
        counter[0] += 1
        if counter[0] < iterations:
            loop.call_soon(callback)
        else:
            loop.stop()

    start = time.perf_counter()
    loop.call_soon(callback)
    loop.run_forever()
    end = time.perf_counter()

    duration = end - start
    ops_per_sec = iterations / duration

    return {
        'duration': duration,
        'iterations': iterations,
        'ops_per_sec': ops_per_sec,
        'us_per_op': (duration / iterations) * 1_000_000
    }


def bench_asyncio_call_soon(iterations: int = 10000) -> Dict[str, Any]:
    """
    Benchmark asyncio.call_soon throughput.

    Baseline measurement of Python's standard asyncio event loop for
    comparison with PyLoop performance.

    Args:
        iterations: Number of callbacks to schedule and execute

    Returns:
        Dictionary containing performance metrics (same as bench_pyloop_call_soon)
    """
    import asyncio

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    counter = [0]

    def callback():
        counter[0] += 1
        if counter[0] < iterations:
            loop.call_soon(callback)
        else:
            loop.stop()

    start = time.perf_counter()
    loop.call_soon(callback)
    loop.run_forever()
    end = time.perf_counter()
    loop.close()

    duration = end - start
    ops_per_sec = iterations / duration

    return {
        'duration': duration,
        'iterations': iterations,
        'ops_per_sec': ops_per_sec,
        'us_per_op': (duration / iterations) * 1_000_000
    }


def bench_pyloop_timers(num_timers: int = 1000) -> Dict[str, Any]:
    """
    Benchmark PyLoop.call_later with multiple timers.

    Tests timer scheduling performance by creating many timers with varying
    delays. This is critical for applications that use asyncio.sleep or
    schedule delayed operations.

    Args:
        num_timers: Number of timers to schedule

    Returns:
        Dictionary containing performance metrics:
        - duration: Total execution time in seconds
        - num_timers: Number of timers scheduled
        - timers_per_sec: Timers scheduled per second
        - avg_scheduling_time_us: Average time to schedule one timer
    """
    from data_bridge.pyloop import PyLoop

    loop = PyLoop()
    completed = [0]

    def callback():
        completed[0] += 1
        if completed[0] >= num_timers:
            loop.stop()

    start = time.perf_counter()

    # Schedule many timers with small delays (0-9ms)
    for i in range(num_timers):
        delay = (i % 10) * 0.001  # 0-9ms delays
        loop.call_later(delay, callback)

    loop.run_forever()
    end = time.perf_counter()

    duration = end - start

    return {
        'duration': duration,
        'num_timers': num_timers,
        'timers_per_sec': num_timers / duration,
        'avg_scheduling_time_us': (duration / num_timers) * 1_000_000
    }


def bench_asyncio_timers(num_timers: int = 1000) -> Dict[str, Any]:
    """
    Benchmark asyncio.call_later with multiple timers.

    Baseline measurement of Python's standard asyncio timer scheduling
    for comparison with PyLoop performance.

    Args:
        num_timers: Number of timers to schedule

    Returns:
        Dictionary containing performance metrics (same as bench_pyloop_timers)
    """
    import asyncio

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    completed = [0]

    def callback():
        completed[0] += 1
        if completed[0] >= num_timers:
            loop.stop()

    start = time.perf_counter()

    # Schedule many timers with small delays
    for i in range(num_timers):
        delay = (i % 10) * 0.001  # 0-9ms delays
        loop.call_later(delay, callback)

    loop.run_forever()
    end = time.perf_counter()
    loop.close()

    duration = end - start

    return {
        'duration': duration,
        'num_timers': num_timers,
        'timers_per_sec': num_timers / duration,
        'avg_scheduling_time_us': (duration / num_timers) * 1_000_000
    }


def bench_pyloop_empty_iterations(iterations: int = 10000) -> Dict[str, Any]:
    """
    Benchmark empty PyLoop iterations.

    Measures the baseline overhead of the event loop by timing how long it
    takes to schedule and execute empty callbacks. This reveals the minimum
    cost per iteration without any actual work being done.

    Args:
        iterations: Number of empty iterations to perform

    Returns:
        Dictionary containing performance metrics:
        - duration: Total execution time in seconds
        - iterations: Number of iterations performed
        - us_per_iteration: Microseconds per iteration
    """
    from data_bridge.pyloop import PyLoop

    loop = PyLoop()
    counter = [0]

    def tick():
        counter[0] += 1
        if counter[0] < iterations:
            loop.call_soon(tick)
        else:
            loop.stop()

    start = time.perf_counter()
    loop.call_soon(tick)
    loop.run_forever()
    end = time.perf_counter()

    return {
        'duration': end - start,
        'iterations': iterations,
        'us_per_iteration': ((end - start) / iterations) * 1_000_000
    }


def bench_asyncio_empty_iterations(iterations: int = 10000) -> Dict[str, Any]:
    """
    Benchmark empty asyncio iterations.

    Baseline measurement of Python's standard asyncio event loop overhead
    for comparison with PyLoop.

    Args:
        iterations: Number of empty iterations to perform

    Returns:
        Dictionary containing performance metrics (same as bench_pyloop_empty_iterations)
    """
    import asyncio

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    counter = [0]

    def tick():
        counter[0] += 1
        if counter[0] < iterations:
            loop.call_soon(tick)
        else:
            loop.stop()

    start = time.perf_counter()
    loop.call_soon(tick)
    loop.run_forever()
    end = time.perf_counter()
    loop.close()

    return {
        'duration': end - start,
        'iterations': iterations,
        'us_per_iteration': ((end - start) / iterations) * 1_000_000
    }


def format_results(name: str, pyloop_result: Dict[str, Any], asyncio_result: Dict[str, Any]) -> None:
    """
    Format and display benchmark results in a readable format.

    Prints comparison between PyLoop and asyncio performance, including:
    - Raw performance metrics for both implementations
    - Speedup calculation (PyLoop vs asyncio)
    - Performance improvement percentage

    Args:
        name: Name of the benchmark
        pyloop_result: Results from PyLoop benchmark
        asyncio_result: Results from asyncio benchmark
    """
    print(f"\n{'='*70}")
    print(f"Benchmark: {name}")
    print(f"{'='*70}")

    print("\nPyLoop Results:")
    for key, value in pyloop_result.items():
        if 'per_sec' in key:
            print(f"  {key}: {value:,.2f}")
        elif 'us' in key or 'per_iteration' in key:
            print(f"  {key}: {value:.3f} µs")
        elif 'duration' in key:
            print(f"  {key}: {value:.6f}s")
        else:
            print(f"  {key}: {value:,}")

    print("\nAsyncio Results:")
    for key, value in asyncio_result.items():
        if 'per_sec' in key:
            print(f"  {key}: {value:,.2f}")
        elif 'us' in key or 'per_iteration' in key:
            print(f"  {key}: {value:.3f} µs")
        elif 'duration' in key:
            print(f"  {key}: {value:.6f}s")
        else:
            print(f"  {key}: {value:,}")

    # Calculate speedup based on the most relevant metric
    if 'ops_per_sec' in pyloop_result:
        speedup = pyloop_result['ops_per_sec'] / asyncio_result['ops_per_sec']
    elif 'timers_per_sec' in pyloop_result:
        speedup = pyloop_result['timers_per_sec'] / asyncio_result['timers_per_sec']
    elif 'us_per_iteration' in pyloop_result:
        # For iteration overhead, lower is better, so invert the ratio
        speedup = asyncio_result['us_per_iteration'] / pyloop_result['us_per_iteration']
    else:
        speedup = pyloop_result['duration'] / asyncio_result['duration']

    print(f"\n{'-'*70}")
    print(f"Speedup: {speedup:.2f}x")
    if speedup > 1.0:
        print(f"PyLoop is {(speedup - 1) * 100:.1f}% faster than asyncio")
    elif speedup < 1.0:
        print(f"PyLoop is {(1 - speedup) * 100:.1f}% slower than asyncio")
    else:
        print("PyLoop and asyncio have equivalent performance")
    print('-'*70)


def run_warmup():
    """
    Run warmup iterations to stabilize performance measurements.

    Warms up both PyLoop and asyncio implementations to ensure:
    - JIT compilation is complete
    - Caches are populated
    - System resources are allocated
    - Performance measurements are consistent
    """
    print("\n" + "="*70)
    print("Warming up...")
    print("="*70)

    # Warmup PyLoop
    try:
        bench_pyloop_call_soon(1000)
        print("✓ PyLoop warmup complete")
    except Exception as e:
        print(f"✗ PyLoop warmup failed: {e}")
        return False

    # Warmup asyncio
    try:
        bench_asyncio_call_soon(1000)
        print("✓ Asyncio warmup complete")
    except Exception as e:
        print(f"✗ Asyncio warmup failed: {e}")
        return False

    return True


def run_benchmark(
    name: str,
    pyloop_fn: Callable[[], Dict[str, Any]],
    asyncio_fn: Callable[[], Dict[str, Any]]
) -> bool:
    """
    Run a single benchmark and display results.

    Args:
        name: Benchmark name
        pyloop_fn: Function that runs PyLoop benchmark
        asyncio_fn: Function that runs asyncio benchmark

    Returns:
        True if benchmark succeeded, False otherwise
    """
    print(f"\n\nRunning {name}...")

    try:
        # Run PyLoop benchmark
        pyloop_result = pyloop_fn()

        # Run asyncio benchmark
        asyncio_result = asyncio_fn()

        # Display results
        format_results(name, pyloop_result, asyncio_result)

        return True

    except Exception as e:
        print(f"\n✗ Benchmark failed: {e}")
        import traceback
        traceback.print_exc()
        return False


def main():
    """
    Run all benchmarks and display summary.

    Executes the complete benchmark suite:
    1. Warmup phase (stabilize measurements)
    2. Callback scheduling benchmark
    3. Timer scheduling benchmark
    4. Event loop overhead benchmark
    5. Summary of results
    """
    print("="*70)
    print("PyLoop vs Asyncio Performance Benchmark Suite")
    print("="*70)
    print("\nThis benchmark suite measures core event loop operations:")
    print("  1. Callback Scheduling - call_soon throughput")
    print("  2. Timer Scheduling - call_later with multiple timers")
    print("  3. Event Loop Overhead - baseline iteration cost")
    print("\nEach benchmark compares PyLoop (Rust/Tokio) against standard asyncio.")

    # Run warmup
    if not run_warmup():
        print("\n✗ Warmup failed, aborting benchmarks")
        sys.exit(1)

    # Track success
    all_passed = True

    # Benchmark 1: Callback scheduling
    all_passed &= run_benchmark(
        "Callback Scheduling (50k iterations)",
        lambda: bench_pyloop_call_soon(50000),
        lambda: bench_asyncio_call_soon(50000)
    )

    # Benchmark 2: Timer scheduling
    all_passed &= run_benchmark(
        "Timer Scheduling (5k timers)",
        lambda: bench_pyloop_timers(5000),
        lambda: bench_asyncio_timers(5000)
    )

    # Benchmark 3: Empty iterations
    all_passed &= run_benchmark(
        "Event Loop Overhead (10k iterations)",
        lambda: bench_pyloop_empty_iterations(10000),
        lambda: bench_asyncio_empty_iterations(10000)
    )

    # Print summary
    print("\n" + "="*70)
    if all_passed:
        print("✓ All Benchmarks Complete!")
    else:
        print("✗ Some benchmarks failed - see output above")
    print("="*70)
    print("\nKey Takeaways:")
    print("  - PyLoop is backed by Tokio runtime (Rust)")
    print("  - Asyncio is pure Python implementation")
    print("  - Speedup values show relative performance")
    print("  - Values >1.0x indicate PyLoop is faster")
    print("  - Lower latency (µs) is better")
    print("\nNote: This is Phase 1-2.5 implementation. Full coroutine")
    print("      execution optimization is pending (Phase 3).")
    print("="*70)

    return 0 if all_passed else 1


if __name__ == "__main__":
    sys.exit(main())
