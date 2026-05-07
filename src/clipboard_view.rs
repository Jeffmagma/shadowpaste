use chrono::Local;
use dioxus::prelude::*;

use crate::db::ClipboardEntry;
use crate::monitor::ClipboardContent;

/// highlight text match fragments
fn highlight_fragments(text: &str, query: &str) -> Vec<(String, bool)> {
	if query.is_empty() {
		return vec![(text.to_string(), false)];
	}
	let lower_text = text.to_lowercase();
	let lower_query = query.to_lowercase();
	let mut result = Vec::new();
	let mut last = 0;
	for (start, _) in lower_text.match_indices(&lower_query) {
		if start > last {
			result.push((text[last..start].to_string(), false));
		}
		result.push((text[start..start + query.len()].to_string(), true));
		last = start + query.len();
	}
	if last < text.len() {
		result.push((text[last..].to_string(), false));
	}
	if result.is_empty() {
		result.push((text.to_string(), false));
	}
	result
}

/// display a single clipboard entry
#[component]
pub fn ClipboardView(
	entry: ClipboardEntry,
	on_delete: EventHandler<i64>,
	on_context_menu: EventHandler<(i64, f64, f64)>,
	search_query: String,
	similarity: f32,
) -> Element {
	let time_str = entry.copied_at.with_timezone(&Local).format("%b %d %Y, %I:%M %p").to_string(); // jan 1, 2021, 12:00 PM
	let entry_id = entry.id;
	let header = if search_query.is_empty() {
		time_str.clone()
	} else {
		format!("{time_str} — sim: {similarity:.3}")
	};
	rsx! {
		div { class: "flex items-start gap-3 p-3 rounded-lg border border-slate-800 bg-slate-900/50 hover:bg-slate-800 hover:border-slate-700 transition-all group relative",
			oncontextmenu: move |evt| {
				evt.prevent_default();
				let coords = evt.client_coordinates();
				on_context_menu.call((entry_id, coords.x, coords.y));
			},
			div { class: "flex-1 flex flex-col gap-1.5 min-w-0",
				span { class: "text-[10px] uppercase font-semibold tracking-wider text-slate-500", "{header}" }
				match entry.content {
					ClipboardContent::Text(ref text) => {
						let fragments = highlight_fragments(text, &search_query);
						rsx! {
							p { class: "text-sm text-slate-300 line-clamp-4 font-mono break-all leading-relaxed",
								for (i, (frag, is_match)) in fragments.iter().enumerate() {
									if *is_match {
										span {
											key: "{i}",
											class: "bg-yellow-500/20 text-yellow-200 rounded px-0.5 font-medium",
											"{frag}"
										}
									} else {
										span { key: "{i}", "{frag}" }
									}
								}
							}
						}
					},
					ClipboardContent::Image(ref src) => rsx! {
						div { class: "rounded-md overflow-hidden bg-slate-950 border border-slate-800",
							img { src: "{src}", class: "max-w-full max-h-64 h-auto object-contain" }
						}
					},
					ClipboardContent::Empty => rsx! {
						p { class: "text-xs text-slate-500 italic", "Empty Clipboard" }
					}
				}
			}
			button {
				class: "opacity-0 group-hover:opacity-100 transition-opacity text-slate-500 hover:text-red-400 p-1.5 rounded-md hover:bg-slate-700/50 cursor-pointer absolute top-2 right-2",
				title: "Delete entry",
				onclick: move |_| on_delete(entry_id),
				svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24", stroke_width: "2",
					path { d: "M6 18L18 6M6 6l12 12", stroke_linecap: "round", stroke_linejoin: "round" }
				}
			}
		}
	}
}

