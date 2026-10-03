//! Synthetic native text layout and editing regressions; no service or device adapters.
use crate::{avatars::Avatars, composer_text, fonts};
use egui::{
	Context, Event, FontId, Key, Modifiers, text::CCursor, text::CCursorRange, text::LayoutJob,
};

const SAMPLES: [&str; 4] = [
	"این یک پیام آزمایشی برای بررسی ترتیب واژه‌ها در Serein است. آغاز جمله باید در خط اول بماند و ادامهٔ آن در خط‌های بعدی قرار بگیرد.",
	"هذه رسالة تجريبية للتحقق من ترتيب الكلمات في Serein. يجب أن تبقى بداية الجملة في السطر الأول ثم تظهر بقية الكلمات بالترتيب الصحيح.",
	"یہ ایک آزمائشی پیغام ہے جس سے Serein میں الفاظ کی ترتیب دیکھی جا سکتی ہے۔ جملے کا آغاز پہلی سطر میں رہنا چاہیے۔",
	"ئەمە پەیامێکی تاقیکردنەوەیە بۆ پشکنینی ڕیزبەندی وشەکان لە Serein. دەستپێکی ڕستەکە دەبێت لە دێڕی یەکەم بمێنێتەوە.",
];

fn context() -> Context {
	let ctx = Context::default();
	fonts::install(&ctx);
	crate::design::apply(&ctx);
	ctx
}

fn logical_rows(galley: &egui::Galley) -> String {
	galley
		.rows
		.iter()
		.flat_map(|row| {
			row.glyphs
				.iter()
				.map(|glyph| glyph.chr)
				.chain(row.ends_with_newline.then_some('\n'))
		})
		.collect()
}

/// Compare actual left-to-right glyph positions with Unicode's per-line visual ordering.
fn assert_visual_order(galley: &egui::Galley, source: &str) {
	let bidi = unicode_bidi::BidiInfo::new(source, None);
	let mut byte = 0;
	for row in &galley.rows {
		let text: String = row.glyphs.iter().map(|glyph| glyph.chr).collect();
		let end = byte + text.len();
		let mut expected = vec![];
		let mut column = 0;
		for paragraph in bidi
			.paragraphs
			.iter()
			.filter(|p| p.range.start < end && byte < p.range.end)
		{
			let from = byte.max(paragraph.range.start);
			let to = end.min(paragraph.range.end);
			let levels = bidi.reordered_levels(paragraph, from..to);
			let scalar_levels: Vec<_> = source[from..to]
				.char_indices()
				.map(|(offset, _)| levels[from + offset])
				.collect();
			for index in unicode_bidi::BidiInfo::reorder_visual(&scalar_levels) {
				if row.glyphs[column + index].advance_width > 0.01 {
					expected.push(column + index);
				}
			}
			column += scalar_levels.len();
		}
		let mut actual: Vec<_> = row
			.glyphs
			.iter()
			.enumerate()
			.filter(|(_, glyph)| glyph.advance_width > 0.01)
			.collect();
		actual.sort_by(|a, b| a.1.pos.x.total_cmp(&b.1.pos.x));
		assert_eq!(
			actual.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
			expected,
			"visual order for row {text:?}"
		);
		assert!(
			row.glyphs
				.iter()
				.all(|glyph| glyph.pos.x.is_finite() && glyph.advance_width >= 0.0)
		);
		assert!(row.visuals.mesh.is_valid());
		byte = end + usize::from(row.ends_with_newline);
	}
}

#[test]
fn rtl_wrapping_preserves_logical_lines_in_persian_arabic_urdu_and_kurdish() {
	let ctx = context();
	let mut layout = composer_text::Layout::default();
	let mut avatars = Avatars::default();
	ctx.run_ui(Default::default(), |ui| {
		for source in SAMPLES {
			for width in [120.0, 330.0, 900.0] {
				let galley =
					layout.galley(ui, source, width, &[], &[], &[], false, &mut avatars, true);
				assert_eq!(galley.job.text, source);
				assert_eq!(
					logical_rows(&galley),
					source,
					"wrapped rows must retain the sentence's beginning first"
				);
				assert_eq!(galley.end().index.0, source.chars().count());
				assert_visual_order(&galley, source);
				if width < 900.0 {
					assert!(galley.rows.len() > 1);
				}
			}
		}
	})
	.drop_without_applying_deltas();
	assert!(avatars.take_requests().is_empty());
}

