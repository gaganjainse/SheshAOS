/// Full VT100/ANSI terminal emulator backed by the `vte` parser crate.
/// Implements a proper cell-grid model with cursor, scrollback, and ANSI
/// attribute tracking (color, bold, italic, underline, reverse).
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use vte::{Params, Parser, Perform};

use iced::keyboard;
use tokio::sync::mpsc;

// --- Grid geometry constants ---
pub const DEFAULT_ROWS: usize = 30;
pub const DEFAULT_COLS: usize = 120;
/// Approximate monospace cell width at 14px.
pub const CELL_W: f32 = 8.4;
/// Line height at 14px with 1.2 line spacing.
pub const CELL_H: f32 = 17.0;
/// PTY read chunk size (64KB) - yield lock after each chunk to prevent GUI starvation
const PTY_READ_CHUNK: usize = 64 * 1024;
/// Maximum buffer size before applying backpressure
const PTY_MAX_BUFFER: usize = 1024 * 1024; // 1MB

// ──────────────────────────────────────────────────────────────────────────────
// Cell types
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TermColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CellAttr {
    pub fg: TermColor,
    pub bg: TermColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub strikethrough: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub attr: CellAttr,
}

impl Default for Cell {
    fn default() -> Self {
        Self { ch: ' ', attr: CellAttr::default() }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// VTE performer — owns the terminal grid and implements vte::Perform callbacks
// ──────────────────────────────────────────────────────────────────────────────

pub struct TermPerformer {
    /// Active screen grid [row][col].
    pub grid: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub current_attr: CellAttr,
    pub rows: usize,
    pub cols: usize,
    /// Scrollback buffer (oldest line first).
    pub scrollback: VecDeque<Vec<Cell>>,
    pub max_scrollback: usize,
    /// Terminal title set via OSC 0/1/2.
    pub title: String,
    /// Scroll region (0-indexed, inclusive).
    scroll_top: usize,
    scroll_bot: usize,
    /// Saved cursor/attr for ESC 7 / ESC 8 / CSI s / CSI u.
    saved_cursor: (usize, usize),
    saved_attr: CellAttr,
    /// Dirty line tracking for optimized rendering.
    /// A line is dirty if any cell in it has changed since last render.
    pub dirty_lines: Vec<bool>,
}

impl TermPerformer {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            current_attr: CellAttr::default(),
            rows,
            cols,
            scrollback: VecDeque::new(),
            max_scrollback: 5000,
            title: String::from("bash"),
            scroll_top: 0,
            scroll_bot: rows.saturating_sub(1),
            saved_cursor: (0, 0),
            saved_attr: CellAttr::default(),
            dirty_lines: vec![true; rows], // Initially all lines are dirty
        }
    }

    /// Mark a line as dirty (needs re-rendering).
    #[inline]
    fn mark_dirty(&mut self, row: usize) {
        if row < self.dirty_lines.len() {
            self.dirty_lines[row] = true;
        }
    }

    /// Mark a range of lines as dirty.
    fn mark_dirty_range(&mut self, start: usize, end: usize) {
        for row in start..=end.min(self.rows - 1) {
            self.dirty_lines[row] = true;
        }
    }

    /// Clear dirty flags after rendering.
    pub fn clear_dirty(&mut self) {
        self.dirty_lines.fill(false);
    }

