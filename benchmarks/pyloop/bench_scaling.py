"""
PyLoop Scaling Analysis - Test performance at different scales

This script runs benchmarks at different scales to understand how
PyLoop and asyncio performance scales with workload size.

Usage:
    python benchmarks/pyloop/bench_scaling.py
"""

import time
import sys
from typing import Dict, Any, List, Tuple


def bench_callback_scaling(loop_type: str, iterations: int) -> Dict[str, Any]:
    """
    Benchmark callback scheduling at different scales.

    Args:
        loop_type: Either "pyloop" or "asyncio"
        iterations: Number of callbacks to schedule

    Returns:
        Performance metrics dictionary
    """
    if loop_type == "pyloop":
        from data_bridge.pyloop import PyLoop
        loop = PyLoop()
    else:
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

    if loop_type == "asyncio":
        loop.close()

    duration = end - start
    return {
        'iterations': iterations,
        'duration': duration,
        'ops_per_sec': iterations / duration,
        'us_per_op': (duration / iterations) * 1_000_000
    }


def bench_timer_scaling(loop_type: str, num_timers: int) -> Dict[str, Any]:
    """
    Benchmark timer scheduling at different scales.

    Args:
        loop_type: Either "pyloop" or "asyncio"
        num_timers: Number of timers to schedule

    Returns:
        Performance metrics dictionary
    """
    if loop_type == "pyloop":
        from data_bridge.pyloop import PyLoop
        loop = PyLoop()
    else:
        import asyncio
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)

    completed = [0]

    def callback():
        completed[0] += 1
        if completed[0] >= num_timers:
            loop.stop()

    start = time.perf_counter()

    for i in range(num_timers):
        delay = (i % 10) * 0.001
        loop.call_later(delay, callback)

    loop.run_forever()
    end = time.perf_counter()

    if loop_type == "asyncio":
        loop.close()

    duration = end - start
    return {
        'num_timers': num_timers,
        'duration': duration,
        'timers_per_sec': num_timers / duration,
    }


