Root Cause Analysis of the Performance Regression

FastAPI’s single event loop stays fast due to uvloop and minimal cross-thread overhead. Uvicorn (FastAPI’s ASGI server) uses uvloop, a high-performance event loop written in C, instead of Python’s default asyncio loop. This yields around 10–15% higher throughput in typical workloads. Uvicorn also runs networking and request handling on the same loop thread, avoiding extra thread context switches. In contrast, our current design incurs overhead from crossing the Rust↔Python FFI boundary and thread communication for every request. Each request goes through an unbounded Tokio MPSC channel and a oneshot channel to hand off to the Python thread and back, introducing locking and synchronization cost. While each such operation is microsecond-scale, together they add up to roughly ~0.1 ms per request (about 10% of a 1ms request) – aligning with the observed 10–15% slowdown. Additionally, our Python event loop uses the pure-Python asyncio implementation, which executes a lot of Python bytecode per event, making it slower than uvloop’s optimized C routines.

GIL acquisition/release patterns also add overhead. In the thread-local model, each request acquired the GIL once inside spawn_blocking. The global loop model now acquires the GIL twice: once to execute the coroutine and again in the main thread to convert the result. The PyO3 documentation notes that attaching/detaching the Python interpreter (i.e. acquiring/releasing the GIL) is on the order of hundreds of microseconds (sub-millisecond). This overhead is small but not zero – it’s a contributing factor when aiming for sub-millisecond latency. Our global loop thread currently releases the GIL after every task and re-acquires it for the next task, whereas uvloop (being in C within a single thread) can schedule tasks without repeatedly entering/exiting the interpreter. In summary, the lack of uvloop, the cross-thread channel messaging, and the extra GIL churn explain why our single-loop approach is ~10% slower in these microbenchmarks, whereas FastAPI’s single-loop approach (with uvloop and no FFI hops) does not suffer that penalty.

Finally, task serialization vs concurrency is a subtle factor. In our global loop design, requests are processed one at a time on the Python thread. If an async handler performs I/O and awaits, our design cannot run another Python task in the meantime – the next task is sitting in the channel queue until the first is completely done. Uvicorn, however, will create a Task for each request on the event loop, so multiple handlers can interleave (when one awaits I/O, the loop can switch to another). In our sequential latency test this didn’t affect throughput (since we sent one request at a time), but in a real concurrent scenario this serialization could further hurt throughput. In essence, our current model doesn’t exploit async concurrency; it behaves closer to a synchronous queue of tasks on that single loop. FastAPI/uvicorn avoid this by letting the event loop handle multiple coroutines concurrently. This difference didn’t show up in the sequential ops/sec numbers, but it’s an architectural limitation to note.

Profiling the Overhead

To pinpoint the overhead in our implementation, we should profile both the Rust side and Python side of request handling:

CPU Flamegraph Profiling (Rust): Use a profiler like perf or cargo flamegraph on the running server to see where CPU time is spent per request. This can reveal if significant time is consumed in the channel send/recv, thread context switches, or in the Python::with_gil(...) calls. For example, a flamegraph might show the stack in the Python thread around mpsc::blocking_recv and PyEval_EvalFrame (the Python VM). If channel synchronization is a bottleneck, it will show up as noticeable time in mpsc::send or waiting on the blocking_recv. In general, these should be small, but a flamegraph will confirm their impact.

High-resolution timing: We can instrument the code to measure critical sections. For instance, record timestamps around sending the task to the channel and getting the response back. This directly measures the end-to-end overhead of our Rust↔Python round-trip (minus the handler’s actual execution). Comparing this to the FastAPI baseline handler time would quantify the overhead. Similarly, timing how long event_loop.call_method1("run_until_complete", ...) takes for a trivial coroutine (e.g., an async def noop()) would show the base overhead of scheduling a task on the global loop.

Python profiling: Although our handlers are trivial (just returning a dict), using a tool like py-spy or Python’s built-in profilers on the embedded Python thread can ensure there isn’t an unexpected Python-side bottleneck. For example, if our response conversion (py_result_to_response) is doing something heavy in Python (like serializing JSON in pure Python), that could be eating time. Based on our optimizations (using sonic-rs and lazy PyDicts), it’s likely minimal, but it’s worth verifying. Using py-spy in sampling mode attached to the running process can show how much time is spent in Python code versus native code.