    /// Resize the terminal grid to new dimensions.
    /// Preserves existing content where possible, truncates or extends as needed.
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        // Truncate or extend rows
        self.grid.truncate(new_rows);
        while self.grid.len() < new_rows {
            self.grid.push(vec![Cell::default(); new_cols]);
        }
        // Resize each row's columns
        for row in &mut self.grid {
            row.truncate(new_cols);
            while row.len() < new_cols {
                row.push(Cell::default());
            }
        }
        self.rows = new_rows;
        self.cols = new_cols;
        self.scroll_bot = new_rows.saturating_sub(1);
        self.dirty_lines = vec![true; new_rows];
        // Clamp cursor
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }

    // ── Scroll region helpers ───────────────────────────────────────────────

    /// Scroll the scroll region up by `n` lines, pushing ejected rows to
    /// the scrollback buffer (only when scroll_top == 0).
    fn scroll_up(&mut self, n: usize) {
        for _ in 0..n {
            let removed = self.grid.remove(self.scroll_top);
            if self.scroll_top == 0 {
                self.scrollback.push_back(removed);
                if self.scrollback.len() > self.max_scrollback {
                    self.scrollback.pop_front();
                }
            }
            let insert_at = self.scroll_bot.min(self.grid.len());
            self.grid.insert(insert_at, vec![Cell::default(); self.cols]);
        }
        // All lines in scroll region are now dirty
        self.mark_dirty_range(self.scroll_top, self.scroll_bot);
    }

    /// Scroll the scroll region down by `n` lines (inserts blank rows at top).
    fn scroll_down(&mut self, n: usize) {
        for _ in 0..n {
            let remove_at = self.scroll_bot.min(self.grid.len().saturating_sub(1));
            self.grid.remove(remove_at);
            self.grid.insert(self.scroll_top, vec![Cell::default(); self.cols]);
        }
        // All lines in scroll region are now dirty
        self.mark_dirty_range(self.scroll_top, self.scroll_bot);
    }

    // ── Erase helpers ───────────────────────────────────────────────────────

    fn erase_in_display(&mut self, mode: u16) {
        let (r, c) = (self.cursor_row, self.cursor_col);
        match mode {
            0 => {
                // Cursor to end of screen
                for col in c..self.cols {
                    self.grid[r][col] = Cell::default();
                }
                for row in (r + 1)..self.rows {
                    self.grid[row] = vec![Cell::default(); self.cols];
                }
                self.mark_dirty_range(r, self.rows - 1);
            }
            1 => {
                // Beginning of screen to cursor
                for row in 0..r {
                    self.grid[row] = vec![Cell::default(); self.cols];
                }
                for col in 0..=c.min(self.cols - 1) {
                    self.grid[r][col] = Cell::default();
                }
                self.mark_dirty_range(0, r);
            }
            2 | 3 => {
                // Entire screen
                for row in &mut self.grid {
                    *row = vec![Cell::default(); self.cols];
                }
                if mode == 2 {
                    self.cursor_row = 0;
                    self.cursor_col = 0;
                }
                self.mark_dirty_range(0, self.rows - 1);
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: u16) {
        let (r, c) = (self.cursor_row.min(self.rows - 1), self.cursor_col);
        match mode {
            0 => {
                for col in c..self.cols {
                    self.grid[r][col] = Cell::default();
                }
            }
            1 => {
                for col in 0..=c.min(self.cols - 1) {
                    self.grid[r][col] = Cell::default();
                }
            }
            2 => {
                self.grid[r] = vec![Cell::default(); self.cols];
            }
            _ => {}
        }
        self.mark_dirty(r);
    }

    // ── SGR attribute parsing ───────────────────────────────────────────────

    fn apply_sgr(&mut self, params: &Params) {
        // Flatten all subparameters into a single list so both
        // `38;5;n` (separate) and `38:5:n` (colon subparams) work.
        let ps: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();

        if ps.is_empty() {
            self.current_attr = CellAttr::default();
            return;
        }

        let mut i = 0;
        while i < ps.len() {
            match ps[i] {
                0 => self.current_attr = CellAttr::default(),
                1 => self.current_attr.bold = true,
                2 => self.current_attr.dim = true,
                3 => self.current_attr.italic = true,
                4 => self.current_attr.underline = true,
                7 => self.current_attr.reverse = true,
                9 => self.current_attr.strikethrough = true,
                22 => {
                    self.current_attr.bold = false;
                    self.current_attr.dim = false;
                }
                23 => self.current_attr.italic = false,
                24 => self.current_attr.underline = false,
                27 => self.current_attr.reverse = false,
                29 => self.current_attr.strikethrough = false,
                30..=37 => self.current_attr.fg = TermColor::Indexed(ps[i] as u8 - 30),
                38 => {
                    if i + 2 < ps.len() && ps[i + 1] == 5 {
                        self.current_attr.fg = TermColor::Indexed(ps[i + 2] as u8);
                        i += 2;
                    } else if i + 4 < ps.len() && ps[i + 1] == 2 {
                        self.current_attr.fg =
                            TermColor::Rgb(ps[i + 2] as u8, ps[i + 3] as u8, ps[i + 4] as u8);
                        i += 4;
                    }
                }
                39 => self.current_attr.fg = TermColor::Default,
                40..=47 => self.current_attr.bg = TermColor::Indexed(ps[i] as u8 - 40),
                48 => {
                    if i + 2 < ps.len() && ps[i + 1] == 5 {
                        self.current_attr.bg = TermColor::Indexed(ps[i + 2] as u8);
                        i += 2;
                    } else if i + 4 < ps.len() && ps[i + 1] == 2 {
                        self.current_attr.bg =
                            TermColor::Rgb(ps[i + 2] as u8, ps[i + 3] as u8, ps[i + 4] as u8);
                        i += 4;
                    }
                }
                49 => self.current_attr.bg = TermColor::Default,
                // Bright foreground (90–97) → indexed 8–15
                90..=97 => self.current_attr.fg = TermColor::Indexed(ps[i] as u8 - 90 + 8),
                // Bright background (100–107) → indexed 8–15
                100..=107 => self.current_attr.bg = TermColor::Indexed(ps[i] as u8 - 100 + 8),
                _ => {}
            }
            i += 1;
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// vte::Perform implementation
// ──────────────────────────────────────────────────────────────────────────────

impl Perform for TermPerformer {
    fn print(&mut self, c: char) {
        // Auto-wrap
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            if self.cursor_row >= self.scroll_bot {
                self.scroll_up(1);
            } else {
                self.cursor_row += 1;
            }
        }
        // Clamp
        let row = self.cursor_row.min(self.rows - 1);
        let col = self.cursor_col.min(self.cols - 1);

        // Apply reverse video to stored attributes
        let attr = if self.current_attr.reverse {
            CellAttr { fg: self.current_attr.bg, bg: self.current_attr.fg, ..self.current_attr }
        } else {
            self.current_attr
        };

        self.grid[row][col] = Cell { ch: c, attr };
        self.mark_dirty(row);
        self.cursor_col += 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // Line feed / vertical tab / form feed
            b'\n' | b'\x0B' | b'\x0C' => {
                if self.cursor_row >= self.scroll_bot {
                    self.scroll_up(1);
                } else {
                    self.cursor_row += 1;
                }
            }
            b'\r' => {
                self.cursor_col = 0;
            }
            b'\x08' => {
                // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            b'\x07' => { /* Bell — ignore */ }
            b'\t' => {
                // Advance to next 8-column tab stop
                let next = (self.cursor_col / 8 + 1) * 8;
                self.cursor_col = next.min(self.cols - 1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let ps: Vec<u16> = params.iter().map(|p| p[0]).collect();
        let p1 = ps.first().copied().unwrap_or(0);
        let p2 = ps.get(1).copied().unwrap_or(0);

        match action {
            // Cursor movement
            'A' => self.cursor_row = self.cursor_row.saturating_sub(p1.max(1) as usize),
            'B' => self.cursor_row = (self.cursor_row + p1.max(1) as usize).min(self.rows - 1),
            'C' => self.cursor_col = (self.cursor_col + p1.max(1) as usize).min(self.cols - 1),
            'D' => self.cursor_col = self.cursor_col.saturating_sub(p1.max(1) as usize),
            // Cursor next/prev line
            'E' => {
                self.cursor_row = (self.cursor_row + p1.max(1) as usize).min(self.rows - 1);
                self.cursor_col = 0;
            }
            'F' => {
                self.cursor_row = self.cursor_row.saturating_sub(p1.max(1) as usize);
                self.cursor_col = 0;
            }
            // Cursor to column
            'G' => self.cursor_col = (p1.max(1) as usize - 1).min(self.cols - 1),
            // Cursor position (row, col) — 1-indexed
            'H' | 'f' => {
                self.cursor_row = (p1.max(1) as usize - 1).min(self.rows - 1);
                self.cursor_col = (p2.max(1) as usize - 1).min(self.cols - 1);
            }
            // Erase in display
            'J' => self.erase_in_display(p1),
            // Erase in line
            'K' => self.erase_in_line(p1),
            // Insert lines
            'L' => self.scroll_down(p1.max(1) as usize),
            // Delete lines
            'M' => self.scroll_up(p1.max(1) as usize),
            // Delete characters
            'P' => {
                let n = p1.max(1) as usize;
                let r = self.cursor_row.min(self.rows - 1);
                let c = self.cursor_col;
                for i in c..(self.cols).saturating_sub(n) {
                    self.grid[r][i] = self.grid[r][i + n].clone();
                }
                for i in (self.cols.saturating_sub(n))..self.cols {
                    self.grid[r][i] = Cell::default();
                }
                self.mark_dirty(r);
            }
            // Scroll up
            'S' => self.scroll_up(p1.max(1) as usize),
            // Scroll down
            'T' => self.scroll_down(p1.max(1) as usize),
            // Erase characters
            'X' => {
                let n = p1.max(1) as usize;
                let r = self.cursor_row.min(self.rows - 1);
                let c = self.cursor_col;
                for col in c..(c + n).min(self.cols) {
                    self.grid[r][col] = Cell::default();
                }
                self.mark_dirty(r);
            }
            // Cursor to row (absolute)
            'd' => self.cursor_row = (p1.max(1) as usize - 1).min(self.rows - 1),
            // SGR
            'm' => self.apply_sgr(params),
            // Set scroll region
            'r' => {
                let top = (p1.max(1) as usize).saturating_sub(1);
                let bot = if p2 == 0 { self.rows - 1 } else { (p2 as usize).saturating_sub(1) };
                self.scroll_top = top.min(self.rows - 1);
                self.scroll_bot = bot.min(self.rows - 1);
                // Cursor to top-left of new region
                self.cursor_row = self.scroll_top;
                self.cursor_col = 0;
            }
            // Save cursor
            's' => {
                self.saved_cursor = (self.cursor_row, self.cursor_col);
                self.saved_attr = self.current_attr;
            }
            // Restore cursor
            'u' => {
                let (r, c) = self.saved_cursor;
                self.cursor_row = r;
                self.cursor_col = c;
                self.current_attr = self.saved_attr;
            }
            // Private DEC modes (h/l) — ignore for now
            'h' | 'l' => {}
            // Device Status Report — ignore
            'n' => {}
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }
        let cmd = std::str::from_utf8(params[0]).unwrap_or("");
        if matches!(cmd, "0" | "1" | "2") {
            if let Some(title) = params.get(1).and_then(|b| std::str::from_utf8(b).ok()) {
                self.title = title.to_string();
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            // Reverse index
            b'M' => {
                if self.cursor_row == self.scroll_top {
                    self.scroll_down(1);
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
            // Save cursor
            b'7' => {
                self.saved_cursor = (self.cursor_row, self.cursor_col);
                self.saved_attr = self.current_attr;
            }
            // Restore cursor
            b'8' => {
                let (r, c) = self.saved_cursor;
                self.cursor_row = r;
                self.cursor_col = c;
                self.current_attr = self.saved_attr;
            }
            // Full reset
            b'c' => {
                *self = TermPerformer::new(self.rows, self.cols);
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
}

// ──────────────────────────────────────────────────────────────────────────────
// TerminalState — public API wrapping performer + parser + PTY
// ──────────────────────────────────────────────────────────────────────────────

pub struct TerminalState {
    pub performer: TermPerformer,
    parser: Parser,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    /// Channel for receiving PTY output from the reader task.
    pty_rx: mpsc::Receiver<Vec<u8>>,
    /// Sender for PTY output (kept to allow cloning for the reader task).
    pty_tx: mpsc::Sender<Vec<u8>>,
    /// Keeps the PTY master alive for the process lifetime.
    _master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    /// Handle to the PTY reader task for cleanup.
    _reader_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TerminalState {
    pub fn new() -> Self {
        let (pty_tx, pty_rx) = mpsc::channel(32);
        Self {
            performer: TermPerformer::new(DEFAULT_ROWS, DEFAULT_COLS),
            parser: Parser::new(),
            writer: None,
            pty_rx,
            pty_tx,
            _master: None,
            _reader_handle: None,
        }
    }

    /// Spawn a real shell, wire up PTY reader/writer.
    /// Call once immediately after `new()`.
    pub fn wire_pty(&mut self) {
        let pty_system = native_pty_system();
        let pair = match pty_system.openpty(PtySize {
            rows: DEFAULT_ROWS as u16,
            cols: DEFAULT_COLS as u16,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[SheshAOS] PTY open failed: {e}");
                return;
            }
        };

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let cmd = CommandBuilder::new(&shell);
        if let Err(e) = pair.slave.spawn_command(cmd) {
            eprintln!("[SheshAOS] Shell spawn failed: {e}");
            return;
        }

        // Writer (stdin → PTY)
        match pair.master.take_writer() {
            Ok(w) => self.writer = Some(Arc::new(Mutex::new(w))),
            Err(e) => eprintln!("[SheshAOS] PTY writer failed: {e}"),
        }

        // Reader (PTY → mpsc channel) — spawn_blocking with backpressure
        let pty_tx = self.pty_tx.clone();
        let mut reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[SheshAOS] PTY reader failed: {e}");
                return;
            }
        };

        let reader_handle = tokio::task::spawn_blocking(move || {
            let mut buffer = Vec::with_capacity(PTY_READ_CHUNK);
            let mut total_read = 0usize;

            loop {
                // Resize buffer to chunk size
                buffer.resize(PTY_READ_CHUNK, 0);

                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        buffer.truncate(n);
                        total_read += n;

                        // Send chunk to GUI thread
                        if pty_tx.blocking_send(buffer.clone()).is_err() {
                            break; // Channel closed
                        }

                        // Backpressure: if buffer is getting large, yield to allow GUI to process
                        if total_read >= PTY_MAX_BUFFER {
                            std::thread::yield_now();
                            total_read = 0;
                        }
                    }
                }
            }
        });

        self._reader_handle = Some(reader_handle);
        self._master = Some(pair.master);
    }

    /// Drain the output channel and feed bytes through the VTE parser.
    /// Call this every ~16–50 ms from the iced tick subscription.
    pub fn poll_output(&mut self) {
        // Try to receive all available chunks without blocking
        while let Ok(chunk) = self.pty_rx.try_recv() {
            for byte in chunk {
                self.parser.advance(&mut self.performer, byte);
            }
        }
        // Clear dirty flags after processing new output
        self.performer.clear_dirty();
    }

    fn write_to_pty(&self, data: &[u8]) {
        if let Some(ref w) = self.writer {
            if let Ok(mut writer) = w.lock() {
                let _ = writer.write_all(data);
                let _ = writer.flush();
            }
        }
    }

    /// Handle a regular printable character from the keyboard.
    pub fn handle_char(&mut self, c: char) {
        let mut buf = [0u8; 4];
        self.write_to_pty(c.encode_utf8(&mut buf).as_bytes());
    }

    /// Handle special / modified keys from the keyboard.
    pub fn handle_key(&mut self, key: keyboard::Key, modifiers: keyboard::Modifiers) {
        use keyboard::key::Named;
        use keyboard::Key;

        match key {
            Key::Named(named) => match named {
                Named::Enter => self.write_to_pty(b"\r"),
                Named::Backspace => self.write_to_pty(b"\x7f"),
                Named::Tab => self.write_to_pty(b"\t"),
                Named::Escape => self.write_to_pty(b"\x1b"),
                Named::ArrowUp => self.write_to_pty(b"\x1b[A"),
                Named::ArrowDown => self.write_to_pty(b"\x1b[B"),
                Named::ArrowRight => self.write_to_pty(b"\x1b[C"),
                Named::ArrowLeft => self.write_to_pty(b"\x1b[D"),
                Named::Home => self.write_to_pty(b"\x1b[H"),
                Named::End => self.write_to_pty(b"\x1b[F"),
                Named::PageUp => self.write_to_pty(b"\x1b[5~"),
                Named::PageDown => self.write_to_pty(b"\x1b[6~"),
                Named::Delete => self.write_to_pty(b"\x1b[3~"),
                Named::Insert => self.write_to_pty(b"\x1b[2~"),
                Named::F1 => self.write_to_pty(b"\x1bOP"),
                Named::F2 => self.write_to_pty(b"\x1bOQ"),
                Named::F3 => self.write_to_pty(b"\x1bOR"),
                Named::F4 => self.write_to_pty(b"\x1bOS"),
                Named::F5 => self.write_to_pty(b"\x1b[15~"),
                Named::F6 => self.write_to_pty(b"\x1b[17~"),
                Named::F7 => self.write_to_pty(b"\x1b[18~"),
                Named::F8 => self.write_to_pty(b"\x1b[19~"),
                Named::F9 => self.write_to_pty(b"\x1b[20~"),
                Named::F10 => self.write_to_pty(b"\x1b[21~"),
                Named::F11 => self.write_to_pty(b"\x1b[23~"),
                Named::F12 => self.write_to_pty(b"\x1b[24~"),
                _ => {}
            },
            Key::Character(ref c) => {
                if modifiers.control() {
                    if let Some(ch) = c.chars().next() {
                        // Standard Ctrl+letter → ASCII control code 1–26
                        let ctrl = (ch.to_ascii_lowercase() as u8) & 0x1F;
                        self.write_to_pty(&[ctrl]);
                    }
                } else if modifiers.alt() {
                    if let Some(ch) = c.chars().next() {
                        // Alt+key → ESC prefix
                        let mut buf = [0u8; 4];
                        let s = ch.encode_utf8(&mut buf);
                        let mut seq = vec![b'\x1b'];
                        seq.extend_from_slice(s.as_bytes());
                        self.write_to_pty(&seq);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn title(&self) -> &str {
        &self.performer.title
    }
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_performer() -> TermPerformer {
        TermPerformer::new(24, 80)
    }

    fn feed(p: &mut TermPerformer, parser: &mut Parser, s: &str) {
        for byte in s.bytes() {
            parser.advance(p, byte);
        }
    }

    #[test]
    fn test_print_simple() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "hello");
        assert_eq!(p.grid[0][0].ch, 'h');
        assert_eq!(p.grid[0][4].ch, 'o');
        assert_eq!(p.cursor_col, 5);
    }

    #[test]
    fn test_newline_and_cr() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "hi\r\nworld");
        assert_eq!(p.grid[0][0].ch, 'h');
        assert_eq!(p.grid[1][0].ch, 'w');
        assert_eq!(p.cursor_row, 1);
    }

    #[test]
    fn test_ansi_color() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[31mX");
        assert_eq!(p.grid[0][0].ch, 'X');
        assert_eq!(p.grid[0][0].attr.fg, TermColor::Indexed(1));
    }

    #[test]
    fn test_sgr_reset() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[1;31mA\x1b[0mB");
        assert!(p.grid[0][0].attr.bold);
        assert!(!p.grid[0][1].attr.bold);
        assert_eq!(p.grid[0][1].attr.fg, TermColor::Default);
    }

    #[test]
    fn test_cursor_movement() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[5;10H"); // row=5 col=10 (1-indexed)
        assert_eq!(p.cursor_row, 4);
        assert_eq!(p.cursor_col, 9);
    }

    #[test]
    fn test_erase_line() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "abcde\r\x1b[2K"); // write then erase whole line
        assert_eq!(p.grid[0][0].ch, ' ');
        assert_eq!(p.grid[0][4].ch, ' ');
    }

    #[test]
    fn test_scroll_up_pushes_to_scrollback() {
        let mut p = TermPerformer::new(3, 5);
        let mut parser = Parser::new();
        // Fill 3 rows then scroll up
        feed(&mut p, &mut parser, "aaa\r\nbbb\r\nccc\r\nddd");
        // After 4 lines in 3-row terminal, one line should be in scrollback
        assert!(!p.scrollback.is_empty());
    }

    #[test]
    fn test_ctrl_key_byte() {
        // Ctrl+C = byte 3
        let ctrl_c = (b'c' & 0x1F) as char;
        assert_eq!(ctrl_c as u8, 3);
        // Ctrl+D = byte 4
        let ctrl_d = (b'd' & 0x1F) as char;
        assert_eq!(ctrl_d as u8, 4);
    }

    #[test]
    fn test_terminal_default() {
        let t = TerminalState::default();
        // Title is set from SHELL env var in new()
        assert!(!t.title().is_empty());
    }

    #[test]
    fn test_terminal_title_set_and_get() {
        let mut t = TerminalState::default();
        t.performer.title = "myterm".to_string();
        assert_eq!(t.title(), "myterm");
    }

    #[test]
    fn test_terminal_write_empty() {
        let t = TerminalState::default();
        t.write_to_pty(b"");
        // Should not panic
    }

    #[test]
    fn test_terminal_write_non_empty() {
        let t = TerminalState::default();
        t.write_to_pty(b"hello");
        // Should not panic; underlying writer may be None in test
    }

    #[test]
    fn test_terminal_handle_char_lowercase() {
        let mut t = TerminalState::default();
        t.handle_char('a');
        // Should not panic
    }

    #[test]
    fn test_terminal_handle_char_unicode() {
        let mut t = TerminalState::default();
        t.handle_char('ñ');
        // Should not panic
    }

    #[test]
    fn test_terminal_handle_key_unknown() {
        let mut t = TerminalState::default();
        use keyboard::Key;
        t.handle_key(Key::Unidentified, keyboard::Modifiers::empty());
        // Should not panic
    }

    #[test]
    fn test_term_performer_new_defaults() {
        let p = TermPerformer::new(24, 80);
        assert_eq!(p.rows, 24);
        assert_eq!(p.cols, 80);
        assert_eq!(p.cursor_row, 0);
        assert_eq!(p.cursor_col, 0);
        assert_eq!(p.grid.len(), 24);
        assert_eq!(p.grid[0].len(), 80);
    }

    #[test]
    fn test_term_performer_scrollback_initially_empty() {
        let p = TermPerformer::new(24, 80);
        assert!(p.scrollback.is_empty());
    }

    #[test]
    fn test_term_performer_dirty_lines_init() {
        let p = TermPerformer::new(5, 10);
        assert_eq!(p.dirty_lines.len(), 5);
        assert!(p.dirty_lines.iter().all(|&d| d));
    }

    #[test]
    fn test_term_performer_clear_dirty() {
        let mut p = TermPerformer::new(5, 10);
        p.clear_dirty();
        assert!(p.dirty_lines.iter().all(|&d| !d));
    }

    #[test]
    fn test_cell_default() {
        let c = Cell::default();
        assert_eq!(c.ch, ' ');
        assert_eq!(c.attr.fg, TermColor::Default);
    }

    #[test]
    fn test_cell_attr_default() {
        let attr = CellAttr::default();
        assert_eq!(attr.fg, TermColor::Default);
        assert_eq!(attr.bg, TermColor::Default);
        assert!(!attr.bold);
    }

    #[test]
    fn test_term_color_indexed() {
        assert_eq!(TermColor::Indexed(1), TermColor::Indexed(1));
        assert_ne!(TermColor::Indexed(1), TermColor::Indexed(2));
    }

    #[test]
    fn test_term_color_rgb() {
        assert_eq!(TermColor::Rgb(1, 2, 3), TermColor::Rgb(1, 2, 3));
        assert_ne!(TermColor::Rgb(1, 2, 3), TermColor::Rgb(1, 2, 4));
    }

    #[test]
    fn test_sgr_bold_italic_underline() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[1;3;4;7mA");
        assert!(p.grid[0][0].attr.bold);
        assert!(p.grid[0][0].attr.italic);
        assert!(p.grid[0][0].attr.underline);
        assert!(p.grid[0][0].attr.reverse);
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[10;20H\x1b7\x1b[5;5H\x1b8");
        assert_eq!(p.cursor_row, 9);
        assert_eq!(p.cursor_col, 19);
    }

    #[test]
    fn test_reverse_index_at_top() {
        let mut p = TermPerformer::new(3, 5);
        let mut parser = Parser::new();
        p.cursor_row = 0;
        feed(&mut p, &mut parser, "\x1bM");
        // Should scroll down, cursor stays at top
        assert_eq!(p.cursor_row, 0);
    }

    #[test]
    fn test_scroll_region() {
        let mut p = TermPerformer::new(5, 5);
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[2;4r\x1b[5;5Hx\x1bD");
        // Cursor at row 4 (0-indexed), scroll region 1..3
        assert_eq!(p.cursor_row, 4);
    }

    #[test]
    fn test_erase_display() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "hello\x1b[2J");
        // After erase display, screen should be cleared
        assert_eq!(p.grid[0][0].ch, ' ');
    }

    #[test]
    fn test_multiple_ansi_sequences() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b[31mR\x1b[32mG\x1b[34mB\x1b[0mX");
        assert_eq!(p.grid[0][0].attr.fg, TermColor::Indexed(1)); // red
        assert_eq!(p.grid[0][1].attr.fg, TermColor::Indexed(2)); // green
        assert_eq!(p.grid[0][2].attr.fg, TermColor::Indexed(4)); // blue
        assert_eq!(p.grid[0][3].attr.fg, TermColor::Default); // reset
    }

    #[test]
    fn test_wrap_long_line() {
        let mut p = TermPerformer::new(2, 5);
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "abcdefghi");
        // Should wrap to next line
        assert_eq!(p.cursor_row, 1);
    }

    #[test]
    fn test_term_performer_grid_dimensions() {
        let p = make_performer();
        assert_eq!(p.grid.len(), 24);
        assert_eq!(p.grid[0].len(), 80);
    }

    #[test]
    fn test_term_performer_title_set_via_osc() {
        let mut p = make_performer();
        let mut parser = Parser::new();
        feed(&mut p, &mut parser, "\x1b]0;my title\x07");
        assert_eq!(p.title, "my title");
    }
}
