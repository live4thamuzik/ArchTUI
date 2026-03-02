//! Scrolling and viewport management utilities
//!
//! Provides clean, reusable scrolling logic for all TUI components.

#![allow(dead_code)]

/// Scrolling state for any scrollable component
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollState {
    /// Current scroll offset (index of first visible item)
    pub offset: usize,
    /// Total number of items
    pub total_items: usize,
    /// Number of items that can be visible at once
    pub visible_items: usize,
    /// Currently selected item index
    pub selected_index: usize,
}

impl ScrollState {
    pub fn new(total_items: usize, visible_items: usize) -> Self {
        Self {
            offset: 0,
            total_items,
            visible_items,
            selected_index: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_selected_visible();
        }
    }

    pub fn move_down(&mut self) {
        if self.total_items > 0 && self.selected_index < self.total_items - 1 {
            self.selected_index += 1;
            self.ensure_selected_visible();
        }
    }

    pub fn page_up(&mut self) {
        if self.selected_index >= self.visible_items.saturating_sub(1) {
            self.selected_index = self
                .selected_index
                .saturating_sub(self.visible_items.saturating_sub(1));
        } else {
            self.selected_index = 0;
        }
        self.ensure_selected_visible();
    }

    pub fn page_down(&mut self) {
        if self.total_items == 0 {
            return;
        }
        let max_index = self.total_items - 1;
        let jump_size = self.visible_items.saturating_sub(1);
        self.selected_index = (self.selected_index + jump_size).min(max_index);
        self.ensure_selected_visible();
    }

    pub fn move_to_first(&mut self) {
        self.selected_index = 0;
        self.ensure_selected_visible();
    }

    pub fn move_to_last(&mut self) {
        if self.total_items > 0 {
            self.selected_index = self.total_items - 1;
        }
        self.ensure_selected_visible();
    }

    fn ensure_selected_visible(&mut self) {
        if self.selected_index < self.offset {
            self.offset = self.selected_index;
        } else if self.selected_index >= self.offset + self.visible_items {
            self.offset = self
                .selected_index
                .saturating_sub(self.visible_items.saturating_sub(1));
        }
        let max_offset = self.total_items.saturating_sub(self.visible_items);
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }

    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.offset;
        let end = (start + self.visible_items).min(self.total_items);
        (start, end)
    }

    pub fn page_info(&self) -> Option<(usize, usize)> {
        if self.total_items <= self.visible_items {
            None
        } else {
            let current_page = self.offset / self.visible_items + 1;
            let total_pages = self.total_items.div_ceil(self.visible_items);
            Some((current_page, total_pages))
        }
    }

    pub fn set_selected(&mut self, index: usize) {
        if self.total_items > 0 && index < self.total_items {
            self.selected_index = index;
            self.ensure_selected_visible();
        }
    }

    pub fn update_visible_items(&mut self, new_visible_items: usize) {
        self.visible_items = new_visible_items;
        self.ensure_selected_visible();
    }
}
