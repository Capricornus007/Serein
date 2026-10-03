//! Keep source indices logical; only shaped glyph positions follow visual order.
use super::*;

/// Apply L1 to scalar levels for this line without cloning the whole text's byte levels.
/// Adapted from unicode-bidi 0.3.18's `reorder_levels` (MIT/Apache-2.0), copyright
/// 2015 The Servo Project Developers; the unchanged dependency licenses are bundled.
fn line_levels(
    bidi: &unicode_bidi::BidiInfo<'_>,
    range: Range<usize>,
    base: unicode_bidi::Level,
) -> Vec<unicode_bidi::Level> {
    use unicode_bidi::BidiClass::{B, BN, FSI, LRE, LRI, LRO, PDF, PDI, RLE, RLI, RLO, S, WS};
    let mut levels: Vec<_> = bidi.text[range.clone()]
        .char_indices()
        .map(|(i, _)| bidi.levels[range.start + i])
        .collect();
    let mut reset_from = Some(0);
    let mut previous = base;
    for (scalar, (byte, _)) in bidi.text[range.clone()].char_indices().enumerate() {
        match bidi.original_classes[range.start + byte] {
            B | S => {
                let from = reset_from.unwrap_or(scalar);
                levels[from..=scalar].fill(base);
                reset_from = None;
            }
            WS | FSI | LRI | RLI | PDI => {
                reset_from.get_or_insert(scalar);
            }
            RLE | LRE | RLO | LRO | PDF | BN => {
                reset_from.get_or_insert(scalar);
                levels[scalar] = previous;
            }
            _ => reset_from = None,
        }
        previous = levels[scalar];
    }
    if let Some(from) = reset_from {
        levels[from..].fill(base);
    }
    levels
}

pub(super) fn reorder_rows(bidi: &unicode_bidi::BidiInfo<'_>, rows: &mut [PlacedRow]) {
    let mut offset = 0;
    let mut paragraph_index = 0;
    for placed in rows {
        let row = Arc::make_mut(&mut placed.row);
        let end = offset
            + bidi.text[offset..]
                .char_indices()
                .nth(row.glyphs.len())
                .map_or(bidi.text.len() - offset, |(i, _)| i);
        while paragraph_index + 1 < bidi.paragraphs.len()
            && bidi.paragraphs[paragraph_index].range.end <= offset
        {
            paragraph_index += 1;
        }
        row.paragraph_rtl = bidi
            .paragraphs
            .get(paragraph_index)
            .is_some_and(|p| p.level.is_rtl());
        let mut column = 0;
        let mut index = paragraph_index;
        while let Some(paragraph) = bidi.paragraphs.get(index).filter(|p| p.range.start < end) {
            let from = offset.max(paragraph.range.start);
            let to = end.min(paragraph.range.end);
            let levels = line_levels(bidi, from..to, paragraph.level);
            let glyphs = &mut row.glyphs[column..column + levels.len()];
            for (glyph, level) in glyphs.iter_mut().zip(&levels) {
                glyph.is_rtl = level.is_rtl();
            }
            if levels.iter().any(unicode_bidi::Level::is_rtl) {
                let widths: Vec<_> = glyphs
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        glyphs
                            .get(i + 1)
                            .map_or(g.advance_width, |next| next.pos.x - g.pos.x)
                    })
                    .collect();
                let mut x = glyphs.first().map_or(0.0, |g| g.pos.x);
                for i in unicode_bidi::BidiInfo::reorder_visual(&levels) {
                    glyphs[i].pos.x = x;
                    x += widths[i];
                }
            }
            column += levels.len();
            index += 1;
        }
        offset = end + usize::from(placed.ends_with_newline);
    }
}

pub(super) fn visual_glyphs(row: &Row) -> impl Iterator<Item = &Glyph> {
    let sorted = row.glyphs.iter().any(|g| g.is_rtl).then(|| {
        let mut glyphs: Vec<_> = row.glyphs.iter().collect();
        glyphs.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
        glyphs
    });
    let mut sorted = sorted.map(Vec::into_iter);
    let mut logical = row.glyphs.iter();
    core::iter::from_fn(move || match &mut sorted {
        Some(sorted) => sorted.next(),
        None => logical.next(),
    })
}

/// Preserve the original final paragraph base when elision hides its first strong scalar.
pub(super) fn for_text(
    text: &str,
    final_base: Option<unicode_bidi::Level>,
) -> Option<unicode_bidi::BidiInfo<'_>> {
    if text.is_ascii() && final_base.is_none() {
        return None;
    }
    let mut bidi = unicode_bidi::BidiInfo::new(text, None);
    if let Some(base) = final_base
        && let Some(paragraph) = bidi.paragraphs.last_mut()
    {
        let resolved =
            unicode_bidi::ParagraphBidiInfo::new(&text[paragraph.range.clone()], Some(base));
        bidi.levels[paragraph.range.clone()].copy_from_slice(&resolved.levels);
        paragraph.level = resolved.paragraph_level;
    }
    Some(bidi)
}

pub(super) fn requires_layout(bidi: &unicode_bidi::BidiInfo<'_>) -> bool {
    bidi.has_rtl()
        || bidi
            .paragraphs
            .iter()
            .any(|paragraph| paragraph.level.is_rtl())
}