#[test]
fn rtl_plain_labels_and_explicit_newlines_keep_logical_text_and_visual_direction() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		for source in [
			"گفت‌وگوی فارسی",
			"المحادثة العربية",
			"اردو گفتگو",
			"گفتوگۆی کوردی",
			"مَرْحَبًا لا لأ\nEnglish 123\nاین یک پیام است",
			"123 مرحبا English 45",
			"English \u{2067}مرحبا 123\u{2069} end",
			"مرحبا\nالعربية",
		] {
			for single_line in [false, true] {
				let job = if single_line {
					LayoutJob::simple_singleline(
						source.into(),
						FontId::proportional(15.0),
						ui.visuals().text_color(),
					)
				} else {
					LayoutJob::simple(
						source.into(),
						FontId::proportional(15.0),
						ui.visuals().text_color(),
						100.0,
					)
				};
				let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
				assert_eq!(logical_rows(&galley), source);
				assert_visual_order(&galley, source);
			}
		}
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_combining_clusters_do_not_split_across_automatic_lines() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		for source in ["بَ", "لأ", "مَرْحَبًا", "ب\u{200d}ب", "ب\u{200c}ب"] {
			let galley = ui.fonts_mut(|fonts| {
				fonts.layout_job(LayoutJob::simple(
					source.into(),
					FontId::proportional(15.0),
					ui.visuals().text_color(),
					1.0,
				))
			});
			assert_eq!(logical_rows(&galley), source);
			assert!(galley.rows.iter().all(|row| {
				row.glyphs
					.first()
					.is_none_or(|g| !matches!(g.chr, '\u{064e}' | '\u{0652}' | '\u{064b}'))
			}));
			if source == "بَ" {
				assert_eq!(
					galley.rows.len(),
					1,
					"{source:?}: {:?}",
					galley
						.rows
						.iter()
						.map(|r| r.glyphs.iter().map(|g| g.chr).collect::<String>())
						.collect::<Vec<_>>()
				);
			}
		}
	})
	.drop_without_applying_deltas();
}

fn key(key: Key) -> Event {
	Event::Key {
		key,
		physical_key: None,
		pressed: true,
		repeat: false,
		modifiers: Modifiers::NONE,
	}
}

struct Editor {
	ctx: Context,
	id: egui::Id,
	text: String,
	layout: composer_text::Layout,
	avatars: Avatars,
	time: f64,
}

impl Editor {
	fn new(source: &str) -> Self {
		Self {
			ctx: context(),
			id: egui::Id::unique("rtl-editor"),
			text: source.into(),
			layout: Default::default(),
			avatars: Default::default(),
			time: 0.0,
		}
	}

	fn frame(
		&mut self,
		events: Vec<Event>,
		selection: Option<CCursorRange>,
	) -> (CCursorRange, Vec<egui::OutputCommand>) {
		self.ctx.memory_mut(|memory| memory.request_focus(self.id));
		if let Some(range) = selection {
			let mut state =
				egui::text_edit::TextEditState::load(&self.ctx, self.id).unwrap_or_default();
			state.cursor.set_char_range(Some(range));
			state.store(&self.ctx, self.id);
		}
		let mut result = None;
		let ctx = self.ctx.clone();
		let output = ctx.run_ui(
			egui::RawInput {
				time: Some(self.time),
				events,
				..Default::default()
			},
			|ui| {
				let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, width| {
					self.layout.galley(
						ui,
						buffer.as_str(),
						width,
						&[],
						&[],
						&[],
						false,
						&mut self.avatars,
						true,
					)
				};
				let edit = egui::TextEdit::multiline(&mut self.text)
					.id(self.id)
					.desired_width(140.0)
					.layouter(&mut layouter)
					.show(ui);
				result = edit.cursor_range;
				self.layout.paint(ui, &edit);
			},
		);
		self.time += 0.25;
		let commands = output.platform_output.commands.clone();
		output.drop_without_applying_deltas();
		(
			result.expect("focused editor must retain a cursor"),
			commands,
		)
	}
}

