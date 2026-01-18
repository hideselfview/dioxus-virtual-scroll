//! Web demo for Playwright testing
//!
//! Run with: dx serve --example web_demo

use dioxus::prelude::*;
use dioxus_virtual_scroll::{KeyFn, RenderFn, ScrollTarget, VirtualGrid, VirtualGridConfig};
use std::rc::Rc;

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq)]
struct Album {
    id: String,
    title: String,
}

fn generate_albums(count: usize) -> Vec<Album> {
    (0..count)
        .map(|i| Album {
            id: format!("{}", i + 1),
            title: format!("Album {}", i + 1),
        })
        .collect()
}

#[component]
fn App() -> Element {
    let mut album_count = use_signal(|| 200usize);
    let mut cycle = use_signal(|| 0u32);
    let mut scroll_to_input = use_signal(String::new);

    let albums = generate_albums(album_count());

    // Convert scroll_to input to Option<String> for the grid
    let initial_scroll_to = {
        let input = scroll_to_input();
        if input.is_empty() {
            None
        } else {
            Some(input)
        }
    };

    let config = VirtualGridConfig {
        item_width: 200.0,
        item_height: 280.0,
        buffer_rows: 2,
        gap: 16.0,
    };

    let render_item = RenderFn(Rc::new(move |album: Album, _idx: usize| {
        rsx! {
            div {
                "data-testid": "album-card",
                style: "background: #2a2a4e; border-radius: 8px; overflow: hidden; height: 280px;",
                div {
                    style: "aspect-ratio: 1; background: #3a3a5e; display: flex; align-items: center; justify-content: center;",
                    span { style: "color: #666; font-size: 48px;", "🎵" }
                }
                div { style: "padding: 12px;",
                    h3 { style: "color: white; font-weight: bold; margin: 0;", "{album.title}" }
                    p { style: "color: #888; font-size: 12px; margin: 4px 0 0 0;", "ID: {album.id}" }
                }
            }
        }
    }));

    let key_fn = KeyFn(Rc::new(|album: &Album| album.id.clone()));

    let cycle_val = cycle();

    rsx! {
        style { r#"
            * {{ box-sizing: border-box; }}
            body {{ margin: 0; background: #1a1a2e; font-family: system-ui; }}
            .w-full {{ width: 100%; }}
            .overflow-y-auto {{ overflow-y: auto; }}
            .min-h-0 {{ min-height: 0; }}
        "#}
        div { style: "padding: 20px;",
            h1 { style: "color: white; margin: 0 0 8px 0;",
                "VirtualGrid Demo"
            }
            div { style: "display: flex; gap: 16px; margin-bottom: 16px; align-items: center;",
                label { style: "color: #888;",
                    "Albums: "
                    input {
                        r#type: "number",
                        value: "{album_count}",
                        style: "width: 80px; padding: 4px;",
                        oninput: move |e| {
                            if let Ok(n) = e.value().parse::<usize>() {
                                album_count.set(n);
                            }
                        }
                    }
                }
                label { style: "color: #888;",
                    "Initial scroll to: "
                    input {
                        r#type: "text",
                        value: "{scroll_to_input}",
                        placeholder: "album id",
                        style: "width: 80px; padding: 4px;",
                        oninput: move |e| scroll_to_input.set(e.value())
                    }
                }
                button {
                    style: "padding: 4px 12px; cursor: pointer;",
                    onclick: move |_| cycle += 1,
                    "Remount"
                }
            }
            p { style: "color: #666; font-size: 14px; margin: 0 0 16px 0;",
                "{albums.len()} albums loaded (cycle {cycle_val})"
            }
        }
        div { style: "height: calc(100vh - 150px); padding: 0 20px 20px 20px;",
            VirtualGrid {
                key: "{cycle_val}",
                items: albums,
                config,
                render_item,
                key_fn,
                scroll_target: ScrollTarget::Window,
                initial_scroll_to,
            }
        }
    }
}
