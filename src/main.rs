#![allow(non_snake_case)] // uppercase component function names
mod db;
mod monitor;

use chrono::Local;
use db::{ClipboardEntry, Database};
use dioxus::prelude::*;
use monitor::ClipboardContent;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
	launch(App);
}

#[component]
fn ClipboardView(entry: ClipboardEntry, on_delete: EventHandler<i64>) -> Element {
	let time_str = entry.copied_at.format("%b %d, %I:%M %p").to_string();
	let entry_id = entry.id;

	rsx! {
		div { class: "flex items-start gap-3 group",
			div { class: "flex-1 flex flex-col gap-1 min-w-0",
				span { class: "text-xs text-slate-400", "{time_str}" }
				match entry.content {
					ClipboardContent::Text(ref text) => rsx! {
						p { class: "text-sm line-clamp-3 font-mono break-all", "{text}" }
					},
					ClipboardContent::Image(ref src) => rsx! {
						img { src: "{src}", class: "max-w-full max-h-48 h-auto rounded" }
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

fn App() -> Element {
	let mut history = use_signal(|| Vec::<ClipboardEntry>::new());

	// load db — store Arc in a signal so closures can Copy it
	let db: Signal<std::sync::Arc<std::sync::Mutex<Database>>> = use_signal(|| {
		let database = Database::open().expect("Failed to open database");
		let existing = database.load_all().unwrap_or_default();
		history.write().extend(existing);
		std::sync::Arc::new(std::sync::Mutex::new(database))
	});

	use_effect(move || {
		let mut rx = monitor::start_listener();
		let db = db().clone();

		spawn(async move {
			while let Some(content) = rx.recv().await {
				let mut entry = ClipboardEntry {
					id: 0,
					content,
					copied_at: Local::now(),
				};

				// send to db and capture the row id
				if let Ok(db) = db.lock() {
					if let Ok(row_id) = db.insert(&entry) {
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

	rsx! {
		Stylesheet { href: TAILWIND_CSS }

		div { class: "p-6 font-sans max-w-3xl mx-auto",
			h2 { class: "text-2xl font-bold mb-6", "shadowpaste" }

			for item in history().iter().rev() {
				div { key: "{item.id}", class: "py-3 border-b border-gray-100 last:border-0",
					ClipboardView { entry: item.clone(), on_delete: delete_entry }
				}
			}
		}
	}
}