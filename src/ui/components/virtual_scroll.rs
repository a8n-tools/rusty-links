//! Virtual scrolling utilities and helper functions
//!
//! Note: Virtual scrolling is implemented via server-side pagination in this app.
//! This module provides utilities for calculating visible ranges for future use.
//!
//! For tables with > 1000 items, server-side pagination (20 items/page) is already
//! handling performance optimization.

/// Helper struct for virtual scroll calculations
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualScrollState {
    pub scroll_top: f64,
    pub container_height: f64,
    pub item_height: f64,
    pub total_items: usize,
    pub buffer: usize,
}

impl VirtualScrollState {
    /// Calculate the visible range of items
    pub fn visible_range(&self) -> (usize, usize) {
        let visible_start = (self.scroll_top / self.item_height).floor() as usize;
        let visible_count = (self.container_height / self.item_height).ceil() as usize;

        let start = visible_start.saturating_sub(self.buffer);
        let end = (visible_start + visible_count + self.buffer).min(self.total_items);

        (start, end)
    }

    /// Check if an item is currently visible
    pub fn is_visible(&self, index: usize) -> bool {
        let (start, end) = self.visible_range();
        index >= start && index < end
    }

    /// Calculate the offset for positioning
    pub fn offset(&self) -> f64 {
        let (start, _) = self.visible_range();
        start as f64 * self.item_height
    }

    /// Calculate total height for the scroll container
    pub fn total_height(&self) -> f64 {
        self.total_items as f64 * self.item_height
    }
}
