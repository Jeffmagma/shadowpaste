#![allow(non_snake_case)] // uppercase component function names
mod db;
mod embed;
mod monitor;
mod clipboard_view;
mod titlebar;
mod quick_paste;

use chrono::Local;
use db::{ClipboardEntry, Database};
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
use dioxus::prelude::*;
use dioxus::desktop::{Config, WindowBuilder, trayicon};
use dioxus::desktop::{use_global_shortcut, use_tray_icon_event_handler, use_tray_menu_event_handler, HotKeyState};
use embed::Embedder;
use monitor::ClipboardContent;
use std::sync::{Arc, Mutex};
use crate::clipboard_view::ClipboardView;
use crate::quick_paste::ClipboardWriteSuppression;
use crate::titlebar::Titlebar;
use crate::quick_paste::{QuickPaste, QuickPasteProps, quick_paste_config};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    let cfg = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_decorations(false)
                .with_transparent(true)
				.with_undecorated_shadow(false)
                .with_title("shadowpaste")
                .with_resizable(true)
        )
        .with_close_behaviour(dioxus::desktop::WindowCloseBehaviour::WindowHides);

    LaunchBuilder::desktop().with_cfg(cfg).launch(App);
}

/// get raw image bytes from base64 data URI like "data:image/png;base64,..."
fn decode_data_uri(data_uri: &str) -> Option<Vec<u8>> {
	let (_, b64) = data_uri.split_once(";base64,")?;
	use base64::{Engine as _, engine::general_purpose};
	general_purpose::STANDARD.decode(b64).ok()
}

/// compute embedding for clipboard content based on its type
fn compute_embedding(embedder: &mut Embedder, content: &ClipboardContent) -> Option<Vec<f32>> {
	match content {
		ClipboardContent::Text(t) => embedder.embed_document(t).ok(),
		ClipboardContent::Image(data_uri) => {
			let bytes = decode_data_uri(data_uri)?;
			embedder.embed_image_bytes(&bytes).ok()
		}
		ClipboardContent::Empty => None,
	}
}