#[test]
fn rtl_native_arrows_and_selection_collapse_follow_visual_edges() {
	let mut editor = Editor::new("مرحبا");
	editor.frame(vec![], Some(CCursorRange::one(CCursor::new(0))));
	let (range, _) = editor.frame(vec![key(Key::ArrowLeft)], None);
	assert_eq!(range.primary.index.0, 1);
	let (range, _) = editor.frame(vec![key(Key::ArrowRight)], None);
	assert_eq!(range.primary.index.0, 0);
	let selection = CCursorRange::two(CCursor::new(1), CCursor::new(4));
	let (range, _) = editor.frame(vec![key(Key::ArrowLeft)], Some(selection));
	assert_eq!(
		range.primary.index.0, 4,
		"Left collapses to the left visual endpoint"
	);
	let (range, _) = editor.frame(vec![key(Key::ArrowRight)], Some(selection));
	assert_eq!(
		range.primary.index.0, 1,
		"Right collapses to the right visual endpoint"
	);
}

#[test]
fn rtl_native_copy_paste_delete_and_ime_preserve_logical_buffer_order() {
	let mut editor = Editor::new("مرحبا English 123");
	let end = editor.text.chars().count();
	editor.frame(
		vec![],
		Some(CCursorRange::two(CCursor::new(0), CCursor::new(end))),
	);
	let (_, commands) = editor.frame(vec![Event::Copy], None);
	assert!(commands.iter().any(
		|command| matches!(command, egui::OutputCommand::CopyText(text) if text == &editor.text)
	));
	editor.frame(vec![Event::Paste("سلام دنیا".into())], None);
	assert_eq!(editor.text, "سلام دنیا");
	editor.frame(
		vec![key(Key::Backspace)],
		Some(CCursorRange::one(CCursor::new(4))),
	);
	assert_eq!(editor.text, "سلا دنیا");
	editor.frame(
		vec![Event::Ime(egui::ImeEvent::Preedit {
			text: "ب".into(),
			active_range_chars: None,
		})],
		Some(CCursorRange::one(CCursor::new(3))),
	);
	editor.frame(vec![Event::Ime(egui::ImeEvent::Commit("م".into()))], None);
	assert_eq!(editor.text, "سلام دنیا");
}

#[test]
fn rtl_formatted_messages_wrap_after_shaping_with_styles_and_emoji() {
	let ctx = context();
	crate::emoji::install(&ctx).unwrap();
	for source in SAMPLES {
		for width in [140.0, 420.0] {
			let parsed = crate::markdown::Formatted::parse(&format!("**{source}** 😀"));
			let output = ctx.run_ui(Default::default(), |ui| {
				ui.set_max_width(width);
				parsed.show(ui, &mut None);
			});
			let galley = output
				.shapes
				.iter()
				.find_map(|shape| match &shape.shape {
					egui::Shape::Text(text) if text.galley.text().starts_with(source) => {
						Some(&text.galley)
					}
					_ => None,
				})
				.expect("actual painted message galley");
			assert!(galley.rows.len() > 1);
			assert_eq!(logical_rows(galley), format!("{source} 😀"));
			assert_visual_order(galley, &format!("{source} 😀"));
			output.drop_without_applying_deltas();
		}
	}
}