PyO3 overhead: If we suspect PyO3’s API overhead, we can look at PyO3’s own guidance. The PyO3 guide notes that each Python call has some overhead (especially if converting types or errors). In our case, we do relatively few Python calls per request, so this is probably minor. But for completeness, one could measure how long it takes to convert a Python dict result to an HTTP response. If this shows up noticeably, we might optimize it (e.g., use a faster JSON serializer or move that logic to Rust).

In summary, profile each component: the channel communication, the GIL acquisition, the event loop run, and the response conversion. Given that attaching/detaching the interpreter is sub-millisecond, we expect the overhead to be the combination of small costs rather than one huge bottleneck. The goal of profiling is to find which of those “small” costs is dominating (e.g., if channel sync is, say, 50% of the overhead, we’d focus there).

Optimization Roadmap

Based on the analysis, we can pursue several optimizations to regain (and even exceed) the FastAPI parity. Below is a ranked list of improvements with expected impact:

1. Switch to uvloop for the Python Event Loop (High Impact, Low Effort)

Integrating uvloop can immediately recover a large portion of the performance gap. Uvicorn’s use of uvloop is a known reason for its speed – uvloop can be ~15% faster than asyncio even in Python 3.12+ (and historically 2-4× faster under high concurrency). We should leverage this by initializing our global loop with uvloop. This aligns with our single-loop design (just uses a faster loop implementation).

How to implement: Before creating the event loop in the Python thread, do:

Python::with_gil(|py| {
    let uvloop = py.import("uvloop").expect("Failed to import uvloop");
    uvloop.call_method0("install").expect("uvloop install failed");
});


Calling uvloop.install() sets uvloop as the default policy, so asyncio.new_event_loop() will return a uvloop instance. Alternatively, we can explicitly create a uvloop loop and set it:

Python::with_gil(|py| {
    let uvloop = py.import("uvloop")?;
    let loop_obj = uvloop.call_method0("new_event_loop")?;  // create a uvloop loop
    py.import("asyncio")?.call_method1("set_event_loop", (loop_obj.clone(),))?;
    loop_obj
});


Either approach yields a uvloop-based event loop. Expected gain: ~10% throughput boost (bringing us close to or above FastAPI). This directly addresses the “slower asyncio scheduling” issue – uvloop’s optimized internals reduce per-task overhead. It’s also fully compatible with our design (uvloop implements the same asyncio API). FastAPI’s own best practices recommend uvloop for speed, and our case is no different.

Note: Ensure uvloop is added as a dependency (it’s a PyPI package). Also, uvloop currently doesn’t support Windows (uses UNIX libuv), but that’s usually fine for deployment.

2. Use a Continuous Loop + Thread-Safe Task Scheduling (High Impact, Moderate Effort)

Right now, we call run_until_complete(coro) for each task, fully running it to completion. A more concurrent and potentially more efficient approach is to keep the event loop running (with loop.run_forever() in the dedicated thread) and schedule coroutines via thread-safe calls. In practice, this means:

Start the Python thread and initialize the event loop (as we do), but instead of looping over blocking_recv, simply call loop.run_forever() inside that thread (after setting it as the default loop).

From the Rust side, when a request comes in, use asyncio.run_coroutine_threadsafe(coro, loop) to schedule the coroutine on the running loop. This returns a concurrent.futures.Future that we can await on the Rust side for the result.

The pyo3-asyncio crate can simplify this: it provides pyo3_asyncio::tokio::into_future(py_awaitable) which under the hood does exactly that – it creates a Python Task on the loop using run_coroutine_threadsafe and uses a oneshot to send the result back to Rust. In other words, it automates what our channel+oneshot is doing, but leveraging Python’s own thread-safe scheduling (which is highly optimized using a lock-free deque for ready tasks). We attempted to use this earlier and got “coroutine was never awaited” – that likely happened because the loop wasn’t actually running (run_forever not called) or we didn’t use the provided macros to initialize the runtime. To fix that, we must ensure the loop is running continuously.

