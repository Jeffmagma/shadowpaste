use dioxus::prelude::*;

#[component]
pub fn Titlebar() -> Element {
    let window = dioxus::desktop::use_window();

    rsx! {
        div {
            class: "h-10 bg-slate-900/90 backdrop-blur-md flex items-center justify-between select-none border-b border-slate-800 shrink-0 z-50",
            onmousedown: {
                let window = window.clone();
                move |_| window.drag()
            },
            div { class: "pl-4 flex items-center gap-2",
                span { class: "font-bold text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-indigo-500", "shadowpaste" }
                span { class: "text-xs text-slate-500", "v0.1.0" }
            }
            div { class: "flex h-full",
                // prevent drag on controls container so clicks register
                onmousedown: move |evt| evt.stop_propagation(),
                // minimize
                button {
                    onclick: {
                        let window = window.clone();
                        move |_| window.set_minimized(true)
                    },
                    class: "w-12 h-full hover:bg-slate-800 flex items-center justify-center text-slate-400 transition-colors",
                    span { class: "icon", "─" }
                }
                // maximize/restore
                button {
                    onclick: {
                        let window = window.clone();
                        move |_| window.toggle_maximized()
                    },
                    class: "w-12 h-full hover:bg-slate-800 flex items-center justify-center text-slate-400 transition-colors",
                    span { class: "icon", "□" }
                }
                // close
                button {
                    onclick: {
                        let window = window.clone();
                        move |_| window.close()
                    },
                    class: "w-12 h-full hover:bg-red-500 hover:text-white flex items-center justify-center text-slate-400 transition-colors",
                    "✕"
                }
            }
        }
    }
}
