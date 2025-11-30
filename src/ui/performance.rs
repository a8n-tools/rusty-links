use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: u64) {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::js_sys;

    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap();
    });
    let _ = JsFuture::from(promise).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
async fn sleep_ms(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

// Fallback for non-WASM, non-server builds (e.g., cargo check --features web on x86_64)
#[cfg(all(not(target_arch = "wasm32"), not(feature = "server")))]
async fn sleep_ms(_ms: u64) {
    // No-op: this code path only exists for compilation purposes
    // The actual runtime will use either WASM or server implementations
}

/// Debounce a signal with a configurable delay
/// Returns a debounced signal that updates after the delay period
/// Only updates the debounced value if it actually changed
pub fn use_debounced<T: Clone + PartialEq + 'static>(
    source: Signal<T>,
    delay_ms: u64,
) -> Signal<T> {
    let mut debounced = use_signal(|| source());

    use_effect(move || {
        let current_value = source();

        spawn(async move {
            sleep_ms(delay_ms).await;

            // Only update if the value is different from current debounced value
            // and matches what the source currently is (avoids stale updates)
            if debounced() != current_value && source() == current_value {
                debounced.set(current_value);
            }
        });
    });

    debounced
}

/// Helper to detect if a large list should use virtual scrolling
pub fn should_use_virtual_scroll(item_count: usize) -> bool {
    item_count > 1000
}

/// Calculate visible range for virtual scrolling
pub fn calculate_visible_range(
    scroll_top: f64,
    container_height: f64,
    item_height: f64,
    total_items: usize,
    buffer: usize,
) -> (usize, usize) {
    let visible_start = (scroll_top / item_height).floor() as usize;
    let visible_count = (container_height / item_height).ceil() as usize;

    let start = visible_start.saturating_sub(buffer);
    let end = (visible_start + visible_count + buffer).min(total_items);

    (start, end)
}
