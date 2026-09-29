# Provenance and local changes

- Upstream: https://github.com/Rikorose/DeepFilterNet
- Revision: `d375b2d8309e0935d165700c91da9de862a99c31` (`0.5.7-pre`).
- Copied source: `libDF/src/lib.rs`, `libDF/src/tract.rs`.
- Copied model: `models/DeepFilterNet3_onnx.tar.gz`, 7,983,136 bytes.
- Model SHA-256: `c94d91f70911001c946e0fabb4aa9adc37045f45a03b56008cb0c8244cb63616`.
  These bytes also match upstream release `v0.5.6`, revision
  `978576aa8400552a4ce9730838c635aa30db5e61`.
- Copied license files: upstream `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`.

Hendrik Schröter and DeepFilterNet contributors retain their upstream copyrights.
DeepFilterNet3 paper: [DeepFilterNet: Perceptually Motivated Real-Time Speech
Enhancement](https://arxiv.org/abs/2305.08227), Schröter, Rosenkranz, Escalante-B.
and Maier, INTERSPEECH 2023.

The upstream root license offers MIT or Apache-2.0; README describes the scope as
“all code”. The model archive has no separate license entry. Explicit questions
about pretrained-weight redistribution are still open and unanswered by the
maintainer as checked on 2026-09-29:
[issue 697](https://github.com/Rikorose/DeepFilterNet/issues/697) and
[issue 700](https://github.com/Rikorose/DeepFilterNet/issues/700).
Serein preserves the repository licenses and provenance; it does not represent
the model-specific clarification as received.

Local changes:

1. Keep only the embedded mono inference runtime. Remove training, datasets,
   resampling, codecs, CLI, C/Python/WebAssembly APIs and optional logging queue.
   The vendored crate accepts no arbitrary external model files or bytes.
2. Pin the Tract runtime family to `0.22.4` and ndarray to `0.16.1`. This avoids
   Tract 0.21's upper bounds on `time` and exact older `half` dependency; the
   inference architecture and model weights are unchanged. Adapt the renamed
   graph symbol field and ndarray reshape method to those dependency versions.
3. Add a complete `DfTract::reset()` using Tract's `reset_turn`, fresh session
   state and `reset_op_states`. Reset STFT buffers, normalization, spectral
   rings, features and silence counter. Compiled model and FFT plans are retained.
   Recurrent state allocation is bounded by the fixed model; this is worker work.
4. Make the construction-only initializer private and clear both spectral rings
   there. Upstream's public `init()` was not a full reset and appended one ring.
5. Replace shared `TValue` feature buffers and pointer mutation with owned
   `Tensor` buffers and checked mutable views. Initialize tensors to zero rather
   than uninitialized storage. Remove unused view helpers with unconstrained
   output lifetimes and use mutable pointers for remaining mutable conversions.
   Check complex view contiguity, shapes and float-pair layout before casts.
6. Enforce contiguous, mono hop-sized input/output in release builds and use a
   saturating silence counter. Preserve upstream's five-frame silence flush.
7. Share compiled inference plans through `Arc` and add `freeze` / `unfreeze`
   using Tract's safe frozen-state API. Preparation may cross a thread boundary
   without sending `Rc` inference values, cloning graphs, or recompiling models.
8. Add a deterministic reset regression: process speech-like audio and silence,
   reset repeatedly, require outputs to match a fresh stream exactly and require
   spectral history lengths to stay fixed. No recorded or live audio is used.

No model weights were changed or retrained. Synthetic tests establish resource
and state behavior, not perceptual quality or live-call compatibility.
