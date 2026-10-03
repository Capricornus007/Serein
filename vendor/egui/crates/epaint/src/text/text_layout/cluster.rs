//! Emit one logical editing slot per source scalar, independently of shaped glyph count.
//!
//! `HarfRust`'s monotone clusters are consumed in source order. Native artwork retains the
//! shaper's visual order within each cluster; the parent reorders complete rows afterwards.

use super::*;

pub(super) fn layout_shaped_run(
    fonts: &mut FontsImpl,
    run: &TextRun,
    text: &str,
    buffer: &harfrust::GlyphBuffer,
    metrics: &StyledMetrics,
    ctx: &mut ShapingContext,
    paragraph: &mut Paragraph,
) {
    let infos = buffer.glyph_infos();
    let positions = buffer.glyph_positions();
    let before = paragraph.glyphs.len();

    // Default-ignorable source characters must survive even if a shaper emits no artwork.
    if infos.is_empty() {
        emit_empty(text, run.rtl, metrics, ctx, paragraph);
        return;
    }

    if run.rtl {
        let first_byte = infos[infos.len() - 1].cluster as usize;
        emit_empty(&text[..first_byte], true, metrics, ctx, paragraph);
        let mut end = infos.len();
        while end > 0 {
            let from = infos[end - 1].cluster as usize;
            let mut start = end - 1;
            while start > 0 && infos[start - 1].cluster as usize == from {
                start -= 1;
            }
            let to = if start > 0 {
                infos[start - 1].cluster as usize
            } else {
                text.len()
            };
            emit_cluster(
                fonts,
                run,
                &text[from..to],
                &infos[start..end],
                &positions[start..end],
                metrics,
                ctx,
                paragraph,
            );
            end = start;
        }
    } else {
        let first_byte = infos[0].cluster as usize;
        emit_empty(&text[..first_byte], false, metrics, ctx, paragraph);
        let mut start = 0;
        while start < infos.len() {
            let from = infos[start].cluster as usize;
            let mut end = start + 1;
            while end < infos.len() && infos[end].cluster as usize == from {
                end += 1;
            }
            let to = infos
                .get(end)
                .map_or(text.len(), |info| info.cluster as usize);
            emit_cluster(
                fonts,
                run,
                &text[from..to],
                &infos[start..end],
                &positions[start..end],
                metrics,
                ctx,
                paragraph,
            );
            start = end;
        }
    }

    debug_assert_eq!(
        paragraph.glyphs.len() - before,
        text.chars().count(),
        "shaping must preserve one slot per source scalar"
    );
}

fn begin_cluster(ctx: &mut ShapingContext, paragraph: &mut Paragraph) {
    if !ctx.is_first_glyph_in_section {
        paragraph.cursor_x_px += ctx.extra_letter_spacing * ctx.pixels_per_point;
    }
    ctx.is_first_glyph_in_section = false;
}

fn emit_empty(
    text: &str,
    rtl: bool,
    metrics: &StyledMetrics,
    ctx: &mut ShapingContext,
    paragraph: &mut Paragraph,
) {
    if text.is_empty() {
        return;
    }
    begin_cluster(ctx, paragraph);
    for (index, chr) in text.chars().enumerate() {
        let mut glyph = ctx.glyph(chr, 0, 0.0, metrics, GlyphAllocation::default());
        glyph.pos.x = paragraph.cursor_x_px / ctx.pixels_per_point;
        glyph.is_rtl = rtl;
        glyph.cluster_start = index == 0;
        paragraph.glyphs.push(glyph);
    }
}

