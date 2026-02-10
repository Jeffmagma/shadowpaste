#![allow(non_snake_case)] // uppercase component function names
mod db;
mod monitor;

use chrono::Local;
use db::{ClipboardEntry, Database};
use dioxus::prelude::*;
use monitor::ClipboardContent;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
	dioxus::launch(App);
}

#[component]
fn ClipboardView(entry: ClipboardEntry) -> Element {
	let time_str = entry.copied_at.format("%b %d, %I:%M %p").to_string();

	rsx! {
		div { class: "flex flex-col gap-1",
			span { class: "text-xs text-slate-400", "{time_str}" }
			match entry.content {
				ClipboardContent::Text(ref text) => rsx! {
					p { class: "text-sm line-clamp-3 font-mono", "{text}" }
				},
				ClipboardContent::Image(ref src) => rsx! {
					img { src: "{src}", class: "max-w-full h-auto rounded" }
				},
				ClipboardContent::Empty => rsx! {
					p { class: "text-xs text-slate-400", "Empty Clipboard" }
				}
			}
		}
	}
}

fn App() -> Element {
	let mut history = use_signal(|| Vec::<ClipboardEntry>::new());

	// load db
	let db = use_hook(|| {
		let database = Database::open().expect("Failed to open database");
		let existing = database.load_all().unwrap_or_default();
		history.write().extend(existing);
		std::sync::Arc::new(std::sync::Mutex::new(database))
	});

	use_effect(move || {
		let mut rx = monitor::start_listener();
		let db = db.clone();

		spawn(async move {
			while let Some(content) = rx.recv().await {
				let entry = ClipboardEntry {
					id: 0,
					content,
					copied_at: Local::now(),
				};

				// send to db
				if let Ok(db) = db.lock() {
					let _ = db.insert(&entry);
				}

				history.write().push(entry);
			}
		});
	});

	rsx! {
		document::Stylesheet { href: TAILWIND_CSS }

		div { class: "p-6 font-sans max-w-3xl mx-auto",
			h2 { class: "text-2xl font-bold mb-6", "shadowpaste" }

			for (i, item) in history().iter().enumerate() {
				div { key: "{i}", class: "py-3 border-b border-gray-100 last:border-0",
					ClipboardView { entry: item.clone() }
				}
			}
		}
	}
}