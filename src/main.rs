#![allow(non_snake_case)] // uppercase component function names
mod db;
mod embed;
mod monitor;
mod clipboard_view;

use chrono::Local;
use db::{ClipboardEntry, Database};
use dioxus::prelude::*;
use embed::Embedder;
use monitor::ClipboardContent;
use std::sync::{Arc, Mutex};
use crate::clipboard_view::ClipboardView;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
	launch(App);
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

		spawn(async move {
			while let Some(content) = rx.recv().await {
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

		div { class: "p-6 font-sans max-w-3xl mx-auto",
			h2 { class: "text-2xl font-bold mb-4", "shadowpaste" }

			input {
				class: "w-full mb-4 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-400",
				placeholder: "Search clipboard history...",
				value: "{search_query}",
				oninput: move |e| {
					search_query.set(e.value());
					query_embedding.set(None); // reset so the query re-embeds
				},
			}

			for (entry, sim) in items.iter() {
				div { key: "{entry.id}", class: "py-3 border-b border-gray-100 last:border-0",
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