//! Reproduces wry-bindgen bridge crashes in dioxus-desktop.
//!
//! The wry-bindgen IPC bridge between Rust and the webview can panic with
//! `U8BufferEmpty` / `U32BufferEmpty` when web_sys calls run before the bridge
//! has finished initializing. This happens because bridge initialization is not
//! atomic -- the Rust thread can start running effects before the JS side is ready.
//!
//! Run with: cargo run --example bridge_crash
//!
//! The app renders a VirtualGrid whose hooks (`use_resize_observer`,
//! `use_element_scroll_listeners`) make web_sys calls on mount. On some launches
//! these calls will hit an uninitialized bridge and panic.
//!
//! See: docs/wry-bindgen-crashes.md

use dioxus::prelude::*;
use dioxus_virtual_scroll::{KeyFn, RenderFn, VirtualGrid, VirtualGridConfig};
use std::rc::Rc;

const ITEM_COUNT: usize = 500;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(make_config())
        .launch(App);
}

fn make_config() -> dioxus::desktop::Config {
    use dioxus::desktop::WindowBuilder;

    dioxus::desktop::Config::default()
        .with_background_color((0x1a, 0x1a, 0x2e, 0xff))
        .with_window(
            WindowBuilder::new()
                .with_title("bridge crash repro")
                .with_inner_size(dioxus::desktop::LogicalSize::new(800, 600)),
        )
}

#[derive(Clone, PartialEq)]
struct Item {
    id: String,
    label: String,
}

fn generate_items(count: usize) -> Vec<Item> {
    (0..count)
        .map(|i| Item {
            id: format!("{i}"),
            label: format!("Item {i}"),
        })
        .collect()
}

#[component]
fn App() -> Element {
    let items = generate_items(ITEM_COUNT);

    let config = VirtualGridConfig {
        item_width: 200.0,
        item_height: 80.0,
        buffer_rows: 2,
        gap: 8.0,
    };

    let render_item = RenderFn(Rc::new(move |item: Item, _idx: usize| {
        rsx! {
            div {
                style: "background: #2a2a4e; border-radius: 4px; padding: 12px; height: 80px; display: flex; align-items: center; justify-content: center;",
                span { style: "color: white;", "{item.label}" }
            }
        }
    }));

    let key_fn = KeyFn(Rc::new(|item: &Item| item.id.clone()));

    rsx! {
        style { r#"body {{ margin: 0; background: #1a1a2e; font-family: system-ui; }}"# }
        div {
            style: "padding: 16px; height: 100vh; display: flex; flex-direction: column;",
            h1 { style: "color: white; margin: 0 0 8px 0; font-size: 18px;",
                "wry-bindgen bridge crash repro"
            }
            p { style: "color: #888; margin: 0 0 16px 0; font-size: 14px;",
                "VirtualGrid hooks make web_sys calls on mount. "
                "On some launches these hit an uninitialized bridge and panic with U8BufferEmpty."
            }
            VirtualGrid {
                items,
                config,
                render_item,
                key_fn,
                container_style: "flex: 1; min-height: 0; height: 100%;",
            }
        }
    }
}
