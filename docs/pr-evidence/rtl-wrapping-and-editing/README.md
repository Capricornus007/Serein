# RTL wrapping and editing evidence

Baseline: `ca1956d6d41700409d38a00dbae1a1999f30218a` (`origin/main` at task start).
The original checkout's unrelated untracked paths were preserved; this task uses
`fix/rtl-wrapping-and-editing` in a separate worktree. Before captures use the new
fixture-only `prime_rtl_chat` harness with unchanged baseline UI/renderer code.
All content, channel names, authors and drafts are synthetic; no account or transports
are available in the preview. Images are native WGPU framebuffer captures, not mockups.

| Capture | Viewport | Appearance | Scale |
| --- | --- | --- | --- |
| `before.png`, `after.png` | 1120 × 760 | Dark | 1.0 |
| `before-light-narrow.png`, `after-light-narrow.png` | 760 × 760 | Light | 1.0 |

The 760px width is the desktop application's supported minimum. Additional inspected
760 × 520 dark captures exercised scripted wheel scrolling and the tall wrapped
composer; their build outputs remain ignored. A 540px stress capture is below the
product's minimum width and is not presented as supported layout evidence.

Reproduce the inspected scenarios:

```sh
cargo rustc --release --locked -p serein --example profile_preview --features demo -- -C lto=off
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET target/release/examples/profile_preview --demo --page=rtl --output=rtl.png
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET target/release/examples/profile_preview --demo --page=rtl --light --width=760 --output=rtl-light.png
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET target/release/examples/profile_preview --demo --page=rtl --width=760 --height=520 --scroll=-300 --output=rtl-scroll.png
```

Only the preview's final crate uses `lto=off`, identically for before/after captures
and native process sampling. Standard distribution measurements use fresh
`cargo xtask package` outputs with the normal release profile and voice included.
Production dependency versions are unchanged; egui/epaint resolve to the exact
vendored 0.36.2 source revision. `source-provenance.json` records all local deltas,
and an aggregate identity for unchanged upstream files.

Behavioral verification uses actual fonts, painted message galleys and native egui
TextEdit events. Sixteen RTL tests cover four languages at three widths, labels,
explicit newlines, mixed Latin/digits, directional controls, marks/expansions,
styles/emoji/mention slots, alignment and elision, bracket artwork, preserved
indent/base, visual arrows and multi-row selection collapse, disjoint selection,
copy/paste/delete and IME preedit/commit. Four of the initial five tests failed on
the baseline. The bracket-artwork test additionally reproduced a hidden-context
mirroring bug before its fix. The final workspace check passed 1,146 tests with
24 existing ignored tests; renderer/editor suites passed 93 epaint + 67 egui tests.
Strict workspace, preview and standalone vendored-library Clippy passed, with no
checks disabled. The supplemental vendored dev-test workspace used an ignored
lockfile initialized from the production lock plus upstream dev-only dependencies;
it does not change the shipped dependency graph.

`measurements.json` records package identities, raw process samples and screenshot
identities. Process sampling opens the same seeded RTL conversation, warms up for
8 seconds, settles for 3 seconds, then takes twenty 1-second psutil samples. CPU is
one-core process percentage; RSS is resident process memory, not GPU memory. Settled
RSS is the median of the final five samples; child processes are recorded separately.
No Cargo build runs during sampling. This is one before/after pair, not a confidence
interval or active frame/startup benchmark. Host and renderer are recorded there and
in `docs/performance.md`.

Limits: native OS IME/event routing, screen readers and other OS rendering remain
unverified. Equal scalar caret cells, word-wise navigation, forced intra-word joining
and separate interactive markdown widget limits are documented in
`vendor/egui/SEREIN-PATCH.md`. This shared-renderer work develops the foundation of
closed PR #434 and overlaps open message-only PR #531; reconcile those alternatives
before merging both. Synthetic evidence does not establish Discord interoperability.
