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
pub fn ClipboardView(entry: ClipboardEntry, on_delete: EventHandler<i64>, search_query: String, similarity: f32) -> Element {
	let time_str = entry.copied_at.with_timezone(&Local).format("%b %d %Y, %I:%M %p").to_string(); // jan 1, 2021, 12:00 PM
	let entry_id = entry.id;
	let header = if search_query.is_empty() {
		time_str.clone()
	} else {
		format!("{time_str} — sim: {similarity:.3}")
	};

	rsx! {
        div { class: "flex items-start gap-3 group",
            div { class: "flex-1 flex flex-col gap-1 min-w-0",
                span { class: "text-xs text-slate-400", "{header}" }
                match entry.content {
                    ClipboardContent::Text(ref text) => {
                        let fragments = highlight_fragments(text, &search_query);
                        rsx! {
                            p { class: "text-sm line-clamp-3 font-mono break-all",
                                for (i, (frag, is_match)) in fragments.iter().enumerate() {
                                    if *is_match {
                                        mark {
                                            key: "{i}",
                                            class: "bg-yellow-200 rounded px-0.5",
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
                        img { src: "{src}", class: "max-w-full max-h-48 h-auto rounded object-contain" }
                    },
                    ClipboardContent::Empty => rsx! {
                        p { class: "text-xs text-slate-400", "Empty Clipboard" }
                    }
                }
            }
            button {
                class: "opacity-0 group-hover:opacity-100 transition-opacity text-slate-400 hover:text-red-500 p-1 shrink-0 cursor-pointer",
                title: "Delete entry",
                onclick: move |_| on_delete(entry_id),
                "✕"
            }
        }
    }
}

