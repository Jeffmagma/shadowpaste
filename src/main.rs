#![allow(non_snake_case)] // uppercase component function names
mod monitor;

use dioxus::prelude::*;
use monitor::ClipboardContent;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
	dioxus::launch(App);
}

#[component]
fn ClipboardView(content: ClipboardContent) -> Element {
	match content {
		ClipboardContent::Text(text) => rsx! {
			p { class: "text-sm line-clamp-3 font-mono", "{text}" }
		},
		ClipboardContent::Image(src) => rsx! {
			img { src: "{src}", class: "max-w-full h-auto rounded" }
		},
		ClipboardContent::Empty => rsx! {
			p { class: "text-xs text-slate-400", "Empty Clipboard" }
		}
	}
}

fn App() -> Element {
	let mut history = use_signal(|| Vec::<ClipboardContent>::new());

	use_effect(move || {
		let mut rx = monitor::start_listener();
		
		spawn(async move {
			while let Some(content) = rx.recv().await {
				history.write().push(content);
			}
		});
	});

	rsx! {
		document::Stylesheet { href: TAILWIND_CSS }

		div { class: "p-6 font-sans max-w-3xl mx-auto",
			h2 { class: "text-2xl font-bold mb-6", "shadowpaste" }

			for (i, item) in history().iter().enumerate() {
				div { key: "{i}", class: "py-3 border-b border-gray-100 last:border-0",
					ClipboardView { content: item.clone() }
				}
			}
		}
	}
}