Expected benefits: This approach allows true async concurrency. Multiple Python handlers can execute concurrently on the single loop (just as in FastAPI), which is ideal for I/O-bound workloads. Even for our CPU-bound microbenchmarks, it eliminates the artificial queuing – e.g., if one request is waiting on a DB call, another can run in the interim. This reduces latency under load and increases throughput by not idling the loop. It also potentially cuts down overhead: we remove our own MPSC channel and use Python’s internal queue. The overhead of run_coroutine_threadsafe + a Future is quite low – it’s essentially an atomic enqueue of the coroutine into the loop’s deque (which is implemented in C). We’d still use a oneshot to get the result in Rust (which pyo3-asyncio does internally), but overall we drop one layer of synchronization.

In implementation terms, we might replace the PythonEventLoopExecutor::execute logic with something like:

// Pseudocode
fn execute(coro: Py<PyAny>) -> impl Future<Output=PyResult<Py<PyAny>>> {
    Python::with_gil(|py| {
        // Convert the PyAny coroutine into a Rust Future using pyo3_asyncio
        pyo3_asyncio::tokio::into_future(coro.as_ref(py))
    })?
}


And ensure the Python thread calls loop.run_forever() (plus call loop.stop() on shutdown via call_soon_threadsafe). This eliminates the manual channel send/recv for each task.

Challenges: We must properly initialize the GIL in that thread (call prepare_freethreaded_python() once in main thread before starting any Python threads – likely already done by PyO3), and be careful to join the thread on shutdown. Also, when using run_forever, the loop will not exit on its own after each task, so we need a mechanism to break out of the loop when shutting down (e.g., sending a special shutdown command to call loop.stop()).

If implementing without pyo3-asyncio, we can still do it manually: call loop.call_soon_threadsafe to schedule a coroutine and use an asyncio.Future or an Event to signal completion back. But leveraging the existing library saves us from reinventing that wheel (and likely avoids mistakes – pyo3-asyncio was built exactly for bridging Tokio and asyncio).

In summary, moving to a continuously running loop with thread-safe scheduling will make our architecture mirror Uvicorn’s design more closely. We expect it to restore true async concurrency and marginally reduce per-request overhead (one less context switch per request). It’s a moderate effort because it changes how we manage the event loop’s lifecycle, but it adheres to Python best practices (single loop, one thread) and should be stable.

3. Optimize the Rust↔Python Communication Channels (Medium Impact, Low Effort)

If we stick with the current mpsc + oneshot approach (e.g., if we don’t immediately implement run_forever), we can still squeeze out some overhead:

Use a faster channel: Tokio’s mpsc::unbounded_channel is convenient for async, but since our pattern is essentially sync (blocking recv in a dedicated thread), we could use a more efficient synchronous channel. The Crossbeam channel is a good candidate – it’s fast and lock-free for the common case. We don’t need the channel to be async because the Python thread is just doing a blocking wait. By using, say, crossbeam::channel::unbounded(), we can send from the Tokio thread and recv in the Python thread with potentially lower latency. Community benchmarks often show crossbeam’s channels outperforming std and async channels in throughput. The change would be fairly minimal (just replace the Tokio channel with a crossbeam one in PythonEventLoopExecutor). We’d still need a way to wake up the Python thread – crossbeam’s Receiver::recv() will block until an item is sent, which works fine in a dedicated thread.

Batch task processing: Another micro-optimization: currently, after each run_until_complete, we release the GIL and loop back to blocking_recv. We could instead drain the channel queue while holding the GIL, executing tasks back-to-back if multiple are queued. For example, after finishing one coroutine, check rx.try_recv() in a loop and accumulate all pending tasks, then run each via event_loop.run_until_complete without dropping the GIL between them. This would amortize the GIL overhead over batches of tasks and reduce thread context switches. It effectively processes bursts of incoming requests in one Python GIL session. The risk is if tasks are long-running, we may starve the other threads briefly, but if we keep Python tasks relatively short (which web handlers usually are), this is fine. Uvicorn’s loop naturally does this (it will keep running tasks in the loop until it needs to poll for I/O). For us, since each request is independent, it’s safe to handle them one after another without releasing GIL, as long as we eventually yield to let Tokio do other work. We just need to be careful not to block the Rust runtime too long; however, since this is a dedicated thread, it’s not blocking Tokio’s reactor – it only might delay servicing new channel messages by a few microseconds.

