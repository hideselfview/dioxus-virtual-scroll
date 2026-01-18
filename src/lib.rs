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
    /// Stored to prevent the closure from being dropped (must outlive the event listeners)
    _raf_callback: Rc<Closure<dyn FnMut()>>,
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
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
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
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
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

/// Hook to observe container resize via ResizeObserver.
/// Updates width_signal always; updates height_signal if Some.
fn use_resize_observer(
    container_id: String,
    mut width_signal: Signal<f64>,
    height_signal: Option<Signal<f64>>,
) {
    let handle = use_hook(|| Rc::new(std::cell::RefCell::new(None::<ResizeObserverCleanup>)));
    let handle_clone = handle.clone();

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

                    if (width_signal() - width).abs() > 1.0 {
                        width_signal.set(width);
                    }

                    if let Some(mut h_sig) = height_signal {
                        let height = size.block_size();
                        if (h_sig() - height).abs() > 1.0 {
                            h_sig.set(height);
                        }
                    }
                }
            }) as Box<dyn FnMut(Vec<web_sys_x::ResizeObserverEntry>)>,
        );

        let observer = web_sys_x::ResizeObserver::new(callback.as_ref().unchecked_ref())
            .expect("ResizeObserver should be supported");
        observer.observe(&element);

        *handle_clone.borrow_mut() = Some(ResizeObserverCleanup {
            observer,
            _callback: callback,
        });
    });
}

/// Hook to set up window scroll and resize listeners for window-scroll mode.
fn use_window_scroll_listeners(
    mut scroll_top: Signal<f64>,
    mut container_height: Signal<f64>,
    element_offset_top: Signal<Option<f64>>,
) {
    let handle = use_hook(|| Rc::new(std::cell::RefCell::new(None::<WindowListenersCleanup>)));
    let handle_clone = handle.clone();

    use_effect(move || {
        let Some(window) = web_sys_x::window() else {
            return;
        };

        // Set initial height from window
        if let Ok(inner_height) = window.inner_height() {
            if let Some(h) = inner_height.as_f64() {
                container_height.set(h);
            }
        }

        // rAF-based throttling: pending flag shared between scroll and rAF callbacks
        let scroll_pending = Rc::new(std::cell::Cell::new(false));

        // Create rAF callback once - it reads current scroll position and updates signal
        let pending_for_raf = scroll_pending.clone();
        let raf_callback: Rc<Closure<dyn FnMut()>> = Rc::new(Closure::wrap(Box::new(move || {
            pending_for_raf.set(false);
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
            as Box<dyn FnMut()>));

        // Scroll handler schedules the rAF callback if not already pending
        let pending_for_scroll = scroll_pending.clone();
        let raf_for_scroll = raf_callback.clone();
        let scroll_closure: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
            if pending_for_scroll.get() {
                return;
            }
            pending_for_scroll.set(true);

            if let Some(window) = web_sys_x::window() {
                let _ = window.request_animation_frame((*raf_for_scroll).as_ref().unchecked_ref());
            }
        }) as Box<dyn FnMut()>);

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
        }) as Box<dyn FnMut()>);

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

        *handle_clone.borrow_mut() = Some(WindowListenersCleanup {
            scroll_callback: scroll_closure,
            resize_callback: resize_closure,
            _raf_callback: raf_callback,
        });
    });
}

/// Compute effective config with measured item height override
fn effective_config(config: &VirtualGridConfig, measured_height: Option<f64>) -> VirtualGridConfig {
    let mut cfg = config.clone();
    if let Some(h) = measured_height {
        cfg.item_height = h;
    }
    cfg
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
            }
        },
        ScrollTarget::Window => rsx! {
            WindowScrollGrid {
                items,
                config,
                render_item,
                key_fn,
                item_class,
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
) -> Element {
    let mut scroll_top = use_signal(|| 0.0_f64);
    let mut container_width = use_signal(|| 1000.0_f64);
    let mut container_height = use_signal(|| 800.0_f64);
    let measured_item_height: Signal<Option<f64>> = use_signal(|| None);
    let mut mounted_element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let scroll_query_pending = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    let container_id = use_container_id();

    use_resize_observer(
        container_id.clone(),
        container_width,
        Some(container_height),
    );

    // Compute layout
    let eff_config = effective_config(&config, measured_item_height());
    let layout = GridLayout::calculate(
        items.len(),
        &eff_config,
        container_width(),
        container_height(),
        scroll_top(),
    );

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

                // Wait for layout to stabilize before measuring
                spawn(async move {
                    wait_for_layout().await;

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
                items,
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
) -> Element {
    let mut scroll_top = use_signal(|| 0.0_f64);
    let mut container_width = use_signal(|| 1000.0_f64);
    let container_height = use_signal(|| 800.0_f64);
    let measured_item_height: Signal<Option<f64>> = use_signal(|| None);
    let mut element_offset_top: Signal<Option<f64>> = use_signal(|| None);
    let container_id = use_container_id();

    use_window_scroll_listeners(scroll_top, container_height, element_offset_top);
    use_resize_observer(container_id.clone(), container_width, None);

    // Compute layout
    let eff_config = effective_config(&config, measured_item_height());
    let layout = GridLayout::calculate(
        items.len(),
        &eff_config,
        container_width(),
        container_height(),
        scroll_top(),
    );

    let container_id_for_mount = container_id.clone();

    rsx! {
        div {
            id: "{container_id}",
            class: "virtual-grid-container w-full",
            style: "overflow-anchor: none;",
            onmounted: move |_evt| {
                let container_id = container_id_for_mount.clone();

                // Wait for layout to stabilize before measuring
                spawn(async move {
                    wait_for_layout().await;

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
                items,
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
    items: Vec<T>,
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

    // Only measure from first item when needed
    let needs_measurement = measured_item_height.read().is_none();

    rsx! {
        div {
            class: "virtual-grid-spacer-top",
            style: "height: {layout.top_padding}px;",
        }

        div { class: "virtual-grid-content min-h-0", style: "{grid_style}",
            for (i , idx) in (layout.start_idx..layout.end_idx).enumerate() {
                {
                    let item = items[idx].clone();
                    let item_key = (key_fn.0)(&item);
                    let measure_this = i == 0 && needs_measurement;
                    rsx! {
                        div {
                            key: "{item_key}",
                            class: "{item_class}",
                            "data-index": "{idx}",
                            "data-key": "{item_key}",
                            onmounted: move |evt| {
                                if !measure_this {
                                    return;
                                }
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
                }
            }
        }

        div {
            class: "virtual-grid-spacer-bottom",
            style: "height: {layout.bottom_padding}px;",
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Waits for two animation frames, allowing layout to stabilize after mount.
async fn wait_for_layout() {
    let promise =
        js_sys_x::eval("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
            .expect("eval for rAF promise should not fail")
            .unchecked_into::<js_sys_x::Promise>();

    let _ = wasm_bindgen_futures_x::JsFuture::from(promise).await;
}