#[expect(clippy::too_many_arguments)]
fn emit_cluster(
    fonts: &mut FontsImpl,
    run: &TextRun,
    text: &str,
    infos: &[harfrust::GlyphInfo],
    positions: &[harfrust::GlyphPosition],
    metrics: &StyledMetrics,
    ctx: &mut ShapingContext,
    paragraph: &mut Paragraph,
) {
    let count = text.chars().count();
    let Some(chr) = text.chars().next() else {
        return;
    };
    begin_cluster(ctx, paragraph);
    let origin = paragraph.cursor_x_px;
    let scale = metrics.px_scale_factor;

    // Single glyphs and ordinary base/mark clusters use only inline storage. The shaper
    // already owns its bounded run buffer; this temporary retains only the current cluster.
    let mut artwork = smallvec::SmallVec::<[Glyph; 4]>::new();
    for (index, (info, pos)) in infos.iter().zip(positions).enumerate() {
        let glyph_id = skrifa::GlyphId::new(info.glyph_id);
        let glyph = if glyph_id == skrifa::GlyphId::NOTDEF {
            // Match the upstream fallback: one base bitmap/tofu, no duplicate NOTDEF
            // artwork and no tofu for a missing combining mark. Logical slots remain.
            if index > 0 || is_combining_mark(chr) {
                continue;
            }
            let cluster_text = unicode_segmentation::UnicodeSegmentation::graphemes(text, true)
                .next()
                .unwrap_or_default();
            if let Some(raster) = fonts.rasterize_cluster(
                FontPriority::Lowest,
                ctx.family,
                cluster_text,
                ctx.pixels_per_point,
                ctx.font_size,
            ) {
                raster_glyph(ctx, paragraph, chr, &raster, metrics)
            } else {
                let fallback_key = fonts.resolve_face(ctx.family, chr);
                let fallback_metrics = fonts
                    .face(fallback_key)
                    .map(|face| {
                        face.styled_metrics(
                            ctx.pixels_per_point,
                            ctx.font_size,
                            &Default::default(),
                        )
                    })
                    .unwrap_or_default();
                let (_, glyph_info) = fonts.glyph_info(ctx.family, chr, &fallback_metrics);
                if glyph_info.id == Some(skrifa::GlyphId::NOTDEF) {
                    fonts.on_missing_glyph(ctx.family, cluster_text);
                }
                let advance =
                    glyph_info.advance_width_unscaled.0 * fallback_metrics.px_scale_factor;
                let OutlineGlyph { allocation, x_px } = allocate_glyph_info(
                    fonts,
                    fallback_key,
                    &fallback_metrics,
                    glyph_info,
                    paragraph.cursor_x_px,
                    chr,
                );
                paragraph.cursor_x_px += advance;
                ctx.glyph(chr, x_px, advance, &fallback_metrics, allocation)
            }
        } else {
            let mut advance = pos.x_advance as f32 * scale;
            // Tabs and thin spaces remain layout concepts, as in the upstream emitter.
            if chr == '\t' {
                let tab_size = fonts
                    .face(run.font_key)
                    .map_or(4.0, |face| face.tweak().tab_size);
                let (_, space_info) = fonts.glyph_info(ctx.family, ' ', metrics);
                advance = tab_size * space_info.advance_width_unscaled.0 * scale;
            } else if matches!(chr, '\u{2009}' | '\u{202f}') {
                let thin_space = fonts
                    .face(run.font_key)
                    .map_or(0.5, |face| face.tweak().thin_space_width);
                let (_, space_info) = fonts.glyph_info(ctx.family, ' ', metrics);
                advance = thin_space * space_info.advance_width_unscaled.0 * scale;
            }
            let OutlineGlyph {
                mut allocation,
                x_px,
            } = fonts.allocate_glyph(
                run.font_key,
                metrics,
                &ShapedGlyph {
                    glyph_id,
                    h_pos: paragraph.cursor_x_px + pos.x_offset as f32 * scale,
                    is_cjk: is_cjk(chr),
                },
            );
            allocation.uv_rect.offset.y -= pos.y_offset as f32 * scale / ctx.pixels_per_point;
            paragraph.cursor_x_px += advance;
            ctx.glyph(chr, x_px, advance, metrics, allocation)
        };
        artwork.push(glyph);
    }

    let width = paragraph.cursor_x_px - origin;
    let cell = width / count as f32;
    let first = paragraph.glyphs.len();
    let mut artwork = artwork.into_iter();
    for (index, chr) in text.chars().enumerate() {
        let visual = if run.rtl { count - index - 1 } else { index };
        let visual_x = (origin + visual as f32 * cell) / ctx.pixels_per_point;
        let mut slot = if let Some(mut art) = artwork.next() {
            art.uv_rect.offset.x += art.pos.x - visual_x;
            art
        } else {
            ctx.glyph(chr, 0, cell, metrics, GlyphAllocation::default())
        };
        slot.chr = chr;
        slot.pos.x = (origin + index as f32 * cell) / ctx.pixels_per_point;
        slot.advance_width = cell / ctx.pixels_per_point;
        slot.is_rtl = run.rtl;
        slot.cluster_start = index == 0;
        paragraph.glyphs.push(slot);
    }

    // Expansions are artwork, never invented source characters. Keep every extra
    // rectangle on the first logical slot; its offset uses that slot's visual cell.
    let first_visual = if run.rtl { count - 1 } else { 0 };
    let first_visual_x = (origin + first_visual as f32 * cell) / ctx.pixels_per_point;
    let extra: Vec<_> = artwork
        .filter(|art| !art.uv_rect.is_nothing())
        .map(|mut art| {
            art.uv_rect.offset.x += art.pos.x - first_visual_x;
            art.uv_rect
        })
        .collect();
    if !extra.is_empty() {
        paragraph.glyphs[first].extra_uv_rects = Some(extra.into());
    }
}
