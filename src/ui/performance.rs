use dioxus::prelude::*;
use std::time::Duration;

/// Debounce a signal value with a configurable delay
/// Returns a debounced signal that updates after the delay period
pub fn use_debounced<T: Clone + PartialEq + 'static>(
    value: T,
    delay_ms: u64,
) -> Signal<T> {
    let mut debounced = use_signal(|| value.clone());

    use_effect(move || {
        let current_value = value.clone();
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            debounced.set(current_value);
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
