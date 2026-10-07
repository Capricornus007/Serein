# Performance findings

Recent synthetic/offline measurements are workload-specific. They do not establish live Discord
performance, universal device results or application-wide memory bounds. The raw PR screenshot,
log and per-run evidence archive has been removed; the summaries below retain the useful results.

## Voice default-device polling — October 7, 2026

An offline probe compared creating a fresh PulseAudio client for every metadata poll with reusing
one client. Across five measured runs after warmup, 300 polls created 300 clients and left 300 peer
sockets connected in the fresh-client pattern; reuse created one client and left one socket. The
probe used synthetic socket pairs and a 2 ms pause per query. This identifies dependency resource
retention in the polling pattern; it does not establish application socket behavior or whole-process
memory use.

Both standard voice-enabled macOS packages were the same size: executable 68,180,992 B, installed
app 74,201,013 B, distribution 74,265,778 B. ZIP sizes differed by 37 B (48,039,907 vs 48,039,944 B),
which is packaging variation, not a runtime change. No Discord call, microphone, physical device,
callback latency, frame timing or CPU/RSS comparison was measured.

## CPU and memory audit — October 2, 2026

Release workloads compared a 100,000-event synthetic timeline, cursor scans, and synthetic video
frame ownership on an Apple M1 with Rust 1.98.1:

| Workload | Before | After | Result |
| --- | ---: | ---: | ---: |
| Reducer replay median | 153.985 ms | 53.504 ms | −65.25% |
| Full-history cursor query batch | 263.428 ms | 6.948 ms | −97.36% |
| Cursor batch with deleted tail rows | 341.132 ms | 16.075 ms | −95.29% |
| Coalesced 1080p frame peak RSS | 34.406 MiB | 26.516 MiB | −22.93% |
| Standard executable | 62,006,096 B | 62,006,112 B | +16 B |
| Installed bundle | 68,016,525 B | 68,016,541 B | +16 B |

The reducer retained 500 rows in both builds. Cursor timings isolate hot lookup work and do not
predict whole-UI gains. Frame conversion time was effectively unchanged; the lower peak RSS came
from reusing the undisplayed pending frame. With uploads every third frame, peak RSS was unchanged.

A focused native idle sample showed mean CPU of 1.180% before and 1.275% after, and settled RSS of
108.766 MiB and 111.484 MiB. This small, noisy sample supports no idle-performance improvement
claim. Startup latency, full-frame p95, GPU memory, live traffic and other platforms were unmeasured.

These workloads can be repeated with the pinned toolchain using `cargo replay` for the reducer and
the existing ignored client-core, desktop-frame and replay-soak workloads for detailed memory work.
The delivery skill documents how to compare a task baseline with the changed build. Do not compare
results from different machines or claim live-client behavior from synthetic fixtures.
