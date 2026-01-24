# dioxus-virtual-scroll

Virtual scrolling for Dioxus. [Demo](https://hideselfview.github.io/dioxus-virtual-scroll/demo)

https://github.com/user-attachments/assets/c85c43f8-efd9-41de-9361-a030e36621bd

## Status

Early development. Basic features work.

## Features

- **Grid layout** - auto-fills columns based on container width
- **Auto-measurement** - measures container and item dimensions with ResizeObserver
- **3 scroll modes** - Window, Container, or Element (see below)
- **Tested** - [E2E tests](e2e/virtual-grid.spec.ts) with Playwright

## Scroll Modes

### Container (default)

The grid creates its own scrollable container. Use when you want the grid to manage its own scroll area.

```
┌─────────────────────────┐
│  Page content           │
├─────────────────────────┤
│ ┌─────────────────────┐ │
│ │ VirtualGrid         │ │  ← This div scrolls
│ │ ┌─────┐ ┌─────┐     │ │
│ │ │     │ │     │ ... │▐│
│ │ └─────┘ └─────┘     │▐│
│ │ ┌─────┐ ┌─────┐     │▐│  (scrollbar)
│ │ │     │ │     │ ... │▐│
│ │ └─────┘ └─────┘     │ │
│ └─────────────────────┘ │
└─────────────────────────┘
```

```rust
VirtualGrid {
    scroll_target: ScrollTarget::Container,
    container_style: "height: 500px;",  // height required
    // ...
}
```

### Window

The grid uses window/body scrolling. Use when the grid is part of a full-page layout.

```
┌─────────────────────────┐
│  Page header            │
├─────────────────────────┤
│  VirtualGrid            │
│ ┌─────┐ ┌─────┐         │
│ │     │ │     │ ...     │
│ └─────┘ └─────┘         │▐
│ ┌─────┐ ┌─────┐         │▐  ← Window scrollbar
│ │     │ │     │ ...     │▐
│ └─────┘ └─────┘         │
│         ...             │
└─────────────────────────┘
```

```rust
VirtualGrid {
    scroll_target: ScrollTarget::Window,
    // ...
}
```

### Element

The grid uses an external scrollable element that you provide via `MountedData`. Use when the grid lives inside an existing scroll container alongside other content.

```
┌─────────────────────────┐
│  Page content           │
├─────────────────────────┤
│ ┌─────────────────────┐ │
│ │   Scroll element    │ │  ← This div scrolls
│ │ ┌─────────────────┐ │▐│
│ │ │ Header/content  │ │▐│
│ │ └─────────────────┘ │▐│  (scrollbar)
│ │  VirtualGrid        │▐│
│ │ ┌─────┐ ┌─────┐     │▐│
│ │ │     │ │     │ ... │▐│
│ │ └─────┘ └─────┘     │ │
│ └─────────────────────┘ │
└─────────────────────────┘
```

```rust
let mut scroll_container: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

rsx! {
    div {
        style: "height: 500px; overflow-y: auto;",
        onmounted: move |evt| scroll_container.set(Some(evt.data())),
        
        div { "Content above the grid" }
        
        VirtualGrid {
            scroll_target: ScrollTarget::Element(scroll_container.into()),
            // ...
        }
    }
}
```

## TODO

- **Initial scroll** - always begins at 0
- **Scroll handle** - programmatic seeking to specific items

## Usage

```rust
use dioxus::prelude::*;
use dioxus_virtual_scroll::{VirtualGrid, VirtualGridConfig, RenderFn, KeyFn, ScrollTarget};
use std::rc::Rc;

let config = VirtualGridConfig {
    item_width: 200.0,
    item_height: 280.0,
    buffer_rows: 2,
    gap: 16.0,
};

let render_item = RenderFn(Rc::new(|item: MyItem, idx: usize| {
    rsx! { div { "{item.name}" } }
}));

let key_fn = KeyFn(Rc::new(|item: &MyItem| item.id.clone()));

rsx! {
    VirtualGrid {
        items: my_items,
        config,
        render_item,
        key_fn,
        scroll_target: ScrollTarget::Container,
        container_style: "height: 500px;",  // height required for Container mode
    }
}
```

## Examples

```bash
# Web demo (for browser testing)
dx serve --example web_demo

# Desktop stress test (demonstrates IPC issue under load)
cargo run --example stress_test
```

## Development

```bash
# Install git hooks (runs fmt, clippy, playwright on commit)
./scripts/setup-hooks.sh

# Install e2e test dependencies
cd e2e && npm install && npx playwright install chromium

# Run e2e tests manually
cd e2e && npm test
```

## Known Issues

In Dioxus desktop, heavy scrolling with concurrent activity (e.g., image loading) can trigger IPC buffer exhaustion (`U8BufferEmpty` panic). See `examples/stress_test.rs` for reproduction.

## License

MIT OR Apache-2.0

