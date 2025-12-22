//! Scrolling and viewport management utilities
//!
//! Provides clean, reusable scrolling logic for all TUI components.

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
    /// Create a new scroll state
    pub fn new(total_items: usize, visible_items: usize) -> Self {
        Self {
            offset: 0,
            total_items,
            visible_items,
            selected_index: 0,
        }
    }

    /// Update visible items count (for window resize)
    pub fn update_visible_items(&mut self, new_visible_items: usize) {
        self.visible_items = new_visible_items;
        self.ensure_selected_visible();
    }

    /// Move selection up by one item
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_selected_visible();
        }
    }

    /// Move selection down by one item
    pub fn move_down(&mut self) {
        // For selection dialogs, only go up to the last item
        if self.selected_index < self.total_items {
            self.selected_index += 1;
            self.ensure_selected_visible();
        }
    }

    /// Move selection up by one page
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

    /// Move selection down by one page
    pub fn page_down(&mut self) {
        let max_index = self.total_items;
        let jump_size = self.visible_items.saturating_sub(1);
        if self.selected_index + jump_size <= max_index {
            self.selected_index = (self.selected_index + jump_size).min(max_index);
        } else {
            self.selected_index = max_index;
        }
        self.ensure_selected_visible();
    }

    /// Jump to first item
    pub fn move_to_first(&mut self) {
        self.selected_index = 0;
        self.ensure_selected_visible();
    }

    /// Jump to last item
    pub fn move_to_last(&mut self) {
        self.selected_index = self.total_items;
        self.ensure_selected_visible();
    }

    /// Ensure the selected item is visible by adjusting scroll offset
    fn ensure_selected_visible(&mut self) {
        // If selected item is above visible area, scroll up
        if self.selected_index < self.offset {
            self.offset = self.selected_index;
        }
        // If selected item is below visible area, scroll down
        else if self.selected_index >= self.offset + self.visible_items {
            self.offset = self
                .selected_index
                .saturating_sub(self.visible_items.saturating_sub(1));
        }

        // Ensure offset doesn't exceed bounds
        let max_offset = self.total_items.saturating_sub(self.visible_items.saturating_sub(1));
        if self.offset > max_offset {
            self.offset = max_offset.max(0);
        }
    }

    /// Get the range of visible items (start, end)
    pub fn visible_range(&self) -> (usize, usize) {
        let start = self.offset;
        let end = (start + self.visible_items).min(self.total_items);
        (start, end)
    }

    /// Get current page info for display
    pub fn page_info(&self) -> Option<(usize, usize)> {
        if self.total_items <= self.visible_items {
            None // No pagination needed
        } else {
            let current_page = self.offset / self.visible_items + 1;
            let total_pages = self.total_items.div_ceil(self.visible_items);
            Some((current_page, total_pages))
        }
    }

    /// Set selected index directly (useful for external updates)
    pub fn set_selected(&mut self, index: usize) {
        if index <= self.total_items {
            self.selected_index = index;
            self.ensure_selected_visible();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_state_new() {
        let state = ScrollState::new(100, 10);
        assert_eq!(state.offset, 0);
        assert_eq!(state.total_items, 100);
        assert_eq!(state.visible_items, 10);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_move_down() {
        let mut state = ScrollState::new(10, 5);
        state.move_down();
        assert_eq!(state.selected_index, 1);
        state.move_down();
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_move_up() {
        let mut state = ScrollState::new(10, 5);
        state.selected_index = 5;
        state.move_up();
        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn test_move_up_at_zero() {
        let mut state = ScrollState::new(10, 5);
        state.move_up();
        assert_eq!(state.selected_index, 0); // Should stay at 0
    }

    #[test]
    fn test_page_down() {
        let mut state = ScrollState::new(50, 10);
        state.page_down();
        assert_eq!(state.selected_index, 9); // visible_items - 1
    }

    #[test]
    fn test_page_up() {
        let mut state = ScrollState::new(50, 10);
        state.selected_index = 20;
        state.page_up();
        assert_eq!(state.selected_index, 11); // 20 - (10 - 1)
    }

    #[test]
    fn test_move_to_first() {
        let mut state = ScrollState::new(50, 10);
        state.selected_index = 25;
        state.move_to_first();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_move_to_last() {
        let mut state = ScrollState::new(50, 10);
        state.move_to_last();
        assert_eq!(state.selected_index, 50);
    }

    #[test]
    fn test_visible_range() {
        let state = ScrollState::new(50, 10);
        let (start, end) = state.visible_range();
        assert_eq!(start, 0);
        assert_eq!(end, 10);
    }

    #[test]
    fn test_visible_range_scrolled() {
        let mut state = ScrollState::new(50, 10);
        state.offset = 15;
        let (start, end) = state.visible_range();
        assert_eq!(start, 15);
        assert_eq!(end, 25);
    }

    #[test]
    fn test_page_info_no_pagination() {
        let state = ScrollState::new(5, 10);
        assert!(state.page_info().is_none());
    }

    #[test]
    fn test_page_info_with_pagination() {
        let state = ScrollState::new(50, 10);
        let info = state.page_info();
        assert!(info.is_some());
        let (current, total) = info.unwrap();
        assert_eq!(current, 1);
        assert_eq!(total, 5);
    }

    #[test]
    fn test_set_selected() {
        let mut state = ScrollState::new(50, 10);
        state.set_selected(25);
        assert_eq!(state.selected_index, 25);
    }

    #[test]
    fn test_set_selected_out_of_bounds() {
        let mut state = ScrollState::new(10, 5);
        state.set_selected(100);
        assert_eq!(state.selected_index, 0); // Should not change
    }

    #[test]
    fn test_update_visible_items() {
        let mut state = ScrollState::new(50, 10);
        state.selected_index = 25;
        state.update_visible_items(5);
        assert_eq!(state.visible_items, 5);
    }

    #[test]
    fn test_scrolling_keeps_selection_visible() {
        let mut state = ScrollState::new(50, 10);
        // Move down past visible area
        for _ in 0..15 {
            state.move_down();
        }
        // Selection should be visible
        let (start, end) = state.visible_range();
        assert!(state.selected_index >= start && state.selected_index < end);
    }
}
