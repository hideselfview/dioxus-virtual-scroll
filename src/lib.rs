//! Virtual scrolling grid component for Dioxus
//!
//! Renders only visible items plus a buffer, using spacer elements to maintain scroll height.
//!
//! ## Scroll Target
//!
//! By default, the grid creates its own scrollable container. Set `scroll_target` to `Window`
//! to use window scrolling instead (useful when the grid is in a page that scrolls).

use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_x::prelude::*;

// =============================================================================
// Cleanup handles
// =============================================================================

/// Cleanup handle for ResizeObserver - disconnects on drop
struct ResizeObserverCleanup {
    observer: web_sys_x::ResizeObserver,
    _callback: Closure<dyn FnMut(Vec<web_sys_x::ResizeObserverEntry>)>,
}

impl Drop for ResizeObserverCleanup {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

/// Cleanup handle for window event listeners
struct WindowListenersCleanup {
    scroll_callback: Closure<dyn FnMut()>,
    resize_callback: Closure<dyn FnMut()>,
}

impl Drop for WindowListenersCleanup {
    fn drop(&mut self) {
        if let Some(window) = web_sys_x::window() {
            let _ = window.remove_event_listener_with_callback(
                "scroll",
                self.scroll_callback.as_ref().unchecked_ref(),
            );
            let _ = window.remove_event_listener_with_callback(
                "resize",
                self.resize_callback.as_ref().unchecked_ref(),
            );
        }
    }
}

// =============================================================================
// Public types
// =============================================================================

/// Scroll target for the virtual grid
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ScrollTarget {
    /// Grid has its own scrollable container (default)
    #[default]
    Container,
    /// Use window/body scrolling
    Window,
}

/// Wrapper for render functions that allows capturing state.
/// PartialEq returns false to ensure re-renders when the closure might have changed.
pub struct RenderFn<T>(pub Rc<dyn Fn(T, usize) -> Element>);

impl<T> Clone for RenderFn<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T> PartialEq for RenderFn<T> {
    fn eq(&self, _other: &Self) -> bool {
        false // Conservative: assume render function may have changed
    }
}

/// Function to extract a stable key from an item for DOM reconciliation.
pub struct KeyFn<T>(pub Rc<dyn Fn(&T) -> String>);

impl<T> Clone for KeyFn<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T> PartialEq for KeyFn<T> {
    fn eq(&self, _other: &Self) -> bool {
        false // Conservative: assume key function may have changed
    }
}

/// Configuration for the virtual grid
#[derive(Clone, PartialEq)]
pub struct VirtualGridConfig {
    /// Minimum width of each item (used to calculate column count)
    pub item_width: f64,
    /// Height of each item (not including gap)
    pub item_height: f64,
    /// Number of extra rows to render above/below viewport
    pub buffer_rows: usize,
    /// Gap between items in pixels
    pub gap: f64,
}

/// Computed grid layout for virtual scrolling
#[derive(Debug, Clone, PartialEq)]
pub struct GridLayout {
    /// Number of columns that fit in the container
    pub columns: usize,
    /// First row to render (including buffer)
    pub start_row: usize,
    /// Last row to render (exclusive, including buffer)
    pub end_row: usize,
    /// First item index to render
    pub start_idx: usize,
    /// Last item index to render (exclusive)
    pub end_idx: usize,
    /// Height of top spacer in pixels
    pub top_padding: f64,
    /// Height of bottom spacer in pixels
    pub bottom_padding: f64,
}

impl GridLayout {
    /// Calculate grid layout based on container dimensions and scroll position
    pub fn calculate(
        item_count: usize,
        config: &VirtualGridConfig,
        container_width: f64,
        container_height: f64,
        scroll_top: f64,
    ) -> Self {
        let columns = ((container_width + config.gap) / (config.item_width + config.gap))
            .floor()
            .max(1.0) as usize;

        let total_rows = if item_count == 0 {
            0
        } else {
            item_count.div_ceil(columns)
        };

        let row_height = config.item_height + config.gap;
        let first_visible_row = (scroll_top / row_height).floor() as usize;
        let visible_row_count = ((container_height / row_height).ceil() as usize).max(1) + 1;

        let start_row = first_visible_row.saturating_sub(config.buffer_rows);
        let end_row = (first_visible_row + visible_row_count + config.buffer_rows).min(total_rows);

        Self {
            columns,
            start_row,
            end_row,
            start_idx: start_row * columns,
            end_idx: (end_row * columns).min(item_count),
            top_padding: (start_row as f64) * row_height,
            bottom_padding: ((total_rows.saturating_sub(end_row)) as f64) * row_height,
        }
    }
}

// =============================================================================
// Shared helpers
// =============================================================================

/// Generate a unique container ID for this grid instance
fn use_container_id() -> String {
    use_hook(|| {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        format!("virtual-grid-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    })
}

/// Compute effective config with measured item height override
fn effective_config(config: &VirtualGridConfig, measured_height: Option<f64>) -> VirtualGridConfig {
    let mut cfg = config.clone();
    if let Some(h) = measured_height {
        cfg.item_height = h;
    }
    cfg
}

/// Slice items to only those visible in the current viewport
fn slice_visible_items<T: Clone>(items: &[T], layout: &GridLayout) -> Vec<(usize, T)> {
    if layout.start_idx < items.len() {
        items[layout.start_idx..layout.end_idx]
            .iter()
            .enumerate()
            .map(|(i, item)| (layout.start_idx + i, item.clone()))
            .collect()
    } else {
        vec![]
    }
}

/// Calculate scroll position to bring an item into view
fn scroll_position_for_key<T>(
    key: &str,
    items: &[T],
    key_fn: &KeyFn<T>,
    config: &VirtualGridConfig,
    container_width: f64,
) -> Option<f64> {
    let index = items.iter().position(|item| (key_fn.0)(item) == key)?;
    let columns = ((container_width + config.gap) / (config.item_width + config.gap))
        .floor()
        .max(1.0) as usize;
    let row = index / columns;
    let row_height = config.item_height + config.gap;
    Some((row as f64) * row_height)
}

// =============================================================================
// Public component - entry point
// =============================================================================

/// Virtual scrolling grid that only renders visible items
#[component]
pub fn VirtualGrid<T: Clone + PartialEq + 'static>(
    items: Vec<T>,
    config: VirtualGridConfig,
    render_item: RenderFn<T>,
    /// Function to extract a stable key from each item
    key_fn: KeyFn<T>,
    #[props(default = "grid-item".to_string())] item_class: String,
    /// Container class - must include height constraint for virtual scrolling to work
    /// (ignored when scroll_target is Window)
    #[props(default = "h-[calc(100vh-12rem)]".to_string())]
    container_class: String,
    /// Scroll target: Container (default) for own scrollable area, Window for body scrolling
    #[props(default)]
    scroll_target: ScrollTarget,
    /// Key of item to scroll to on mount
    #[props(default)]
    initial_scroll_to: Option<String>,
) -> Element {
    match scroll_target {
        ScrollTarget::Container => rsx! {
            ContainerScrollGrid {
                items,
                config,
                render_item,
                key_fn,
                item_class,
                container_class,
                initial_scroll_to,
            }
        },
        ScrollTarget::Window => rsx! {
            WindowScrollGrid {
                items,
                config,
                render_item,
                key_fn,
                item_class,
                initial_scroll_to,
            }
        },
    }
}

// =============================================================================
// Container scroll mode
// =============================================================================

#[component]
fn ContainerScrollGrid<T: Clone + PartialEq + 'static>(
    items: Vec<T>,
    config: VirtualGridConfig,
    render_item: RenderFn<T>,
    key_fn: KeyFn<T>,
    item_class: String,
    container_class: String,
    initial_scroll_to: Option<String>,
) -> Element {
    let mut scroll_top = use_signal(|| 0.0_f64);
    let mut container_width = use_signal(|| 1000.0_f64);
    let mut container_height = use_signal(|| 800.0_f64);
    let measured_item_height: Signal<Option<f64>> = use_signal(|| None);
    let mut mounted_element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let scroll_query_pending = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    let container_id = use_container_id();

    // ResizeObserver for container dimensions
    let resize_observer_handle =
        use_hook(|| Rc::new(std::cell::RefCell::new(None::<ResizeObserverCleanup>)));
    {
        let container_id = container_id.clone();
        let resize_observer_handle = resize_observer_handle.clone();

        use_effect(move || {
            let Some(window) = web_sys_x::window() else {
                return;
            };
            let Some(document) = window.document() else {
                return;
            };
            let Some(element) = document.get_element_by_id(&container_id) else {
                return;
            };

            let callback: Closure<dyn FnMut(Vec<web_sys_x::ResizeObserverEntry>)> = Closure::wrap(
                Box::new(move |entries: Vec<web_sys_x::ResizeObserverEntry>| {
                    for entry in entries {
                        let sizes = entry.content_box_size();
                        let size = sizes.get(0);
                        let size: web_sys_x::ResizeObserverSize = size.unchecked_into();
                        let width = size.inline_size();
                        let height = size.block_size();

                        if (container_width() - width).abs() > 1.0 {
                            container_width.set(width);
                        }
                        if (container_height() - height).abs() > 1.0 {
                            container_height.set(height);
                        }
                    }
                }) as Box<dyn FnMut(Vec<web_sys_x::ResizeObserverEntry>)>,
            );

            let observer =
                web_sys_x::ResizeObserver::new(callback.as_ref().unchecked_ref()).unwrap();
            observer.observe(&element);

            *resize_observer_handle.borrow_mut() = Some(ResizeObserverCleanup {
                observer,
                _callback: callback,
            });
        });
    }

    // Compute layout
    let eff_config = effective_config(&config, measured_item_height());
    let layout = GridLayout::calculate(
        items.len(),
        &eff_config,
        container_width(),
        container_height(),
        scroll_top(),
    );
    let visible_items = slice_visible_items(&items, &layout);

    // Initial scroll handling
    let initial_scroll_done = use_hook(|| std::cell::Cell::new(false));
    {
        let initial_scroll_to = initial_scroll_to.clone();
        let key_fn = key_fn.clone();
        let items = items.clone();
        let eff_config = eff_config.clone();
        let cw = container_width();

        use_effect(move || {
            if let Some(ref key) = initial_scroll_to {
                if !initial_scroll_done.get() && cw > 0.0 {
                    initial_scroll_done.set(true);
                    if let Some(pos) =
                        scroll_position_for_key(key, &items, &key_fn, &eff_config, cw)
                    {
                        scroll_top.set(pos);
                    }
                }
            }
        });
    }

    let container_classes =
        format!("virtual-grid-container w-full overflow-y-auto {container_class}");
    let container_id_for_mount = container_id.clone();

    rsx! {
        div {
            id: "{container_id}",
            class: "{container_classes}",
            style: "overflow-anchor: none;",
            onscroll: move |_evt| {
                if scroll_query_pending.get() {
                    return;
                }

                if let Some(element) = mounted_element.read().clone() {
                    scroll_query_pending.set(true);
                    let pending = scroll_query_pending.clone();
                    spawn(async move {
                        if let Ok(scroll) = element.get_scroll_offset().await {
                            if (scroll_top() - scroll.y).abs() > 0.5 {
                                scroll_top.set(scroll.y);
                            }
                        }
                        pending.set(false);
                    });
                }
            },
            onmounted: move |evt| {
                mounted_element.set(Some(evt.data()));
                let container_id = container_id_for_mount.clone();

                spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(32)).await;

                    let Some(window) = web_sys_x::window() else { return };
                    let Some(document) = window.document() else { return };
                    let Some(element) = document.get_element_by_id(&container_id) else {

                        return
                    };
                    let rect = element.get_bounding_client_rect();
                    container_width.set(rect.width());
                    container_height.set(rect.height());
                });
            },
            GridContent {
                layout,
                visible_items,
                config: eff_config,
                item_class,
                render_item,
                key_fn,
                measured_item_height,
            }
        }
    }
}