fn App() -> Element {
	let mut history = use_signal(|| Vec::<ClipboardEntry>::new());
	let mut search_query = use_signal(|| String::new());
	let mut query_embedding: Signal<Option<Vec<f32>>> = use_signal(|| None);
	let mut loading_status = use_signal(|| "Loading embedding models...".to_string());
	let clipboard_write_suppression: Signal<ClipboardWriteSuppression> = use_signal(|| Arc::new(Mutex::new(false)));
	
	let window = dioxus::desktop::use_window();

	// initialize tray
	let _tray = use_signal(|| trayicon::init_tray_icon(trayicon::default_tray_icon(), None));

	// click icon to restore window
	let window_clone = window.clone();
	use_tray_icon_event_handler(move |event| {
		if let trayicon::TrayIconEvent::Click {
			button: trayicon::MouseButton::Left,
			button_state: trayicon::MouseButtonState::Up,
			..
		} = event {
			window_clone.set_visible(true);
			window_clone.set_focus();
		}
	});

	// only a quit option in the tray
	use_tray_menu_event_handler(move |_event| {
		std::process::exit(0);
	});

	// ctrl+shift+v opens the quick-paste popup window
	let window_for_hotkey = window.clone();
	let history_for_hotkey = history;
	let suppression_for_hotkey = clipboard_write_suppression();
	let _ = use_global_shortcut(
		"Ctrl+Shift+KeyV",
		move |state| {
			if state == HotKeyState::Pressed {
				let w = window_for_hotkey.clone();
				let mut entries = history_for_hotkey();
				entries.reverse();
				let suppression = suppression_for_hotkey.clone();
				spawn(async move {
					let dom = dioxus::core::VirtualDom::new_with_props(
						QuickPaste,
						QuickPasteProps { entries },
					)
					.with_root_context(suppression);
					let popup = w.new_window(dom, quick_paste_config()).await;
					popup.set_focus();
				});
			}
		},
	);

	// load db
	let db: Signal<Arc<Mutex<Database>>> = use_signal(|| {
		let database = Database::open().expect("Failed to open database");
		let existing = database.load_all().unwrap_or_default();
		history.write().extend(existing);
		Arc::new(Mutex::new(database))
	});

	let mut embedder: Signal<Option<Arc<Mutex<Embedder>>>> = use_signal(|| None);

	// load embedding models in the background
	use_effect(move || {
		spawn(async move {
			let result = tokio::task::spawn_blocking(|| Embedder::new()).await;

			let emb = match result {
				Ok(Ok(e)) => Arc::new(Mutex::new(e)),
				Ok(Err(e)) => {
					loading_status.set(format!("Failed to load models: {e}"));
					return;
				}
				Err(e) => {
					loading_status.set(format!("Model loading panicked: {e}"));
					return;
				}
			};

			embedder.set(Some(emb));
		});
	});

	// start clipboard listener
	use_effect(move || {
		let mut rx = monitor::start_listener();
		let db = db().clone();
		let suppression = clipboard_write_suppression();

		spawn(async move {
			while let Some(content) = rx.recv().await {
				let should_skip = if let Ok(mut suppressed) = suppression.lock() {
					if *suppressed {
						*suppressed = false;
						true
					} else {
						false
					}
				} else {
					false
				};

				if should_skip {
					continue;
				}

				let emb = if let Some(ref emb_arc) = embedder() {
					if let Ok(mut emb_guard) = emb_arc.lock() {
						compute_embedding(&mut emb_guard, &content)
					} else {
						None
					}
				} else {
					None
				};

				let mut entry = ClipboardEntry {
					id: 0,
					content,
					copied_at: Local::now(),
					embedding: emb,
				};

				if let Ok(db_guard) = db.lock() {
					if let Ok(row_id) = db_guard.insert(&entry) {
						entry.id = row_id;
					}
				}

				history.write().push(entry);
			}
		});
	});

	let delete_entry = move |id: i64| {
		if let Ok(db_guard) = db().lock() {
			let _ = db_guard.delete_by_id(id);
		}
		history.write().retain(|e| e.id != id);
	};

	// splash screen while embedding models load
	let is_ready = embedder().is_some();

	if !is_ready {
		return rsx! {
			Stylesheet { href: TAILWIND_CSS }
			div { class: "flex flex-col items-center justify-center h-screen gap-4",
				div { class: "text-4xl font-bold text-slate-700", "shadowpaste" }
				div { class: "flex items-center gap-3",
					// css spinner
					div { class: "w-5 h-5 border-2 border-slate-300 border-t-blue-500 rounded-full animate-spin" }
					span { class: "text-sm text-slate-500", "{loading_status}" }
				}
			}
		};
	}

	// item list, don't filter anything out yet (and show similarity)
	let query = search_query();
	let items: Vec<(ClipboardEntry, f32)> = {
		let hist = history();
		if query.trim().is_empty() {
			let mut v: Vec<(ClipboardEntry, f32)> = hist.iter().cloned().map(|e| (e, 0.0_f32)).collect();
			v.reverse(); // most recent first
			v
		} else {
			let query_lower = query.trim().to_lowercase();
			let q_emb_opt = query_embedding();

			// (score_for_sorting, entry, similarity)
			let mut scored: Vec<(f32, ClipboardEntry, f32)> = Vec::new();

			for e in hist.iter() {
				let text_match = match &e.content {
					ClipboardContent::Text(t) => t.to_lowercase().contains(&query_lower),
					_ => false,
				};

				let emb_sim = if let Some(ref q_emb) = q_emb_opt {
					e.embedding.as_ref().map(|emb| Embedder::similarity(q_emb, emb)).unwrap_or(0.0)
				} else {
					0.0
				};

				let is_image = matches!(e.content, ClipboardContent::Image(_));

				// it seems like image similarity is about 10x less than text similarity?? even with the same model
				let emb_sim_for_score = if is_image {
					emb_sim * 10.0
				} else {
					emb_sim
				};

				// prefer actual text matches, then similarity
				let score = if text_match {
					2.0 + emb_sim_for_score
				} else {
					emb_sim_for_score
				};

				scored.push((score, e.clone(), emb_sim));
			}

			scored.sort_by(|a, b| {
				b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
			});

			scored.into_iter().map(|(_, entry, sim)| (entry, sim)).collect()
		}
	};

	// embed query text when it changes
	if !query.trim().is_empty() && query_embedding().is_none() {
		if let Some(ref emb_arc) = embedder() {
			if let Ok(mut emb_guard) = emb_arc.lock() {
				if let Ok(emb) = emb_guard.embed_query(query.trim()) {
					query_embedding.set(Some(emb));
				}
			}
		}
	}

	let query_for_view = query.clone();

	rsx! {
		Stylesheet { href: TAILWIND_CSS }
		style { "
			::-webkit-scrollbar {{ width: 8px; }}
			::-webkit-scrollbar-track {{ background: transparent; }}
			::-webkit-scrollbar-thumb {{ background: #334155; border-radius: 4px; }}
			::-webkit-scrollbar-thumb:hover {{ background: #475569; }}
		" }

		div { class: "h-screen w-screen bg-slate-950 text-slate-200 flex flex-col font-sans overflow-hidden rounded-xl border border-slate-800 shadow-2xl",
			Titlebar {}

			div { class: "flex-1 flex flex-col p-4 gap-4 overflow-hidden",
                // search bar
				div { class: "relative group shrink-0",
					div { class: "absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none",
						// search icon
						svg { class: "h-4 w-4 text-slate-500 group-focus-within:text-blue-400 transition-colors", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
							path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" }
						}
					}
					input {
						class: "w-full pl-10 pr-4 py-2.5 bg-slate-900 border border-slate-800 rounded-lg text-sm text-slate-200 placeholder-slate-500
								focus:outline-none focus:ring-1 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all shadow-sm",
						placeholder: "Type to search clipboard history...",
						value: "{search_query}",
						oninput: move |e| {
							search_query.set(e.value());
							query_embedding.set(None); // reset so the query re-embeds
						},
						// auto-focus on start
						onmounted: move |evt| { spawn(async move { let _ = evt.set_focus(true).await; }); }
					}
				}

                // results
				div { class: "flex-1 overflow-y-auto pr-1 space-y-2",
					if items.is_empty() {
						div { class: "flex flex-col items-center justify-center h-full text-slate-600 gap-2",
							div { class: "text-4xl opacity-20", "📋" }
							p { class: "text-sm", "No clipboard history found" }
						}
					}
					for (entry, sim) in items.iter() {
						div { key: "{entry.id}", class: "group/item",
							ClipboardView {
								entry: entry.clone(),
								on_delete: delete_entry,
								search_query: query_for_view.clone(),
								similarity: *sim,
							}
						}
					}
				}
			}
		}
	}
}