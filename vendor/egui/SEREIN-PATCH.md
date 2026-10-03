# Bidirectional layout and editing patch

Upstream: https://github.com/emilk/egui/tree/fe6d63efa4a4df6f56ceab814d4f3a6efab69b88
(version 0.36.2, MIT OR Apache-2.0). Only `crates/egui` and `crates/epaint`, plus
the upstream README/licenses/lints, are vendored. The nested workspace lists those two
members and retains the exact git revision for other egui ecosystem crates. Package
workspace paths are explicit. Serein's root Cargo.lock is authoritative; no package
version is upgraded. `rustfmt.toml` preserves upstream spaces within this subtree.

The implementation develops the shared-renderer foundation from Serein's closed
`fix/arabic-text-bidi` branch (PR #434), independently reviewed and extended here. It
replaces the application's pre-wrap message-string reorder, rather than stacking two
reordering passes. It overlaps the open message-only PR #531; reconcile the two before
merging both. The vendored source delta is recorded in this task's source provenance.

- `text_layout.rs`: resolve Unicode directions, segment font/direction runs before
  HarfRust shaping, keep shaped clusters together at wrap/overflow boundaries, reorder
  each wrapped row after line breaking, and paint backgrounds/underlines in visual order.
- `text_layout/cluster.rs`: preserve exactly one logical source-scalar slot regardless
  of glyph count, including glyph expansions, combining marks, ligatures and ignorables.
  Native cluster artwork remains in shaper order; extra rectangles attach to source slots.
- `text_layout/bidi.rs`: line-level L1/L2 using unicode-bidi 0.3.18, with row-sized level
  buffers instead of cloning the entire job for every row. The L1 adaptation retains
  copyright 2015 The Servo Project Developers (MIT/Apache-2.0). Elision re-shapes the
  visible layout, including changed mirrored brackets and ellipsis, with its original base.
- `text_layout_types.rs` / `cursor.rs`: source-indexed caret geometry, visual Left/Right,
  paragraph direction across wrapped rows, and direction-boundary affinity.
- `egui/text_selection/{visuals,cursor_range,accesskit_text}.rs`: disjoint visual selection
  and IME ranges, directional selection collapse, logical AccessKit text/indices and
  right-edge-relative geometry for homogeneous RTL chunks. Mixed-direction AccessKit
  chunks omit detailed per-character geometry.
- `shapes/text_shape.rs` ignores the added paragraph-direction field during transforms;
  the `font_provider.rs` test helper clones Glyph, which now owns optional extra artwork.
- `image.rs`, `mesh.rs` and `text/font_face.rs` use fixed-size array chunks;
  `egui/util/id_type_map.rs` removes unused lifetimes, for the pinned Rust 1.98 Clippy.
  These small upstream lint compatibility edits preserve behavior.

Synthetic behavioral coverage lives in `crates/ui/src/rtl_tests.rs`, with bundled fonts,
actual message painting, labels and TextEdit. Native before/after WGPU captures use
`profile_preview --demo --page=rtl`; release package and process evidence are recorded
under `docs/pr-evidence/rtl-wrapping-and-editing` and `docs/performance.md`.

Limits: scalar caret cells inside native clusters use equal advance subdivisions rather
than GDEF ligature caret data. Ctrl/Option word movement retains upstream logical
behavior. Joining forms are not reshaped at forced intra-word breaks
or across rich-text format sections. Separate interactive markdown widgets retain their
existing paragraph-flow limitations. Native OS IME/event routing and screen-reader behavior
are unverified on Windows/macOS/Linux; synthetic events do not prove platform integration.
Remove this fork when upstream supplies compatible logical-indexed bidi layout/editing.
