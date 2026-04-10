use std::{borrow::Cow, sync::{Arc, Mutex}};
use arboard::{Clipboard, ImageData};
use dioxus::desktop::tao::event::{Event, WindowEvent};
use dioxus::desktop::{use_window, use_wry_event_handler, Config, WindowBuilder};
use dioxus::prelude::*;

use crate::db::ClipboardEntry;
use crate::monitor::ClipboardContent;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
pub type ClipboardWriteSuppression = Arc<Mutex<bool>>;

fn decode_data_uri(data_uri: &str) -> Option<Vec<u8>> {
	let (_, b64) = data_uri.split_once(";base64,")?;
	use base64::{engine::general_purpose, Engine as _};
	general_purpose::STANDARD.decode(b64).ok()
}

fn write_clipboard_content(content: &ClipboardContent) -> anyhow::Result<()> {
	let mut clipboard = Clipboard::new()?;
	match content {
		ClipboardContent::Text(text) => clipboard.set_text(text.clone())?,
		ClipboardContent::Image(data_uri) => {
			let bytes = decode_data_uri(data_uri).ok_or_else(|| anyhow::anyhow!("Invalid image data URI"))?;
			let rgba = image::load_from_memory(&bytes)?.into_rgba8();
			let (width, height) = rgba.dimensions();
			clipboard.set_image(ImageData {
				width: width as usize,
				height: height as usize,
				bytes: Cow::Owned(rgba.into_raw()),
			})?;
		}
		ClipboardContent::Empty => {}
	}
	Ok(())
}

/// compact popup window that shows the clipboard history list, opens with ctrl+shift+v
/// closes itself when it loses focus
#[component]
pub fn QuickPaste(entries: Vec<ClipboardEntry>) -> Element {
	let window = use_window();
	let entry_count = entries.len();

	// focus the popup on open
	let window_for_focus = window.clone();
	use_effect(move || {
		window_for_focus.set_focus();
	});

	// close when the window loses focus
	let window_for_focus = window.clone();
	use_wry_event_handler(move |event, _| {
		if let Event::WindowEvent { event: WindowEvent::Focused(false), .. } = event {
			window_for_focus.set_visible(false);
		}
	});

	rsx! {
        Stylesheet { href: TAILWIND_CSS }
        style { "
            ::-webkit-scrollbar {{ width: 6px; }}
            ::-webkit-scrollbar-track {{ background: transparent; }}
            ::-webkit-scrollbar-thumb {{ background: #334155; border-radius: 3px; }}
        " }

        div {
            class: "h-screen w-screen bg-slate-950 text-slate-200 flex flex-col font-sans overflow-hidden rounded-xl border border-slate-800 shadow-2xl",

            // header bar
            div {
                class: "shrink-0 px-3 py-2 border-b border-slate-800 flex items-center gap-2",
                span { class: "text-xs font-semibold text-slate-400 uppercase tracking-widest", "Clipboard" }
	                span { class: "ml-auto text-xs text-slate-600", "{entry_count} items" }
            }

            // clip list
            div { class: "flex-1 overflow-y-auto",
	                if entries.is_empty() {
                    div { class: "flex flex-col items-center justify-center h-full text-slate-600 gap-2",
                        div { class: "text-3xl opacity-20", "📋" }
                        p { class: "text-xs", "No clipboard history" }
                    }
                }
	                for entry in entries.iter().cloned() {
                    QuickPasteRow { entry }
                }
            }
        }
    }
}

/// a single row in the quickpaste list
#[component]
fn QuickPasteRow(entry: ClipboardEntry) -> Element {
	let window = use_window();
	let suppression = use_context::<ClipboardWriteSuppression>();
	let time_str = entry.copied_at.format("%b %d, %I:%M %p").to_string();
	let selected_content = entry.content.clone();

	rsx! {
        div {
            class: "px-3 py-2 border-b border-slate-800/60 hover:bg-slate-800/50 cursor-pointer transition-colors",
	            onclick: move |_| {
	                if let Ok(mut suppressed) = suppression.lock() {
	                    *suppressed = true;
	                }

	                if let Err(err) = write_clipboard_content(&selected_content) {
	                    if let Ok(mut suppressed) = suppression.lock() {
	                        *suppressed = false;
	                    }
	                    eprintln!("Failed to write selected clipboard item: {err}");
	                    return;
	                }

	                window.set_visible(false);
	            },
            div { class: "text-xs text-slate-500 mb-0.5", "{time_str}" }
            match entry.content {
                ClipboardContent::Text(ref text) => rsx! {
                    p {
                        class: "text-sm font-mono truncate text-slate-200",
                        "{text}"
                    }
                },
                ClipboardContent::Image(_) => rsx! {
                    p { class: "text-xs text-slate-400 italic", "🖼 Image" }
                },
                ClipboardContent::Empty => rsx! {
                    p { class: "text-xs text-slate-600 italic", "Empty" }
                },
            }
        }
    }
}

/// window config for quickpaste window
pub fn quick_paste_config() -> Config {
	Config::new()
		.with_window(
			WindowBuilder::new()
				.with_title("shadowpaste – quick paste")
				.with_decorations(false)
				.with_transparent(true)
				.with_resizable(false)
				// so there's this issue where if the focused state is not set to false, the first instance of focus is not registered and the
				// user has to unfocus twice before the unfocus event is triggered. not sure if it's a bug or not, but this fixes it temporarily
				.with_focused(false)
				.with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(360.0_f64, 480.0_f64))
				.with_always_on_top(true)
		)
		.with_close_behaviour(dioxus::desktop::WindowCloseBehaviour::WindowHides)
}