// =============================================================================
// Window scroll mode
// =============================================================================

#[component]
fn WindowScrollGrid<T: Clone + PartialEq + 'static>(
    items: Vec<T>,
    config: VirtualGridConfig,
    render_item: RenderFn<T>,
    key_fn: KeyFn<T>,
    item_class: String,
    initial_scroll_to: Option<String>,
) -> Element {
    let mut scroll_top = use_signal(|| 0.0_f64);
    let mut container_width = use_signal(|| 1000.0_f64);
    let mut container_height = use_signal(|| 800.0_f64);
    let measured_item_height: Signal<Option<f64>> = use_signal(|| None);
    let mut element_offset_top: Signal<Option<f64>> = use_signal(|| None);
    let container_id = use_container_id();

    // Window scroll/resize listeners
    let window_listeners_handle =
        use_hook(|| Rc::new(std::cell::RefCell::new(None::<WindowListenersCleanup>)));

    if window_listeners_handle.borrow().is_none() {
        if let Some(window) = web_sys_x::window() {
            if let Ok(inner_height) = window.inner_height() {
                if let Some(h) = inner_height.as_f64() {
                    container_height.set(h);
                }
            }

            let scroll_closure: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
                if let Some(window) = web_sys_x::window() {
                    let window_y = window.scroll_y().unwrap_or(0.0);
                    if let Some(offset) = element_offset_top() {
                        let new_scroll_top = (window_y - offset).max(0.0);
                        if (scroll_top() - new_scroll_top).abs() > 0.5 {
                            scroll_top.set(new_scroll_top);
                        }
                    }
                }
            })
                as Box<dyn FnMut()>);

            let resize_closure: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
                if let Some(window) = web_sys_x::window() {
                    if let Ok(h) = window.inner_height() {
                        if let Some(h) = h.as_f64() {
                            if (container_height() - h).abs() > 1.0 {
                                container_height.set(h);
                            }
                        }
                    }
                }
            })
                as Box<dyn FnMut()>);

            let scroll_options = web_sys_x::AddEventListenerOptions::new();
            scroll_options.set_passive(true);
            window
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "scroll",
                    scroll_closure.as_ref().unchecked_ref(),
                    &scroll_options,
                )
                .ok();

            let resize_options = web_sys_x::AddEventListenerOptions::new();
            resize_options.set_passive(true);
            window
                .add_event_listener_with_callback_and_add_event_listener_options(
                    "resize",
                    resize_closure.as_ref().unchecked_ref(),
                    &resize_options,
                )
                .ok();

            *window_listeners_handle.borrow_mut() = Some(WindowListenersCleanup {
                scroll_callback: scroll_closure,
                resize_callback: resize_closure,
            });
        }
    }

    // ResizeObserver for container width only
    let resize_observer_handle =
        use_hook(|| Rc::new(std::cell::RefCell::new(None::<ResizeObserverCleanup>)));
    {
        let container_id = container_id.clone();
        let resize_observer_handle = resize_observer_handle.clone();

        use_effect(move || {
            let Some(window) = web_sys_x::window() else {
                return;
            };
            let Some(document) = window.document() else {
                return;
            };
            let Some(element) = document.get_element_by_id(&container_id) else {
                return;
            };

            let callback: Closure<dyn FnMut(Vec<web_sys_x::ResizeObserverEntry>)> = Closure::wrap(
                Box::new(move |entries: Vec<web_sys_x::ResizeObserverEntry>| {
                    for entry in entries {
                        let sizes = entry.content_box_size();
                        let size = sizes.get(0);
                        let size: web_sys_x::ResizeObserverSize = size.unchecked_into();
                        let width = size.inline_size();

                        if (container_width() - width).abs() > 1.0 {
                            container_width.set(width);
                        }
                    }
                }) as Box<dyn FnMut(Vec<web_sys_x::ResizeObserverEntry>)>,
            );

            let observer =
                web_sys_x::ResizeObserver::new(callback.as_ref().unchecked_ref()).unwrap();
            observer.observe(&element);

            *resize_observer_handle.borrow_mut() = Some(ResizeObserverCleanup {
                observer,
                _callback: callback,
            });
        });
    }

    // Compute layout
    let eff_config = effective_config(&config, measured_item_height());
    let layout = GridLayout::calculate(
        items.len(),
        &eff_config,
        container_width(),
        container_height(),
        scroll_top(),
    );
    let visible_items = slice_visible_items(&items, &layout);

    // Initial scroll handling
    let initial_scroll_done = use_hook(|| std::cell::Cell::new(false));
    {
        let initial_scroll_to = initial_scroll_to.clone();
        let key_fn = key_fn.clone();
        let items = items.clone();
        let eff_config = eff_config.clone();
        let cw = container_width();

        use_effect(move || {
            if let Some(ref key) = initial_scroll_to {
                if !initial_scroll_done.get() && cw > 0.0 {
                    initial_scroll_done.set(true);
                    if let Some(pos) =
                        scroll_position_for_key(key, &items, &key_fn, &eff_config, cw)
                    {
                        if let Some(window) = web_sys_x::window() {
                            let page_y = element_offset_top().unwrap_or(0.0) + pos;
                            window.scroll_to_with_x_and_y(0.0, page_y);
                        }
                    }
                }
            }
        });
    }

    let container_id_for_mount = container_id.clone();

    rsx! {
        div {
            id: "{container_id}",
            class: "virtual-grid-container w-full",
            style: "overflow-anchor: none;",
            onmounted: move |_evt| {
                let container_id = container_id_for_mount.clone();

                spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(32)).await;

                    let Some(window) = web_sys_x::window() else { return };
                    let Some(document) = window.document() else { return };
                    let Some(element) = document.get_element_by_id(&container_id) else {

                        return
                    };
                    let rect = element.get_bounding_client_rect();
                    let scroll_y = window.scroll_y().unwrap_or(0.0);
                    let page_offset = scroll_y + rect.top();
                    element_offset_top.set(Some(page_offset));
                    container_width.set(rect.width());
                    let initial_scroll = (scroll_y - page_offset).max(0.0);
                    scroll_top.set(initial_scroll);
                });
            },
            GridContent {
                layout,
                visible_items,
                config: eff_config,
                item_class,
                render_item,
                key_fn,
                measured_item_height,
            }
        }
    }
}