Main-thread GIL usage: We acquire the GIL again in the main thread to convert the Py<PyAny> result to a Response. This is a second crossing. We could explore doing that conversion in the Python thread to avoid bouncing back into Python on the main thread. For instance, the Python thread could turn the handler’s return value into a final bytes or JSON string (using Rust libraries via PyO3 or calling Python’s json.dumps if that’s what py_result_to_response does) before sending back. Then the main thread would just take a ready byte buffer and build an HTTP response without needing the GIL. This would move all Python work to the Python thread, eliminating GIL contention between threads entirely. The downside is it adds complexity (passing back a bytes/bytearray, status code, headers, etc., instead of a PyObject). However, since we control both sides, it might be doable. This is a lower priority, as the conversion cost hasn’t shown up as a big issue (and doing it in Python thread means the Python thread spends slightly more time per request). It’s a trade-off: if we ever see the main thread frequently contending on the GIL (e.g., in profiling), we could try this. Otherwise, it may not be worth it.

In summary, step 3 is about shaving off microseconds in communication. Switching to crossbeam is straightforward and safe – it should reduce any locking overhead in the channel send (Tokio’s channel is async and involves waking a task, whereas crossbeam’s blocking channel is optimized for thread hand-off). Batching tasks is a bit more custom but can be done carefully to cut down GIL thrash. These might each add only a few percent improvement, but combined could be noticeable. Given they are relatively low-effort, they’re worth implementing if steps 1 and 2 don’t already get us to parity.

4. Introduce Thread-Per-Core Event Loops (Medium Impact, High Effort)

To address the serialization and scaling beyond one core while still in one process, we could run multiple Python event loop threads in parallel, distributing incoming requests among them. This is similar to how one might use multiple workers, but within one process. Our earlier Phase 3 (thread-local loops) actually had N event loops for N concurrent requests, which is an extreme form of this (and ran into Python best-practice issues). A middle ground is to pre-spawn, say, M threads each with its own event loop (e.g., M = number of CPU cores or a fraction thereof), and use a scheduling strategy (round-robin or least-loaded) to assign each incoming request’s coroutine to one of these loops. Essentially, we’d have a pool of Python event loop threads.

Pros: This can increase throughput on multi-core systems, because now two Python handlers could truly run in parallel if one of them releases the GIL. Note that with the GIL in normal CPython, two Python threads cannot execute Python code simultaneously. However, they can interleave more readily: one thread’s Python code can run while another thread’s code is waiting on I/O or otherwise yielded. Even for CPU-bound code, multiple threads won’t run at the same instant, but they can context-switch on GIL releases (the interpreter switches threads at regular intervals or when a thread awaits I/O). The net effect is not speedup for a single CPU-bound request, but it helps when you have many concurrent I/O-bound requests. In those cases, a single event loop might become a bottleneck (it has to juggle all tasks on one CPU). Multiple loops distribute the task load and the GIL context switches across cores. This is somewhat speculative under a GIL, but frameworks like Granian have experimented with exactly this: Granian can run in multi-threaded mode vs single-threaded mode and their benchmarks suggest that with a large number of CPUs, a multi-threaded runtime scales better. They essentially run multiple event loops (each bound to one thread) in one process. We could mimic that.

Cons: Complexity goes up significantly. We would need to manage a pool of threads, each with its own channel or task queue. The routing of requests to loops needs to be consistent (to maximize cache locality, maybe stickiness, though not strictly necessary). Also, synchronization with the GIL becomes more complicated: only one Python thread can hold the GIL at a time, so if two event loop threads both have active Python work, they will time-slice the GIL between them, possibly introducing more context switch overhead and even contention. In the worst case (many CPU-bound handlers), you’d actually degrade performance because threads will be fighting over the GIL and incurring context switch overhead without real parallelism. However, for primarily I/O-bound workloads, each thread might often be waiting (releasing the GIL implicitly when doing I/O in C extensions), so another thread can run – achieving concurrency.