#[test]
fn rtl_controls_expansions_alignment_and_truncation_do_not_corrupt_source_indices() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		for source in [
			"\u{202d}بَ\u{202c}\nمرحبا",
			"ۀ\nسلام",
			"\u{202b}  \u{202c}\nمرحبا",
			"مرحبا 123 456 789 English نهاية   ",
			"مرحبا\tEnglish\n\nسلام\n",
			"English fi ffi e\u{301} 👩\u{200d}💻 end",
			"\u{2067}مرحبا 123\u{2069} English \u{202b}سلام\u{202c}",
		] {
			for align in [egui::Align::LEFT, egui::Align::Center, egui::Align::RIGHT] {
				let mut job = LayoutJob::simple(
					source.into(),
					FontId::proportional(15.0),
					ui.visuals().text_color(),
					65.0,
				);
				job.halign = align;
				let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
				assert_eq!(logical_rows(&galley), source);
				assert_visual_order(&galley, source);
				if source.starts_with("\u{202b}  ") {
					assert!(galley.rows[0].glyphs.iter().all(|g| !g.is_rtl));
				}
			}
		}
		for width in [1.0, 30.0, 90.0] {
			let mut job = LayoutJob::simple(
				SAMPLES[0].into(),
				FontId::proportional(15.0),
				ui.visuals().text_color(),
				width,
			);
			job.wrap.max_rows = 1;
			let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
			assert!(galley.elided);
			assert_eq!(galley.rows.len(), 1);
			assert_eq!(galley.rows[0].glyphs.last().unwrap().chr, '…');
			assert!(galley.rows[0].visuals.mesh.is_valid());
			assert!(galley.rows[0].glyphs.iter().all(|g| g.pos.x.is_finite()));
		}
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_composer_inline_objects_keep_wire_indices_and_caret_geometry() {
	let ctx = context();
	crate::emoji::install(&ctx).unwrap();
	let source = "مرحبا 😀 👩🏽\u{200d}💻 <@1> <#2> <:wave:9001> English نهاية";
	let mut layout = composer_text::Layout::default();
	let mut avatars = Avatars::default();
	ctx.run_ui(Default::default(), |ui| {
		for width in [70.0, 260.0, 1000.0] {
			let galley = layout.galley(ui, source, width, &[], &[], &[], false, &mut avatars, true);
			assert_eq!(galley.job.text, source);
			assert_eq!(logical_rows(&galley), source);
			assert_eq!(galley.end().index.0, source.chars().count());
			for index in 0..=source.chars().count() {
				let cursor = CCursor::new(index);
				assert_eq!(galley.clamp_cursor(&cursor).index.0, index);
				assert!(galley.pos_from_cursor(cursor).is_finite());
			}
			let mut offset = 0;
			for row in &galley.rows {
				let count = row.glyphs.len();
				for markup in ["👩🏽\u{200d}💻", "<@1>", "<#2>", "<:wave:9001>"] {
					let start = source[..source.find(markup).unwrap()].chars().count();
					if offset <= start && start < offset + count {
						assert!(
							start + markup.chars().count() <= offset + count,
							"inline split at row boundary"
						);
					}
				}
				offset += count + usize::from(row.ends_with_newline);
			}
		}
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_wrapped_carets_and_digit_only_rows_follow_paragraph_reading_order() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		let source = "مرحبا\n123\nسلام";
		let galley = ui.fonts_mut(|fonts| {
			fonts.layout_job(LayoutJob::simple(
				source.into(),
				FontId::proportional(15.0),
				ui.visuals().text_color(),
				500.0,
			))
		});
		for index in 0..5 {
			let cursor = CCursor::new(index);
			let next = galley.cursor_left_one_character(&cursor);
			assert_eq!(next.index.0, index + 1);
		}
		let mut cursor = CCursor::new(4);
		cursor.prefer_next_row = true;
		assert_eq!(galley.cursor_left_one_character(&cursor).index.0, 5);
		let cursor = CCursor::new(6);
		assert_eq!(
			galley.cursor_right_one_character(&cursor).index.0,
			7,
			"a separate numeric paragraph reads left to right"
		);
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_multiline_selection_collapses_in_paragraph_reading_order() {
	let mut editor = Editor::new(SAMPLES[1]);
	let end = editor.text.chars().count() - 1;
	let selection = CCursorRange::two(CCursor::new(1), CCursor::new(end));
	editor.frame(vec![], Some(selection));
	let (range, _) = editor.frame(vec![key(Key::ArrowLeft)], Some(selection));
	assert_eq!(range.primary.index.0, end);
	let (range, _) = editor.frame(vec![key(Key::ArrowRight)], Some(selection));
	assert_eq!(range.primary.index.0, 1);
}

#[test]
fn rtl_elision_orders_the_visible_neutral_ellipsis_and_removes_complete_mark_clusters() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		for source in [
			"abc مَرْحَبًا سلام نهاية",
			"مَرْحَبًا سلام English",
			"first\nabc مَرْحَبًا سلام",
		] {
			for width in [20.0, 50.0, 90.0] {
				let mut job = LayoutJob::simple(
					source.into(),
					FontId::proportional(15.0),
					ui.visuals().text_color(),
					width,
				);
				job.wrap.max_rows = if source.contains('\n') { 2 } else { 1 };
				let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
				assert!(galley.elided);
				let visible = logical_rows(&galley);
				assert!(visible.ends_with('…'));
				assert_visual_order(&galley, &visible);
				let retained = visible.trim_end_matches('…');
				let hidden = &source[retained.len()..];
				assert!(
					!hidden.starts_with(['\u{064e}', '\u{0652}', '\u{064b}']),
					"elision split a base/mark cluster: {visible:?}"
				);
			}
		}
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_mixed_selection_paints_disjoint_visual_spans_without_selecting_the_gap() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		let mut galley = ui.fonts_mut(|fonts| {
			fonts.layout_job(LayoutJob::simple_singleline(
				"abc مرحبا xyz".into(),
				FontId::proportional(15.0),
				ui.visuals().text_color(),
			))
		});
		let mut rectangles = Vec::new();
		egui::text_selection::visuals::paint_text_selection(
			&mut galley,
			ui.visuals(),
			&CCursorRange::two(CCursor::new(2), CCursor::new(6)),
			Some(&mut rectangles),
		);
		assert_eq!(rectangles.len(), 2);
		let row = &galley.rows[0];
		let rects: Vec<_> = rectangles
			.iter()
			.map(|rectangle| {
				rectangle
					.vertex_indices
					.iter()
					.fold(egui::Rect::NOTHING, |rect, index| {
						rect.union(egui::Rect::from_min_max(
							row.visuals.mesh.vertices[*index as usize].pos,
							row.visuals.mesh.vertices[*index as usize].pos,
						))
					})
			})
			.collect();
		for (index, glyph) in row.glyphs.iter().enumerate() {
			let center = egui::pos2(glyph.pos.x + glyph.advance_width / 2.0, row.height() / 2.0);
			assert_eq!(
				rects.iter().any(|rect| rect.contains(center)),
				(2..6).contains(&index),
				"selection includes wrong logical glyph {index}"
			);
		}
		assert!(row.visuals.mesh.is_valid());
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_elision_reshapes_brackets_when_visible_context_changes_their_direction() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		let source = "مرحبا abc [defghi] more words";
		let mut checked = false;
		for width in (40..200).step_by(3) {
			let mut job = LayoutJob::simple(
				source.into(),
				FontId::proportional(15.0),
				ui.visuals().text_color(),
				width as f32,
			);
			job.wrap.max_rows = 1;
			job.wrap.break_anywhere = true;
			let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
			let visible = logical_rows(&galley);
			if !visible.contains('[') || visible.contains(']') {
				continue;
			}
			let expected = ui.fonts_mut(|fonts| {
				fonts.layout_job(LayoutJob::simple_singleline(
					visible,
					FontId::proportional(15.0),
					ui.visuals().text_color(),
				))
			});
			let actual = galley.rows[0].glyphs.iter().find(|g| g.chr == '[').unwrap();
			let expected = expected.rows[0]
				.glyphs
				.iter()
				.find(|g| g.chr == '[')
				.unwrap();
			assert_eq!(
				actual.uv_rect.min, expected.uv_rect.min,
				"elision retained artwork shaped in the hidden bracket context"
			);
			assert_eq!(actual.uv_rect.max, expected.uv_rect.max);
			checked = true;
		}
		assert!(
			checked,
			"fixture must expose an unmatched bracket after elision"
		);
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_elision_retains_original_styles_and_base_direction() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		let mut job = LayoutJob::default();
		job.wrap.max_width = 85.0;
		job.wrap.max_rows = 1;
		job.wrap.break_anywhere = true;
		let format = |color| egui::text::TextFormat::simple(FontId::proportional(15.0), color);
		job.append("مرحبا abc ", 0.0, format(egui::Color32::RED));
		job.append("[defghi]", 0.0, format(egui::Color32::GREEN));
		job.append(" نهاية tail", 0.0, format(egui::Color32::BLUE));
		let original = job.clone();
		let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
		assert_eq!(*galley.job, original);
		assert_eq!(logical_rows(&galley), "مرحبا abc […");
		let row = &galley.rows[0];
		for (chr, color) in [
			('م', egui::Color32::RED),
			('[', egui::Color32::GREEN),
			('…', egui::Color32::GREEN),
		] {
			let glyph = row.glyphs.iter().find(|g| g.chr == chr).unwrap();
			assert_eq!(
				row.visuals.mesh.vertices[glyph.first_vertex as usize].color,
				color
			);
		}
		let mut job = original;
		job.wrap.max_width = 1.0;
		let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
		assert_eq!(logical_rows(&galley), "…");
		assert!(
			galley.rows[0].is_rtl(),
			"hidden first strong character still sets the paragraph base"
		);
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_digit_only_soft_rows_continue_at_the_next_visual_line_edge() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		let galley = ui.fonts_mut(|fonts| {
			fonts.layout_job(LayoutJob::simple(
				"مرحبا123456789012345678901234567890مرحبا".into(),
				FontId::proportional(15.0),
				ui.visuals().text_color(),
				42.0,
			))
		});
		let mut offset = 0;
		let mut checked = false;
		for (index, row) in galley.rows.iter().enumerate() {
			if !row.glyphs.is_empty()
				&& row.glyphs.iter().all(|g| g.chr.is_ascii_digit())
				&& index + 1 < galley.rows.len()
			{
				assert!(row.is_rtl());
				let mut cursor = CCursor::new(offset);
				cursor.prefer_next_row = true;
				let next = galley.cursor_left_one_character(&cursor);
				assert_eq!(
					galley.layout_from_cursor(next).row,
					index + 1,
					"Left at the visual edge must continue down the RTL paragraph"
				);
				checked = true;
			}
			offset += row.glyphs.len();
		}
		assert!(checked);
	})
	.drop_without_applying_deltas();
}

