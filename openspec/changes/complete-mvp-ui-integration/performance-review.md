# Performance review

## Task 1.1: UI operation coordinator

- Finding: Project, revision, and operation identity checks are constant-time and retain only one active-operation record. Each completed worker sends one result through an unbounded standard-library channel.
- Impact: CPU and steady-state memory overhead are negligible for one in-flight operation. Results can remain queued briefly if the GTK adapter does not poll, but the single-use task handle and one-active-operation rule prevent an individual worker from flooding the queue.
- I/O and concurrency: The coordinator performs no filesystem, process, or GTK work. Cooperative cancellation uses an atomic token, and result polling is non-blocking. Project switches, source-revision changes, explicit cancellation, and coordinator drop prevent late work from being accepted.
- Mitigation: Every result carries project generation, source revision, and operation identity. Stale results are discarded before they can mutate UI state. Worker adapters must check cancellation around blocking boundaries and terminate owned subprocesses where supported.
- Follow-up: Task 1.2 will exercise completion, cancellation, failure, and stale-result delivery with worker doubles. Later GTK integration must poll the channel from the main context without a busy loop and must not synchronously join long-running workers.

## Task 1.2: Operation result integration tests

- Finding: The worker double sends one terminal result synchronously and uses no threads, timers, filesystem access, or subprocesses.
- Impact: There is no production runtime cost. Test runtime and memory remain constant per scenario, with six small coordinator instances and one result each.
- Mitigation: Integration tests exercise the public UI boundary rather than GTK widgets, so cancellation and stale-result regressions remain deterministic and runnable in headless CI.
- Follow-up: Platform-specific task suites will reuse the same result contract while separately testing real worker-thread and process lifetime behavior.