Implementation idea: Start M Python threads at PythonEventLoopExecutor::init(), each with its own event loop. Have a static array of executors and a scheduling algorithm (like a simple atomic counter for round-robin). The execute(coro) would pick a thread, send the task to that thread’s channel, and await the oneshot. This is similar to how one might implement a thread pool. We must ensure thread-safe initialization (PyO3’s GIL can be used by multiple OS threads as long as we called prepare_freethreaded_python(), which we have). Each thread must call Python::with_gil to create and run its loop.

We’d also need to be careful with global state in Python – since we’re still in one process, if user code assumes a single global event loop (via asyncio.get_event_loop()), it would now get different loops depending on thread. This could confuse libraries that aren’t written to handle that (though many libraries don’t assume multiple event loops unless they maintain background tasks). It’s technically allowed to have multiple event loops as long as each is in a different thread, but it’s not common. We should document that or perhaps make it opt-in.

Expected outcome: If uvloop is also used on each thread, and given our Phase 3 was ~1.09× FastAPI with effectively unlimited threads, using a fixed small number of loop threads might maintain ~1.0–1.1× baseline performance on average, even under heavy concurrency, without the large memory overhead of unbounded threads. Essentially, it’s like running multiple uvicorn worker processes but within one process (since the GIL negates true parallel CPU usage, the benefit is mainly concurrency and using multiple cores for I/O waits).

This is a bigger change, so I’d rank it after exhausting the simpler wins above. If we can reach our “Good” outcome (1.0x–1.1x FastAPI) with a single loop + uvloop, we may not need this. But if we find the single loop is a bottleneck for certain loads, thread-per-core could help. Notably, this approach should be seen as an alternative to running multiple processes – which is another way to scale beyond one core. Since we’re embedding in Rust, spinning multiple processes might not be as straightforward as in Python (where Gunicorn/Uvicorn manages workers). A thread-based approach keeps it in-process and might be easier for our deployment model.

5. Leverage Pure Rust handlers for Critical Endpoints (Targeted Impact)

We have already considered the idea of certain hot routes being handled entirely in Rust (Phase 5 Option 4). This doesn’t fix the general overhead of Python integration, but it avoids it on specific code paths. For example, a health check endpoint that just returns “OK” could be implemented in Rust and bypass Python entirely. This would be an order of magnitude faster – we’d eliminate GIL, PyO3 calls, and even Python JSON serialization. Our HTTP server (Hyper) can easily send a static response in a few microseconds. We estimated 5–10× speedups for such routes, which seems realistic. Granian’s RSGI interface is an illustration: it allows writing an endpoint that calls proto.response_str(...) directly in Rust, avoiding the ASGI send/receive dance for simple responses.

While this doesn’t improve the Python loop performance, it improves overall throughput by offloading work. It’s especially useful for very high frequency, low complexity endpoints. The complexity is manageable: we’d add an API for users to register a Rust callback (perhaps as a function pointer or a closure with a known signature) for a route. In our router, we detect that and handle it differently. Since this is an API framework, this could complicate the user-facing side (users would need to provide a Rust function somehow – maybe via a macro or by writing Rust and registering it... which is advanced for end users). If this framework is primarily for Python users, this may not be heavily used. But as a developer, we could at least implement a few internal endpoints (like the built-in /docs or debugging routes, if any) in Rust to save overhead.

This is more of a strategic optimization than a tactical fix. I’d treat it as an optional feature – if our goal is strictly to match FastAPI for typical Python endpoints, the above methods handle that. Pure Rust handlers push us into a hybrid territory (which could be a selling point: “you can drop to Rust for ultra-high performance on specific routes”).

6. Monitor Emerging Options (Future Outlook)

Looking forward, keep an eye on Python 3.13+ “no GIL” (free-threaded Python) developments. If Python eventually allows disabling the GIL (as an opt-in build), our architecture could truly run multiple Python threads in parallel. Granian 2.0 has experimental support for a no-GIL build, where it treats threads as workers instead of processes. In that scenario, a thread-per-core loop design would scale linearly with cores for CPU-bound tasks. We’d have to handle some differences (e.g. certain C extensions might not be thread-safe even if GIL is removed), but it could be a game-changer for Python API performance. For now, this is experimental – and benchmarks indicate that the no-GIL build can be slightly slower for single-thread code. So it’s not something to adopt in production yet. But our Rust integration would put us in a good position to take advantage of it when it matures (since we already manage threads and could simply not worry about GIL in that scenario).

