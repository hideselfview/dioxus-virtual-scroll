# dioxus-virtual-scroll

Virtual scrolling for Dioxus. Renders only visible items.

## Status

Early development. API will change.

## Features

- **Grid layout** - auto-fills columns based on container width
- **Scroll targets** - scroll within a container or use window scrolling
- **Auto-measurement** - measures container and item dimensions via ResizeObserver

## Not Implemented

- **List layout** - single column virtualized list
- **Scroll handle** - programmatic seeking to specific items

## Usage

```rust
use dioxus_virtual_scroll::{VirtualGrid, VirtualGridConfig, RenderFn, KeyFn, ScrollTarget};

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
        scroll_target: ScrollTarget::Container, // or ScrollTarget::Window
        container_class: "h-[500px]", // height required for Container mode
    }
}
```

## Examples

```bash
# Desktop stress test (demonstrates IPC issue under load)
cargo run --example stress_test

# Web demo (for browser testing)
dx serve --example web_demo
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

