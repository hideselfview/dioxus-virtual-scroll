//! Container scroll demo - grid with its own scrollable container
//!
//! Run with: dx serve --example container_demo

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
    let mut album_count = use_signal(|| 500usize);

    let albums = generate_albums(album_count());

    let config = VirtualGridConfig {
        item_width: 180.0,
        item_height: 240.0,
        buffer_rows: 2,
        gap: 12.0,
    };

    let render_item = RenderFn(Rc::new(move |album: Album, _idx: usize| {
        rsx! {
            div { style: "background: #2a2a4e; border-radius: 8px; overflow: hidden; height: 240px;",
                div { style: "aspect-ratio: 1; background: #3a3a5e; display: flex; align-items: center; justify-content: center;",
                    span { style: "color: #555; font-size: 32px;", "{album.id}" }
                }
                div { style: "padding: 10px;",
                    h3 { style: "color: white; font-weight: bold; margin: 0; font-size: 14px;",
                        "{album.title}"
                    }
                }
            }
        }
    }));

    let key_fn = KeyFn(Rc::new(|album: &Album| album.id.clone()));

    rsx! {
        style {
            r#"
            * {{ box-sizing: border-box; }}
            body {{ margin: 0; background: #1a1a2e; font-family: system-ui; height: 100vh; overflow: hidden; }}
            .w-full {{ width: 100%; }}
            .overflow-y-auto {{ overflow-y: auto; }}
            .min-h-0 {{ min-height: 0; }}
            .h-\[calc\(100vh-12rem\)\] {{ height: calc(100vh - 10rem); }}
        "#
        }
        div { style: "display: flex; flex-direction: column; height: 100vh; padding: 20px;",
            div { style: "flex-shrink: 0;",
                h1 { style: "color: white; margin: 0 0 8px 0;", "Container Scroll Demo" }
                p { style: "color: #666; font-size: 14px; margin: 0 0 12px 0;",
                    "Grid has its own scrollable container. The page itself doesn't scroll."
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
                            },
                        }
                    }
                    span { style: "color: #555; font-size: 12px;", "{albums.len()} albums" }
                }
            }
            div { style: "flex: 1; min-height: 0; border: 1px solid #333; border-radius: 8px; overflow: hidden;",
                VirtualGrid {
                    items: albums,
                    config,
                    render_item,
                    key_fn,
                    scroll_target: ScrollTarget::Container,
                    container_class: "h-full".to_string(),
                }
            }
        }
    }
}
