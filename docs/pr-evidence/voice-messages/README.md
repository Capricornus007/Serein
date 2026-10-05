# Voice Messages evidence

Baseline: `6313e271` (fetched `origin/main`), from a detached baseline worktree.
Task branch: `feat/voice-messages`. Original checkout's untracked
`community-extensions/` and `docs/pr-evidence/profile-client-platforms/` were
preserved in that checkout; all task work used a separate worktree.

## Synthetic native screenshots

All images are actual eframe native windows on Xvfb/X11, scale 1, WGPU/Vulkan
llvmpipe (LLVM 21.1.8). They contain only synthetic offline conversations.
`before.png` shows the baseline without a recorder. `after.png` shows the new
explicit fixture recording with generated waveform/timer data: no audio device,
recording bytes or network send. Both use a 1120×760 dark viewport.
`light-narrow.png` verifies the ready dialog at 760×520, the minimum window size.

Build/run the baseline with `cargo build --locked -p serein --features demo`, then
`./target/debug/serein --demo --demo-chat`. Build the task the same way and add
`--demo-recorder`; add `--demo-light` for the light variant. Choose the composer
**+ → Record a voice message → Record → Stop**. Review shows **Send** disabled in
demo. Escape closes/discards the dialog. The actual production entry is opt-in:
Settings → Extensions → Voice Messages → install/enable and grant access.

Native inspection covered the menu, ready/recording/review states, elapsed timer,
waveform, disabled synthetic Send, Escape and light minimum-width/height layout.
Xwayland XTest clicks did not reliably target the window, so evidence was captured
on an isolated Xvfb display. Window-only XWD captures were converted to PNG
without altering content; each committed image is below 2 MiB.

## Release measurement reproduction

Standard packages on both revisions use pinned Rust 1.98.1 and
`cargo xtask package` (voice included; no default/demo features). Package smoke
checks verify the installed tree. Installed bytes sum regular files extracted by
`dpkg-deb -x`; executable and complete `.deb` sizes come from the separate `dist/`
directories. Package hashes and raw native samples are in `measurements.json`.

For process measurements build each revision with:

```bash
cargo build --locked --release -p serein --no-default-features --features demo
DISPLAY=:77 LD_LIBRARY_PATH=<xdotool-library-directory> python3 docs/pr-evidence/voice-messages/sample.py <copied-release-executable> <samples.json> --xdotool <xdotool-path>
```

Use Xvfb at 1120×760×24, scale 1. The script forces Mesa lavapipe, makes the same
neutral channel-header click on both builds, warms up for eleven seconds total,
and takes twenty one-second CPU/RSS samples. CPU is percent of one logical core;
settled RSS is the median of the final five samples. No microphone is opened and
no recorder dialog is active during the matched idle comparison. The baseline
ignores `--demo-recorder`; the task enables its synthetic opt-in contribution.
This measures idle integration overhead, not physical microphone DSP, encoding,
network delivery, startup peaks, GPU memory or frame latency.