Another area: alternative event loops like rloop (a Rust-based asyncio loop) are being developed. If uvloop maintenance stalls or we want even more performance, a Rust event loop for Python could be interesting (rloop is alpha-quality now, missing some features). But uvloop is proven and robust, so it remains our best bet in the near term.

Lastly, ensure we keep an eye on PyO3 improvements. The PyO3 team is actively working on better async integration. Our use of pyo3-asyncio is part of that. Any updates that reduce overhead (like more zero-cost conversions or better GIL handling) could directly benefit us after upgrading the library.

Recommendation and Conclusion

Primary Recommendation: Optimize the single global event loop path first. By incorporating uvloop and refining our task scheduling, we should be able to reach or exceed FastAPI’s performance while retaining the single-event-loop model (which is important for correctness and developer experience). Uvicorn + FastAPI’s performance has essentially set an upper bound that we can meet – and possibly beat, thanks to our Rust HTTP server. Each improvement targets a specific cause of our regression: uvloop addresses the slower asyncio scheduler, continuous loop scheduling removes the serialization/idling issue, and minor tuning of channels/GIL will trim the remaining fat.

If these changes are implemented, we expect to move from ~0.97× FastAPI back to 1.0–1.1× FastAPI on average (our Phase 3 showed 1.09× when we allowed more parallelism). In fact, with uvloop, it’s conceivable to even surpass FastAPI by more than 10%. FastAPI’s advantage of uvloop will be nullified, and our Rust-based HTTP parsing might give us a slight edge. For example, in the “Path Parameters” test, we’re already slightly faster (1.04×) – likely due to our router’s efficiency. With these optimizations, we can capitalize on those strengths across the board.

Architecture Decision: We should stick with the single global loop design (enhanced as described) unless proven insufficient. It keeps Python semantics simple (one loop, no surprises for library developers) and is easier to maintain. Only if we find through testing that a single loop, even with uvloop, becomes a bottleneck for extremely high loads, should we consider the thread-per-core loop pool. That would be an opt-in power-user feature in my view. The multi-loop approach introduces complexity and possible edge-case bugs (due to multiple loops) that many users won’t need if we can handle thousands of requests per second on one loop.

To put it plainly, resolve the regression by eliminating its causes rather than reverting to the Phase 3 design. Phase 3 (thread-local loops) proved performance can be high, but at the cost of Python-best-practices and potential bugs. We now know we can get performance and correctness: FastAPI is the proof, and our own Phase 6 numbers (just ~10% shy) show we’re very close.

Finally, we should validate each change with benchmarks and possibly real-world scenarios. After implementing uvloop and the continuous loop, run the same benchmark suite to confirm the ops/sec climb back above FastAPI. Also test with concurrent load (multiple concurrent clients) to see that latency and throughput behave as expected under the new scheduling. If possible, profile again to ensure no new bottlenecks were introduced.

In summary, by using uvloop and smarter scheduling, we can remove the 10-15% performance tax that came with the global loop refactor. This aligns us with FastAPI’s approach (single uvloop) and leverages our strengths (Rust speed, efficient FFI). As the creator of Granian (a similar Rust-Python server) noted, a well-designed Rust-Python integration need not add CPU overhead compared to pure Python – our goal is to make this practically true, achieving FastAPI-equivalent or better performance with one event loop. With careful tuning, we will have a solution that is both developer-friendly and high-performance.

Sources:

MagicStack uvloop – high-performance asyncio loop

Emptysquare (J. Davis) on asyncio vs threads performance and overhead

PyO3-asyncio documentation on integrating Rust futures with Python event loop

Stack Overflow discussion on uvloop usage and event loop policies

PyO3 documentation on GIL attach/detach cost

Granian author discussing Rust↔Python thread communication being non-blocking and adding no extra CPU load