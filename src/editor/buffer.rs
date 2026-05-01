pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

struct Snapshot {
    text: String,
    anchor: usize,
    head: usize,
}

pub struct Buffer {
    pub text: String,
    pub selections: Vec<Selection>,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    pub register: String,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            selections: vec![Selection { anchor: 0, head: 0 }],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            register: String::new(),
        }
    }

    fn save_snapshot(&mut self) {
        self.undo_stack.push(Snapshot {
            text: self.text.clone(),
            anchor: self.selections[0].anchor,
            head: self.selections[0].head,
        });
        self.redo_stack.clear();
    }

    fn restore_snapshot(snapshot: &Snapshot, text: &mut String, selections: &mut [Selection]) {
        *text = snapshot.text.clone();
        selections[0].anchor = snapshot.anchor;
        selections[0].head = snapshot.head;
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(Snapshot {
                text: self.text.clone(),
                anchor: self.selections[0].anchor,
                head: self.selections[0].head,
            });
            Self::restore_snapshot(&snapshot, &mut self.text, &mut self.selections);
        }
    }

    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(Snapshot {
                text: self.text.clone(),
                anchor: self.selections[0].anchor,
                head: self.selections[0].head,
            });
            Self::restore_snapshot(&snapshot, &mut self.text, &mut self.selections);
        }
    }

    /// Returns the cursor position of the primary (first) selection.
    pub fn cursor(&self) -> usize {
        self.selections[0].head
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.selections[0].anchor = pos;
        self.selections[0].head = pos;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.save_snapshot();
        let pos = self.cursor();
        self.text.insert(pos, ch);
        self.set_cursor(pos + ch.len_utf8());
    }

    /// Inserts a backslash-newline continuation with indentation.
    /// If the character before cursor is not a space, inserts a space before the backslash.
    pub fn insert_newline(&mut self) {
        self.save_snapshot();
        let pos = self.cursor();
        let needs_space = pos > 0 && self.text.as_bytes().get(pos - 1).copied() != Some(b' ');
        let insertion = if needs_space { " \\\n  " } else { "\\\n  " };
        self.text.insert_str(pos, insertion);
        self.set_cursor(pos + insertion.len());
    }

    pub fn delete_back(&mut self) {
        let pos = self.cursor();
        if pos == 0 {
            return;
        }
        self.save_snapshot();
        let prev = self.text[..pos]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.text.remove(prev);
        self.set_cursor(prev);
    }

    pub fn delete_forward(&mut self) {
        let pos = self.cursor();
        if pos >= self.text.len() {
            return;
        }
        self.save_snapshot();
        self.text.remove(pos);
    }

    pub fn move_left(&mut self) {
        let pos = self.cursor();
        if pos == 0 {
            return;
        }
        let prev = self.text[..pos]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.set_cursor(prev);
    }

    pub fn move_right(&mut self) {
        let pos = self.cursor();
        if pos >= self.text.len() {
            return;
        }
        let next = pos
            + self.text[pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        self.set_cursor(next);
    }

    pub fn move_word_forward(&mut self) {
        let pos = self.cursor();
        let bytes = self.text.as_bytes();
        let len = bytes.len();
        let mut i = pos;

        // skip current word (non-space characters)
        while i < len && bytes[i] != b' ' {
            i += 1;
        }
        // skip spaces
        while i < len && bytes[i] == b' ' {
            i += 1;
        }
        self.set_cursor(i);
    }

    pub fn move_word_back(&mut self) {
        let pos = self.cursor();
        let bytes = self.text.as_bytes();
        let mut i = pos;

        // skip spaces backward
        while i > 0 && bytes[i - 1] == b' ' {
            i -= 1;
        }
        // skip word backward
        while i > 0 && bytes[i - 1] != b' ' {
            i -= 1;
        }
        self.set_cursor(i);
    }

    pub fn move_up(&mut self) {
        let pos = self.cursor();
        let before = &self.text[..pos];
        let Some(newline_pos) = before.rfind('\n') else {
            return; // already on first line
        };
        // Column on current line
        let col = pos - newline_pos - 1;
        // Find start of previous line
        let prev_line_start = before[..newline_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let prev_line_len = newline_pos - prev_line_start;
        self.set_cursor(prev_line_start + col.min(prev_line_len));
    }

    pub fn move_down(&mut self) {
        let pos = self.cursor();
        let after = &self.text[pos..];
        let Some(newline_offset) = after.find('\n') else {
            return; // already on last line
        };
        let next_line_start = pos + newline_offset + 1;
        // Column on current line
        let current_line_start = self.text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = pos - current_line_start;
        // Length of next line
        let next_line_end = self.text[next_line_start..]
            .find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(self.text.len());
        let next_line_len = next_line_end - next_line_start;
        self.set_cursor(next_line_start + col.min(next_line_len));
    }

    pub fn move_line_start(&mut self) {
        self.set_cursor(0);
    }

    pub fn move_line_end(&mut self) {
        self.set_cursor(self.text.len());
    }

    /// Deletes the word before the cursor (like Ctrl-W in shells).
    pub fn delete_word_back(&mut self) {
        let pos = self.cursor();
        if pos == 0 {
            return;
        }
        self.save_snapshot();
        let bytes = self.text.as_bytes();
        let mut i = pos;
        // skip spaces backward
        while i > 0 && bytes[i - 1] == b' ' {
            i -= 1;
        }
        // skip word backward
        while i > 0 && bytes[i - 1] != b' ' {
            i -= 1;
        }
        self.text.drain(i..pos);
        self.set_cursor(i);
    }

    /// Deletes from cursor to line start (like Ctrl-U in shells).
    pub fn delete_to_line_start(&mut self) {
        let pos = self.cursor();
        if pos == 0 {
            return;
        }
        self.save_snapshot();
        self.text.drain(0..pos);
        self.set_cursor(0);
    }

    /// Clears the entire buffer.
    pub fn clear(&mut self) {
        if self.text.is_empty() {
            return;
        }
        self.save_snapshot();
        self.text.clear();
        self.set_cursor(0);
    }

    /// Pastes text at cursor position.
    pub fn paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.save_snapshot();
        let pos = self.cursor();
        self.text.insert_str(pos, text);
        self.set_cursor(pos + text.len());
    }

    /// Copies the selected text (between anchor and head). Returns the copied text.
    pub fn copy_selection(&self) -> Option<String> {
        if !self.has_selection() {
            return None;
        }
        let (start, end) = self.selection_range();
        Some(self.text[start..end].to_string())
    }

    /// Yanks (copies) the selection into the internal register.
    pub fn yank(&mut self) {
        if !self.has_selection() {
            return;
        }
        let (start, end) = self.selection_range();
        self.register = self.text[start..end].to_string();
    }

    /// Yanks the selection into the register, then deletes it.
    /// If no selection, yanks and deletes the char at cursor.
    pub fn yank_delete(&mut self) {
        if !self.has_selection() {
            let pos = self.cursor();
            if pos >= self.text.len() {
                return;
            }
            let char_end = self.char_end_at(pos);
            self.register = self.text[pos..char_end].to_string();
            self.save_snapshot();
            self.text.drain(pos..char_end);
            return;
        }
        self.yank();
        self.save_snapshot();
        let (start, end) = self.selection_range();
        self.text.drain(start..end);
        self.set_cursor(start);
    }

    /// Pastes the internal register at cursor position.
    pub fn paste_register(&mut self) {
        if self.register.is_empty() {
            return;
        }
        self.save_snapshot();
        let pos = self.cursor();
        let reg = self.register.clone();
        self.text.insert_str(pos, &reg);
        self.set_cursor(pos + reg.len());
    }

    /// Selects the entire current line (from line start to line end).
    pub fn select_line(&mut self) {
        let pos = self.cursor();
        let line_start = self.text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = self.text[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(self.text.len());
        self.selections[0].anchor = line_start;
        self.selections[0].head = line_end;
    }

    // --- Visual mode extend (anchor stays, head moves) ---

    pub fn extend_left(&mut self) {
        let pos = self.cursor();
        if pos == 0 {
            return;
        }
        let prev = self.text[..pos]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.move_head(prev);
    }

    pub fn extend_right(&mut self) {
        let pos = self.cursor();
        if pos >= self.text.len() {
            return;
        }
        let next = self.char_end_at(pos);
        self.move_head(next);
    }

    pub fn extend_up(&mut self) {
        let pos = self.cursor();
        let before = &self.text[..pos];
        let Some(newline_pos) = before.rfind('\n') else {
            return;
        };
        let col = pos - newline_pos - 1;
        let prev_line_start = before[..newline_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let prev_line_len = newline_pos - prev_line_start;
        self.move_head(prev_line_start + col.min(prev_line_len));
    }

    pub fn extend_down(&mut self) {
        let pos = self.cursor();
        let after = &self.text[pos..];
        let Some(newline_offset) = after.find('\n') else {
            return;
        };
        let next_line_start = pos + newline_offset + 1;
        let current_line_start = self.text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = pos - current_line_start;
        let next_line_end = self.text[next_line_start..]
            .find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(self.text.len());
        let next_line_len = next_line_end - next_line_start;
        self.move_head(next_line_start + col.min(next_line_len));
    }

    pub fn extend_word_forward(&mut self) {
        let pos = self.cursor();
        let bytes = self.text.as_bytes();
        let len = bytes.len();
        let mut i = pos;
        while i < len && bytes[i] != b' ' {
            i += 1;
        }
        while i < len && bytes[i] == b' ' {
            i += 1;
        }
        self.move_head(i);
    }

    pub fn extend_word_back(&mut self) {
        let pos = self.cursor();
        let bytes = self.text.as_bytes();
        let mut i = pos;
        while i > 0 && bytes[i - 1] == b' ' {
            i -= 1;
        }
        while i > 0 && bytes[i - 1] != b' ' {
            i -= 1;
        }
        self.move_head(i);
    }

    pub fn extend_line_start(&mut self) {
        self.move_head(0);
    }

    pub fn extend_line_end(&mut self) {
        self.move_head(self.text.len());
    }

    // --- Selection-aware operations ---

    fn move_head(&mut self, pos: usize) {
        self.selections[0].head = pos;
    }

    pub fn has_selection(&self) -> bool {
        self.selections[0].anchor != self.selections[0].head
    }

    /// Returns (start, end) of the primary selection, normalized so start <= end.
    pub fn selection_range(&self) -> (usize, usize) {
        let a = self.selections[0].anchor;
        let h = self.selections[0].head;
        (a.min(h), a.max(h))
    }

    pub fn collapse_selection(&mut self) {
        self.selections[0].anchor = self.selections[0].head;
    }

    fn reset_anchor_to_head(&mut self) {
        self.selections[0].anchor = self.selections[0].head;
    }

    fn char_end_at(&self, pos: usize) -> usize {
        pos + self.text[pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0)
    }

    pub fn select_left(&mut self) {
        let pos = self.cursor();
        if pos == 0 {
            return;
        }
        let prev = self.text[..pos]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.selections[0].head = prev;
        self.selections[0].anchor = self.char_end_at(prev);
    }

    pub fn select_right(&mut self) {
        let pos = self.cursor();
        if pos >= self.text.len() {
            return;
        }
        let next = self.char_end_at(pos);
        if next >= self.text.len() {
            self.set_cursor(next);
            return;
        }
        self.selections[0].head = next;
        self.selections[0].anchor = self.char_end_at(next);
    }

    pub fn select_word_forward(&mut self) {
        self.reset_anchor_to_head();
        let pos = self.selections[0].head;
        let bytes = self.text.as_bytes();
        let len = bytes.len();
        let mut i = pos;
        while i < len && bytes[i] != b' ' {
            i += 1;
        }
        while i < len && bytes[i] == b' ' {
            i += 1;
        }
        self.move_head(i);
    }

    pub fn select_word_back(&mut self) {
        self.reset_anchor_to_head();
        let pos = self.selections[0].head;
        let bytes = self.text.as_bytes();
        let mut i = pos;
        while i > 0 && bytes[i - 1] == b' ' {
            i -= 1;
        }
        while i > 0 && bytes[i - 1] != b' ' {
            i -= 1;
        }
        self.move_head(i);
    }

    pub fn delete_selection(&mut self) {
        let (start, end) = self.selection_range();
        if start == end {
            // No selection: delete char at cursor (like Helix 1-char selection)
            self.delete_forward();
            return;
        }
        self.save_snapshot();
        self.text.drain(start..end);
        self.set_cursor(start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that a new buffer starts empty with cursor at position 0.
    #[test]
    fn new_buffer_is_empty() {
        let buf = Buffer::new();
        assert!(buf.text.is_empty());
        assert_eq!(buf.cursor(), 0);
    }

    // Test that inserting a character at cursor position works.
    // Given: empty buffer
    // When: insert 'a'
    // Then: text is "a", cursor is at 1
    #[test]
    fn insert_char() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        assert_eq!(buf.text, "a");
        assert_eq!(buf.cursor(), 1);
    }

    // Test that inserting multiple characters builds up text correctly.
    // Given: empty buffer
    // When: insert 'a', 'b', 'c'
    // Then: text is "abc", cursor is at 3
    #[test]
    fn insert_multiple_chars() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.insert_char('c');
        assert_eq!(buf.text, "abc");
        assert_eq!(buf.cursor(), 3);
    }

    // Test that deleting backward removes the character before cursor.
    // Given: buffer with "ab", cursor at 2
    // When: delete_back
    // Then: text is "a", cursor is at 1
    #[test]
    fn delete_back() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.delete_back();
        assert_eq!(buf.text, "a");
        assert_eq!(buf.cursor(), 1);
    }

    // Test that deleting backward at position 0 does nothing.
    #[test]
    fn delete_back_at_start() {
        let mut buf = Buffer::new();
        buf.delete_back();
        assert!(buf.text.is_empty());
        assert_eq!(buf.cursor(), 0);
    }

    // Test that move_left decrements cursor by 1.
    // Given: buffer with "ab", cursor at 2
    // When: move_left
    // Then: cursor is at 1
    #[test]
    fn move_left() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.move_left();
        assert_eq!(buf.cursor(), 1);
    }

    // Test that move_left at position 0 stays at 0.
    #[test]
    fn move_left_at_start() {
        let mut buf = Buffer::new();
        buf.move_left();
        assert_eq!(buf.cursor(), 0);
    }

    // Test that move_right increments cursor by 1.
    // Given: buffer with "ab", cursor at 0
    // When: move_right
    // Then: cursor is at 1
    #[test]
    fn move_right() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.move_left();
        buf.move_left();
        buf.move_right();
        assert_eq!(buf.cursor(), 1);
    }

    // Test that move_right at end of text stays at end.
    #[test]
    fn move_right_at_end() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.move_right();
        assert_eq!(buf.cursor(), 1);
    }

    // Test that move_word_forward jumps to the start of the next word.
    // Given: buffer with "hello world", cursor at 0
    // When: move_word_forward
    // Then: cursor is at 6 (start of "world")
    #[test]
    fn move_word_forward() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.move_word_forward();
        assert_eq!(buf.cursor(), 6);
    }

    // Test that move_word_forward from middle of word jumps to next word.
    // Given: buffer with "hello world", cursor at 2
    // When: move_word_forward
    // Then: cursor is at 6
    #[test]
    fn move_word_forward_from_middle() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.selections[0].head = 2;
        buf.selections[0].anchor = 2;
        buf.move_word_forward();
        assert_eq!(buf.cursor(), 6);
    }

    // Test that move_word_back jumps to the start of the previous word.
    // Given: buffer with "hello world", cursor at 8
    // When: move_word_back
    // Then: cursor is at 6 (start of "world")
    #[test]
    fn move_word_back() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.selections[0].head = 8;
        buf.selections[0].anchor = 8;
        buf.move_word_back();
        assert_eq!(buf.cursor(), 6);
    }

    // Test that move_word_back from start of second word goes to start of first.
    // Given: buffer with "hello world", cursor at 6
    // When: move_word_back
    // Then: cursor is at 0
    #[test]
    fn move_word_back_to_start() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.selections[0].head = 6;
        buf.selections[0].anchor = 6;
        buf.move_word_back();
        assert_eq!(buf.cursor(), 0);
    }

    // Test that move_line_start moves cursor to position 0.
    #[test]
    fn move_line_start() {
        let mut buf = Buffer::new();
        buf.text = "hello".to_string();
        buf.selections[0].head = 3;
        buf.selections[0].anchor = 3;
        buf.move_line_start();
        assert_eq!(buf.cursor(), 0);
    }

    // Test that move_line_end moves cursor to end of text.
    #[test]
    fn move_line_end() {
        let mut buf = Buffer::new();
        buf.text = "hello".to_string();
        buf.move_line_end();
        assert_eq!(buf.cursor(), 5);
    }

    // Test that delete_forward removes the character at cursor.
    // Given: buffer with "abc", cursor at 1
    // When: delete_forward
    // Then: text is "ac", cursor stays at 1
    #[test]
    fn delete_forward() {
        let mut buf = Buffer::new();
        buf.text = "abc".to_string();
        buf.selections[0].head = 1;
        buf.selections[0].anchor = 1;
        buf.delete_forward();
        assert_eq!(buf.text, "ac");
        assert_eq!(buf.cursor(), 1);
    }

    // Test that delete_forward at end of text does nothing.
    #[test]
    fn delete_forward_at_end() {
        let mut buf = Buffer::new();
        buf.text = "abc".to_string();
        buf.selections[0].head = 3;
        buf.selections[0].anchor = 3;
        buf.delete_forward();
        assert_eq!(buf.text, "abc");
        assert_eq!(buf.cursor(), 3);
    }

    // --- Selection-aware operations (for Helix mode) ---

    // Test that select_right creates 1-char selection at the destination.
    // Given: buffer with "abc", cursor at 0
    // When: select_right
    // Then: head=1, anchor=2 (1-char selection on 'b')
    #[test]
    fn select_right() {
        let mut buf = Buffer::new();
        buf.text = "abc".to_string();
        buf.select_right();
        assert_eq!(buf.selections[0].head, 1);
        assert_eq!(buf.selections[0].anchor, 2);
        assert_eq!(buf.selection_range(), (1, 2));
    }

    // Test that select_left creates 1-char selection at the destination.
    // Given: buffer with "abc", cursor at 2
    // When: select_left
    // Then: head=1, anchor=2 (1-char selection on 'b')
    #[test]
    fn select_left() {
        let mut buf = Buffer::new();
        buf.text = "abc".to_string();
        buf.set_cursor(2);
        buf.select_left();
        assert_eq!(buf.selections[0].head, 1);
        assert_eq!(buf.selections[0].anchor, 2);
        assert_eq!(buf.selection_range(), (1, 2));
    }

    // Test that select_word_forward moves head to next word boundary.
    // Given: buffer with "hello world", anchor=0, head=0
    // When: select_word_forward
    // Then: anchor=0, head=6
    #[test]
    fn select_word_forward() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.select_word_forward();
        assert_eq!(buf.selections[0].anchor, 0);
        assert_eq!(buf.selections[0].head, 6);
    }

    // Test that select_word_back resets anchor to head, then moves head back.
    // Given: buffer with "hello world", anchor=3, head=8
    // When: select_word_back
    // Then: anchor=8, head=6
    #[test]
    fn select_word_back() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.selections[0].anchor = 3;
        buf.selections[0].head = 8;
        buf.select_word_back();
        assert_eq!(buf.selections[0].anchor, 8);
        assert_eq!(buf.selections[0].head, 6);
    }

    // Test that selection_range returns (start, end) regardless of direction.
    // Forward selection: anchor=1, head=4 → (1, 4)
    #[test]
    fn selection_range_forward() {
        let mut buf = Buffer::new();
        buf.text = "abcdef".to_string();
        buf.selections[0].anchor = 1;
        buf.selections[0].head = 4;
        assert_eq!(buf.selection_range(), (1, 4));
    }

    // Backward selection: anchor=4, head=1 → (1, 4)
    #[test]
    fn selection_range_backward() {
        let mut buf = Buffer::new();
        buf.text = "abcdef".to_string();
        buf.selections[0].anchor = 4;
        buf.selections[0].head = 1;
        assert_eq!(buf.selection_range(), (1, 4));
    }

    // Test that delete_selection removes selected text.
    // Given: buffer with "hello world", anchor=0, head=6
    // When: delete_selection
    // Then: text is "world", cursor at 0
    #[test]
    fn delete_selection() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.selections[0].anchor = 0;
        buf.selections[0].head = 6;
        buf.delete_selection();
        assert_eq!(buf.text, "world");
        assert_eq!(buf.cursor(), 0);
        assert_eq!(buf.selections[0].anchor, 0);
    }

    // Test that delete_selection with backward selection works.
    // Given: buffer with "hello world", anchor=6, head=0
    // When: delete_selection
    // Then: text is "world", cursor at 0
    #[test]
    fn delete_selection_backward() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.selections[0].anchor = 6;
        buf.selections[0].head = 0;
        buf.delete_selection();
        assert_eq!(buf.text, "world");
        assert_eq!(buf.cursor(), 0);
    }

    // Test that collapse_selection sets anchor = head.
    // Given: anchor=0, head=5
    // When: collapse_selection
    // Then: anchor=5, head=5
    #[test]
    fn collapse_selection() {
        let mut buf = Buffer::new();
        buf.text = "hello".to_string();
        buf.selections[0].anchor = 0;
        buf.selections[0].head = 5;
        buf.collapse_selection();
        assert_eq!(buf.selections[0].anchor, 5);
        assert_eq!(buf.selections[0].head, 5);
    }

    // Test that has_selection returns true when anchor != head.
    #[test]
    fn has_selection_true() {
        let mut buf = Buffer::new();
        buf.text = "abc".to_string();
        buf.selections[0].anchor = 0;
        buf.selections[0].head = 2;
        assert!(buf.has_selection());
    }

    // Test that has_selection returns false when anchor == head.
    #[test]
    fn has_selection_false() {
        let buf = Buffer::new();
        assert!(!buf.has_selection());
    }

    // Test that pressing w twice selects only the second word, not both.
    // Given: "hello world", cursor at 0
    // When: select_word_forward twice
    // Then: first: anchor=0, head=6 ("hello " selected)
    //       second: anchor=6, head=11 ("world" selected)
    #[test]
    fn select_word_forward_twice_selects_second_word() {
        let mut buf = Buffer::new();
        buf.text = "hello world".to_string();
        buf.select_word_forward();
        assert_eq!(buf.selections[0].anchor, 0);
        assert_eq!(buf.selections[0].head, 6);
        buf.select_word_forward();
        assert_eq!(buf.selections[0].anchor, 6);
        assert_eq!(buf.selections[0].head, 11);
    }

    // Test that pressing h twice moves the 1-char selection left each time.
    // Given: "abc", cursor at 2
    // When: select_left twice
    // Then: first: head=1, anchor=2 (on 'b')
    //       second: head=0, anchor=1 (on 'a')
    #[test]
    fn select_left_twice() {
        let mut buf = Buffer::new();
        buf.text = "abc".to_string();
        buf.set_cursor(2);
        buf.select_left();
        assert_eq!(buf.selections[0].head, 1);
        assert_eq!(buf.selections[0].anchor, 2);
        buf.select_left();
        assert_eq!(buf.selections[0].head, 0);
        assert_eq!(buf.selections[0].anchor, 1);
    }

    // --- Undo/Redo ---

    // Test that undo restores previous state after insert.
    // Given: empty buffer, insert 'a', insert 'b'
    // When: undo
    // Then: text is "a", cursor at 1
    #[test]
    fn undo_after_insert() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        assert_eq!(buf.text, "a");
        assert_eq!(buf.cursor(), 1);
    }

    // Test that undo twice restores to empty.
    #[test]
    fn undo_twice() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.undo();
        assert!(buf.text.is_empty());
        assert_eq!(buf.cursor(), 0);
    }

    // Test that undo on empty buffer does nothing.
    #[test]
    fn undo_on_empty() {
        let mut buf = Buffer::new();
        buf.undo();
        assert!(buf.text.is_empty());
    }

    // Test that redo restores undone state.
    // Given: insert 'a', 'b', undo
    // When: redo
    // Then: text is "ab", cursor at 2
    #[test]
    fn redo_after_undo() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.redo();
        assert_eq!(buf.text, "ab");
        assert_eq!(buf.cursor(), 2);
    }

    // Test that redo does nothing when there's nothing to redo.
    #[test]
    fn redo_without_undo() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.redo();
        assert_eq!(buf.text, "a");
    }

    // Test that a new edit after undo clears redo history.
    // Given: insert 'a', 'b', undo, insert 'c'
    // When: redo
    // Then: text stays "ac" (redo history was cleared)
    #[test]
    fn edit_after_undo_clears_redo() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.insert_char('c');
        buf.redo();
        assert_eq!(buf.text, "ac");
    }

    // Test that undo restores state after delete_back.
    #[test]
    fn undo_after_delete() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.delete_back();
        assert_eq!(buf.text, "a");
        buf.undo();
        assert_eq!(buf.text, "ab");
    }

    // --- Insert newline ---

    // Test that insert_newline adds " \\\n  " when previous char is not space.
    // Given: "echo hello", cursor at 10
    // When: insert_newline
    // Then: "echo hello \\\n  ", cursor at 15
    #[test]
    fn insert_newline_with_space() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.set_cursor(10);
        buf.insert_newline();
        assert_eq!(buf.text, "echo hello \\\n  ");
        assert_eq!(buf.cursor(), 15);
    }

    // Test that insert_newline adds "\\\n  " when previous char is a space.
    // Given: "echo ", cursor at 5
    // When: insert_newline
    // Then: "echo \\\n  ", cursor at 9
    #[test]
    fn insert_newline_already_space() {
        let mut buf = Buffer::new();
        buf.text = "echo ".to_string();
        buf.set_cursor(5);
        buf.insert_newline();
        assert_eq!(buf.text, "echo \\\n  ");
        assert_eq!(buf.cursor(), 9);
    }

    // Test insert_newline at empty buffer.
    #[test]
    fn insert_newline_empty() {
        let mut buf = Buffer::new();
        buf.insert_newline();
        assert_eq!(buf.text, "\\\n  ");
        assert_eq!(buf.cursor(), 4);
    }

    // Test insert_newline in the middle of text.
    // Given: "echo hello world", cursor at 10 (between "hello" and " world")
    // When: insert_newline
    // Then: "echo hello \\\n   world" (space + backslash + newline + 2 spaces + original " world")
    #[test]
    fn insert_newline_middle() {
        let mut buf = Buffer::new();
        buf.text = "echo hello world".to_string();
        buf.set_cursor(10);
        buf.insert_newline();
        assert_eq!(buf.text, "echo hello \\\n   world");
    }

    // --- Move up/down ---

    // Test move_down from first line to second line, same column.
    // Given: "aaa\nbbb", cursor at 1 (on first line, col 1)
    // When: move_down
    // Then: cursor at 5 (second line, col 1)
    #[test]
    fn move_down_basic() {
        let mut buf = Buffer::new();
        buf.text = "aaa\nbbb".to_string();
        buf.set_cursor(1);
        buf.move_down();
        assert_eq!(buf.cursor(), 5);
    }

    // Test move_down clamps to shorter line.
    // Given: "abcdef\nab", cursor at 5 (first line, col 5)
    // When: move_down
    // Then: cursor at 9 (second line, col 2 = end of "ab")
    #[test]
    fn move_down_clamp() {
        let mut buf = Buffer::new();
        buf.text = "abcdef\nab".to_string();
        buf.set_cursor(5);
        buf.move_down();
        assert_eq!(buf.cursor(), 9);
    }

    // Test move_down on last line does nothing.
    #[test]
    fn move_down_last_line() {
        let mut buf = Buffer::new();
        buf.text = "aaa\nbbb".to_string();
        buf.set_cursor(5);
        buf.move_down();
        assert_eq!(buf.cursor(), 5);
    }

    // Test move_up from second line to first line, same column.
    // Given: "aaa\nbbb", cursor at 5 (second line, col 1)
    // When: move_up
    // Then: cursor at 1 (first line, col 1)
    #[test]
    fn move_up_basic() {
        let mut buf = Buffer::new();
        buf.text = "aaa\nbbb".to_string();
        buf.set_cursor(5);
        buf.move_up();
        assert_eq!(buf.cursor(), 1);
    }

    // Test move_up clamps to shorter line.
    // Given: "ab\nabcdef", cursor at 8 (second line, col 5)
    // When: move_up
    // Then: cursor at 2 (first line, col 2 = end of "ab")
    #[test]
    fn move_up_clamp() {
        let mut buf = Buffer::new();
        buf.text = "ab\nabcdef".to_string();
        buf.set_cursor(8);
        buf.move_up();
        assert_eq!(buf.cursor(), 2);
    }

    // Test move_up on first line does nothing.
    #[test]
    fn move_up_first_line() {
        let mut buf = Buffer::new();
        buf.text = "aaa\nbbb".to_string();
        buf.set_cursor(1);
        buf.move_up();
        assert_eq!(buf.cursor(), 1);
    }

    // --- Editing commands ---

    // Test delete_word_back removes the word before cursor.
    // Given: "echo hello", cursor at 10
    // When: delete_word_back
    // Then: "echo ", cursor at 5
    #[test]
    fn delete_word_back() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.set_cursor(10);
        buf.delete_word_back();
        assert_eq!(buf.text, "echo ");
        assert_eq!(buf.cursor(), 5);
    }

    // Test delete_word_back at start does nothing.
    #[test]
    fn delete_word_back_at_start() {
        let mut buf = Buffer::new();
        buf.text = "hello".to_string();
        buf.set_cursor(0);
        buf.delete_word_back();
        assert_eq!(buf.text, "hello");
    }

    // Test delete_to_line_start removes everything before cursor.
    // Given: "echo hello", cursor at 5
    // When: delete_to_line_start
    // Then: "hello", cursor at 0
    #[test]
    fn delete_to_line_start() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.set_cursor(5);
        buf.delete_to_line_start();
        assert_eq!(buf.text, "hello");
        assert_eq!(buf.cursor(), 0);
    }

    // Test clear empties the buffer.
    #[test]
    fn clear_buffer() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.set_cursor(5);
        buf.clear();
        assert!(buf.text.is_empty());
        assert_eq!(buf.cursor(), 0);
    }

    // Test paste inserts text at cursor.
    // Given: "echo ", cursor at 5
    // When: paste "hello"
    // Then: "echo hello", cursor at 10
    #[test]
    fn paste_text() {
        let mut buf = Buffer::new();
        buf.text = "echo ".to_string();
        buf.set_cursor(5);
        buf.paste("hello");
        assert_eq!(buf.text, "echo hello");
        assert_eq!(buf.cursor(), 10);
    }

    // Test copy_selection returns selected text.
    #[test]
    fn copy_selection_text() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.selections[0].anchor = 5;
        buf.selections[0].head = 10;
        assert_eq!(buf.copy_selection(), Some("hello".to_string()));
    }

    // Test copy_selection returns None when no selection.
    #[test]
    fn copy_selection_none() {
        let buf = Buffer::new();
        assert!(buf.copy_selection().is_none());
    }

    // --- Yank/Paste/SelectLine ---

    // Test yank copies selection to register.
    #[test]
    fn yank_selection() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.selections[0].anchor = 5;
        buf.selections[0].head = 10;
        buf.yank();
        assert_eq!(buf.register, "hello");
        // Text is unchanged
        assert_eq!(buf.text, "echo hello");
    }

    // Test yank_delete copies to register and deletes.
    #[test]
    fn yank_delete_selection() {
        let mut buf = Buffer::new();
        buf.text = "echo hello".to_string();
        buf.selections[0].anchor = 5;
        buf.selections[0].head = 10;
        buf.yank_delete();
        assert_eq!(buf.register, "hello");
        assert_eq!(buf.text, "echo ");
        assert_eq!(buf.cursor(), 5);
    }

    // Test paste_register inserts register at cursor.
    #[test]
    fn paste_from_register() {
        let mut buf = Buffer::new();
        buf.text = "echo ".to_string();
        buf.set_cursor(5);
        buf.register = "world".to_string();
        buf.paste_register();
        assert_eq!(buf.text, "echo world");
        assert_eq!(buf.cursor(), 10);
    }

    // Test paste_register does nothing when register is empty.
    #[test]
    fn paste_empty_register() {
        let mut buf = Buffer::new();
        buf.text = "echo".to_string();
        buf.set_cursor(4);
        buf.paste_register();
        assert_eq!(buf.text, "echo");
    }

    // Test select_line selects the entire current line.
    // Given: "aaa\nbbb\nccc", cursor at 5 (on 'b')
    // When: select_line
    // Then: anchor=4, head=7 (selects "bbb")
    #[test]
    fn select_line_middle() {
        let mut buf = Buffer::new();
        buf.text = "aaa\nbbb\nccc".to_string();
        buf.set_cursor(5);
        buf.select_line();
        assert_eq!(buf.selections[0].anchor, 4);
        assert_eq!(buf.selections[0].head, 7);
        assert_eq!(&buf.text[4..7], "bbb");
    }

    // Test select_line on single line.
    #[test]
    fn select_line_single() {
        let mut buf = Buffer::new();
        buf.text = "hello".to_string();
        buf.set_cursor(2);
        buf.select_line();
        assert_eq!(buf.selections[0].anchor, 0);
        assert_eq!(buf.selections[0].head, 5);
    }
}