def run_scaling_analysis():
    """Run scaling analysis for both callback and timer benchmarks."""
    print("="*70)
    print("PyLoop Scaling Analysis")
    print("="*70)
    print("\nAnalyzing how performance scales with workload size...")

    # Test scales for callbacks
    callback_scales = [1_000, 5_000, 10_000, 25_000, 50_000, 100_000]

    # Test scales for timers
    timer_scales = [100, 500, 1_000, 2_500, 5_000, 10_000]

    print("\n" + "-"*70)
    print("1. Callback Scheduling Scaling")
    print("-"*70)

    print("\nPyLoop Results:")
    print(f"{'Iterations':<15} {'Duration (ms)':<15} {'Ops/Sec':<15} {'µs/Op':<15}")
    print("-"*60)

    pyloop_callback_results = []
    for iterations in callback_scales:
        result = bench_callback_scaling("pyloop", iterations)
        pyloop_callback_results.append(result)
        print(f"{result['iterations']:>13,}  "
              f"{result['duration']*1000:>13,.2f}  "
              f"{result['ops_per_sec']:>13,.0f}  "
              f"{result['us_per_op']:>13,.3f}")

    print("\nAsyncio Results:")
    print(f"{'Iterations':<15} {'Duration (ms)':<15} {'Ops/Sec':<15} {'µs/Op':<15}")
    print("-"*60)

    asyncio_callback_results = []
    for iterations in callback_scales:
        result = bench_callback_scaling("asyncio", iterations)
        asyncio_callback_results.append(result)
        print(f"{result['iterations']:>13,}  "
              f"{result['duration']*1000:>13,.2f}  "
              f"{result['ops_per_sec']:>13,.0f}  "
              f"{result['us_per_op']:>13,.3f}")

    print("\nSpeedup Analysis:")
    print(f"{'Iterations':<15} {'Speedup':<15}")
    print("-"*30)
    for pyloop, asyncio in zip(pyloop_callback_results, asyncio_callback_results):
        speedup = pyloop['ops_per_sec'] / asyncio['ops_per_sec']
        print(f"{pyloop['iterations']:>13,}  {speedup:>13,.2f}x")

    print("\n" + "-"*70)
    print("2. Timer Scheduling Scaling")
    print("-"*70)

    print("\nPyLoop Results:")
    print(f"{'Timers':<15} {'Duration (ms)':<15} {'Timers/Sec':<15}")
    print("-"*45)

    pyloop_timer_results = []
    for num_timers in timer_scales:
        result = bench_timer_scaling("pyloop", num_timers)
        pyloop_timer_results.append(result)
        print(f"{result['num_timers']:>13,}  "
              f"{result['duration']*1000:>13,.2f}  "
              f"{result['timers_per_sec']:>13,.0f}")

    print("\nAsyncio Results:")
    print(f"{'Timers':<15} {'Duration (ms)':<15} {'Timers/Sec':<15}")
    print("-"*45)

    asyncio_timer_results = []
    for num_timers in timer_scales:
        result = bench_timer_scaling("asyncio", num_timers)
        asyncio_timer_results.append(result)
        print(f"{result['num_timers']:>13,}  "
              f"{result['duration']*1000:>13,.2f}  "
              f"{result['timers_per_sec']:>13,.0f}")

    print("\nSpeedup Analysis:")
    print(f"{'Timers':<15} {'Speedup':<15}")
    print("-"*30)
    for pyloop, asyncio in zip(pyloop_timer_results, asyncio_timer_results):
        speedup = pyloop['timers_per_sec'] / asyncio['timers_per_sec']
        print(f"{pyloop['num_timers']:>13,}  {speedup:>13,.2f}x")

    # Summary
    print("\n" + "="*70)
    print("Summary")
    print("="*70)

    callback_speedups = [
        p['ops_per_sec'] / a['ops_per_sec']
        for p, a in zip(pyloop_callback_results, asyncio_callback_results)
    ]
    timer_speedups = [
        p['timers_per_sec'] / a['timers_per_sec']
        for p, a in zip(pyloop_timer_results, asyncio_timer_results)
    ]

    print("\nCallback Scheduling:")
    print(f"  Average Speedup: {sum(callback_speedups)/len(callback_speedups):.2f}x")
    print(f"  Min Speedup:     {min(callback_speedups):.2f}x")
    print(f"  Max Speedup:     {max(callback_speedups):.2f}x")

    print("\nTimer Scheduling:")
    print(f"  Average Speedup: {sum(timer_speedups)/len(timer_speedups):.2f}x")
    print(f"  Min Speedup:     {min(timer_speedups):.2f}x")
    print(f"  Max Speedup:     {max(timer_speedups):.2f}x")

    # Check if performance scales linearly
    print("\nScaling Characteristics:")

    # Check callback scaling
    first_ops = pyloop_callback_results[0]['ops_per_sec']
    last_ops = pyloop_callback_results[-1]['ops_per_sec']
    ops_variance = (last_ops - first_ops) / first_ops * 100

    print(f"\nCallback Scheduling (PyLoop):")
    print(f"  First:  {first_ops:,.0f} ops/sec ({callback_scales[0]:,} iterations)")
    print(f"  Last:   {last_ops:,.0f} ops/sec ({callback_scales[-1]:,} iterations)")
    print(f"  Change: {ops_variance:+.1f}%")
    if abs(ops_variance) < 10:
        print("  ✓ Scales linearly (consistent throughput)")
    else:
        print("  ⚠ Does not scale linearly")

    first_timer = pyloop_timer_results[0]['timers_per_sec']
    last_timer = pyloop_timer_results[-1]['timers_per_sec']
    timer_variance = (last_timer - first_timer) / first_timer * 100

    print(f"\nTimer Scheduling (PyLoop):")
    print(f"  First:  {first_timer:,.0f} timers/sec ({timer_scales[0]:,} timers)")
    print(f"  Last:   {last_timer:,.0f} timers/sec ({timer_scales[-1]:,} timers)")
    print(f"  Change: {timer_variance:+.1f}%")
    if abs(timer_variance) < 10:
        print("  ✓ Scales linearly (consistent throughput)")
    else:
        print("  ⚠ Does not scale linearly")

    print("\n" + "="*70)


def main():
    """Run scaling analysis."""
    try:
        run_scaling_analysis()
        return 0
    except Exception as e:
        print(f"\n✗ Scaling analysis failed: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