#[test]
fn rtl_elision_preserves_leading_indent_and_a_numeric_paragraph_base() {
	let ctx = context();
	ctx.run_ui(Default::default(), |ui| {
		let mut job = LayoutJob::default();
		job.wrap.max_width = 85.0;
		job.wrap.max_rows = 1;
		job.wrap.break_anywhere = true;
		let format =
			egui::text::TextFormat::simple(FontId::proportional(15.0), ui.visuals().text_color());
		job.append("", 25.0, format.clone());
		job.append("مرحبا abc [defghi] more words", 0.0, format);
		let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
		assert!(galley.elided);
		assert!(galley.rows[0].glyphs.len() > 1);
		assert!(
			galley.rows[0].glyphs.iter().all(|g| g.pos.x >= 24.99),
			"truncation erased an empty leading section's indent"
		);
		let mut job = LayoutJob::simple(
			"12345678901234567890 مرحبا".into(),
			FontId::proportional(15.0),
			ui.visuals().text_color(),
			45.0,
		);
		job.wrap.max_rows = 1;
		job.wrap.break_anywhere = true;
		job.wrap.overflow_character = None;
		let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
		assert!(galley.elided);
		assert!(
			galley.rows[0]
				.glyphs
				.iter()
				.all(|g| g.chr.is_ascii_digit() && !g.is_rtl)
		);
		assert!(
			galley.rows[0].is_rtl(),
			"the hidden Arabic first strong character still sets the paragraph base"
		);
	})
	.drop_without_applying_deltas();
}