// =============================================================================
// Shared grid content (dumb layout component)
// =============================================================================

#[component]
fn GridContent<T: Clone + PartialEq + 'static>(
    layout: GridLayout,
    visible_items: Vec<(usize, T)>,
    config: VirtualGridConfig,
    item_class: String,
    render_item: RenderFn<T>,
    key_fn: KeyFn<T>,
    measured_item_height: Signal<Option<f64>>,
) -> Element {
    let grid_style = format!(
        "display: grid; grid-template-columns: repeat(auto-fill, minmax({}px, 1fr)); gap: {}px;",
        config.item_width, config.gap
    );

    rsx! {
        div {
            class: "virtual-grid-spacer-top",
            style: "height: {layout.top_padding}px;",
        }

        div { class: "virtual-grid-content min-h-0", style: "{grid_style}",
            for (i , (idx , item)) in visible_items.into_iter().enumerate() {
                {
                    let item_key = (key_fn.0)(&item);
                    if i == 0 {
                        rsx! {
                            div {
                                key: "{item_key}",
                                class: "{item_class}",
                                "data-index": "{idx}",
                                "data-key": "{item_key}",
                                onmounted: move |evt| {
                                    spawn(async move {
                                        if let Ok(rect) = evt.get_client_rect().await {
                                            let h = rect.height();
                                            if measured_item_height().is_none_or(|current| (current - h).abs() > 1.0)
                                            {
                                                measured_item_height.set(Some(h));
                                            }
                                        }
                                    });
                                },
                                {(render_item.0)(item, idx)}
                            }
                        }
                    } else {
                        rsx! {
                            div {
                                key: "{item_key}",
                                class: "{item_class}",
                                "data-index": "{idx}",
                                "data-key": "{item_key}",
                                {(render_item.0)(item, idx)}
                            }
                        }
                    }
                }
            }
        }

        div {
            class: "virtual-grid-spacer-bottom",
            style: "height: {layout.bottom_padding}px;",
        }
    }
}
