//! Stress test demonstrating IPC buffer exhaustion in dioxus-desktop
//!
//! This example shows that VirtualGrid's web_sys_x usage can crash under load,
//! especially when combined with image loading.
//!
//! Run with: cargo run --example stress_test
//!
//! Toggle ENABLE_IMAGES to compare crash speed:
//! - With images: crashes quickly when scrolling
//! - Without images: crashes eventually under aggressive scrolling

use dioxus::prelude::*;
use dioxus_virtual_scroll::{KeyFn, RenderFn, VirtualGrid, VirtualGridConfig};
use std::borrow::Cow;
use std::rc::Rc;

/// Toggle this to compare crash speed with/without images
const ENABLE_IMAGES: bool = true;
const ALBUM_COUNT: usize = 2000;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(make_config())
        .launch(App);
}

fn make_config() -> dioxus::desktop::Config {
    use dioxus::desktop::wry::http::Response as HttpResponse;
    use dioxus::desktop::WindowBuilder;

    dioxus::desktop::Config::default()
        .with_background_color((0x1a, 0x1a, 0x2e, 0xff))
        .with_window(WindowBuilder::new().with_maximized(true))
        .with_custom_protocol("test", move |_webview_id, request| {
            let uri = request.uri().to_string();

            if uri.starts_with("test://image/") {
                // Blocking makes the crash happen faster by creating more load
                std::thread::sleep(std::time::Duration::from_millis(1));

                // Return a 1x1 gray PNG
                let png_data: &[u8] = &[
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49,
                    0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02,
                    0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44,
                    0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x04, 0x00,
                    0x01, 0x27, 0x34, 0x27, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
                    0xAE, 0x42, 0x60, 0x82,
                ];

                HttpResponse::builder()
                    .status(200)
                    .header("Content-Type", "image/png")
                    .body(Cow::Owned(png_data.to_vec()))
                    .unwrap()
            } else {
                HttpResponse::builder()
                    .status(404)
                    .body(Cow::Borrowed(b"Not found" as &[u8]))
                    .unwrap()
            }
        })
}

#[derive(Clone, PartialEq)]
struct Album {
    id: String,
    title: String,
    cover_url: Option<String>,
}

fn generate_albums(count: usize, with_images: bool) -> Vec<Album> {
    (0..count)
        .map(|i| Album {
            id: format!("{}", i + 1),
            title: format!("Album {}", i + 1),
            cover_url: if with_images {
                Some(format!("test://image/{}", i + 1))
            } else {
                None
            },
        })
        .collect()
}

#[component]
fn App() -> Element {
    let albums = generate_albums(ALBUM_COUNT, ENABLE_IMAGES);

    let config = VirtualGridConfig {
        item_width: 200.0,
        item_height: 280.0,
        buffer_rows: 2,
        gap: 16.0,
    };

    let render_item = RenderFn(Rc::new(move |album: Album, _idx: usize| {
        rsx! {
            div { style: "background: #2a2a4e; border-radius: 8px; overflow: hidden; height: 280px;",
                div { style: "aspect-ratio: 1; background: #3a3a5e; display: flex; align-items: center; justify-content: center;",
                    if let Some(url) = &album.cover_url {
                        img {
                            src: "{url}",
                            style: "width: 100%; height: 100%; object-fit: cover;",
                        }
                    } else {
                        span { style: "color: #666;", "No Image" }
                    }
                }
                div { style: "padding: 12px;",
                    div { style: "color: white; font-weight: bold;", "{album.title}" }
                    div { style: "color: #888; font-size: 12px;", "ID: {album.id}" }
                }
            }
        }
    }));

    let key_fn = KeyFn(Rc::new(|album: &Album| album.id.clone()));

    rsx! {
        style {
            r#"
            body {{ margin: 0; background: #1a1a2e; font-family: system-ui; }}
            /* VirtualGrid needs these Tailwind-like classes */
            .w-full {{ width: 100%; }}
            .overflow-y-auto {{ overflow-y: auto; }}
            .min-h-0 {{ min-height: 0; }}
            /* Container needs height constraint for virtualization */
            .grid-container {{ flex: 1; min-height: 0; height: 100%; }}
        "#
        }
        div { style: "padding: 20px; height: 100vh; display: flex; flex-direction: column;",
            h1 { style: "color: white; margin: 0 0 8px 0;", "VirtualGrid IPC Stress Test" }
            p { style: "color: #888; margin: 0 0 16px 0;",
                "{ALBUM_COUNT} albums, images: {ENABLE_IMAGES}"
            }
            p { style: "color: #666; margin: 0 0 16px 0; font-size: 14px;",
                "Scroll aggressively to trigger: 'Failed to decode return value: U8BufferEmpty'"
            }
            VirtualGrid {
                items: albums,
                config,
                render_item,
                key_fn,
                container_class: "grid-container",
            }
        }
    }
}
