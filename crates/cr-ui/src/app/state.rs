//! Application state and main app structure

use anyhow::Result;
use cr_core::comment::Comment;
use cr_core::diff::{DiffNavigator, DiffParser, FileDiff, LineType};
use cr_core::diff::Line as DiffLine;
use cr_core::session::Session;
use cr_core::types::{CommentId, FileId, LineId};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use ratatui::text::Line as TextLine;
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::highlight::Highlighter;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal browsing mode
    Normal,
    /// Input mode for adding/editing comments
    Insert,
    /// Help mode
    Help,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Normal
    }
}

/// Application state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// Current mode
    pub mode: AppMode,
    /// Status message
    pub message: Option<String>,
    /// Should quit
    pub should_quit: bool,
    /// Current file index
    pub current_file: usize,
    /// Current line index within file (cursor position)
    pub current_line: usize,
    /// Scroll offset for diff view
    pub scroll_offset: usize,
    /// Comment editor content
    pub editor_content: String,
    /// Editor cursor position
    pub editor_cursor: usize,
    /// Is this a file-level comment?
    pub is_file_comment: bool,
}

impl AppState {
    /// Create a new app state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set status message
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    /// Clear status message
    pub fn clear_message(&mut self) {
        self.message = None;
    }
}

/// Main application
pub struct App {
    /// Application state
    pub state: AppState,
    /// Current session
    pub session: Session,
    /// Diff navigator
    pub navigator: DiffNavigator,
    /// Terminal
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Diff parser for lazy loading
    parser: DiffParser,
    /// Line comments cache: FileId -> LineId -> Vec<CommentId>
    line_comments: HashMap<FileId, HashMap<LineId, Vec<CommentId>>>,
    /// Syntax highlighter
    highlighter: Highlighter,
}

impl App {
    /// Create a new app with the given session
    pub fn new(session: Session) -> Result<Self> {
        // Install panic hook to restore terminal on panic
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            original_hook(panic_info);
        }));

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let navigator = DiffNavigator::new(session.diff_data.clone());

        // Build line comments index
        let mut line_comments: HashMap<FileId, HashMap<LineId, Vec<CommentId>>> = HashMap::new();
        for comment in session.comments.all_sorted() {
            let file_id = comment.file_id().clone();
            for line_id in comment.line_ids() {
                line_comments
                    .entry(file_id.clone())
                    .or_default()
                    .entry(line_id.clone())
                    .or_default()
                    .push(comment.id.clone());
            }
        }

        let mut app = Self {
            state: AppState::new(),
            session,
            navigator,
            terminal,
            parser: DiffParser::new(),
            line_comments,
            highlighter: Highlighter::new(),
        };

        // Load first file if it's lazy
        app.load_current_file();

        Ok(app)
    }

    /// Run the main application loop
    pub fn run(&mut self) -> Result<()> {
        loop {
            // Render
            self.draw()?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let event::Event::Key(key) = event::read()? {
                    self.handle_input(key)?;
                }
            }

            // Check if should quit
            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self) -> Result<()> {
        let state = self.state.clone();
        let files = &self.session.diff_data.files;
        let comments = &self.session.comments;
        let line_comments = &self.line_comments;
        let highlighter = &self.highlighter;

        // Get current file
        let current_file = files.get(state.current_file);
        let file_count = files.len();
        let session_id = self.session.id.to_string();

        // Collect comments for rendering
        let all_comments: Vec<_> = comments.all_sorted().into_iter().cloned().collect();

        self.terminal.draw(|frame| {
            let area = frame.area();

            match state.mode {
                AppMode::Help => render_help(frame, area),
                AppMode::Insert => render_with_editor(frame, area, &state, current_file, file_count, &all_comments, line_comments, &session_id, highlighter),
                AppMode::Normal => render_diff_only(frame, area, &state, current_file, file_count, &all_comments, line_comments, &session_id, highlighter),
            }
        })?;
        Ok(())
    }

    /// Handle keyboard input
    fn handle_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.state.mode {
            AppMode::Normal => self.handle_normal_input(key),
            AppMode::Insert => self.handle_insert_input(key),
            AppMode::Help => self.handle_help_input(key),
        }
    }

    /// Handle input in normal mode
    fn handle_normal_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => self.state.should_quit = true,
            KeyCode::Char('?') => self.state.mode = AppMode::Help,

            // Line navigation (vim-like)
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            KeyCode::Char('g') => self.goto_top(),
            KeyCode::Char('G') => self.goto_bottom(),

            // File navigation
            KeyCode::Char('n') => self.next_file(),
            KeyCode::Char('N') => self.prev_file(),
            KeyCode::Char(']') if key.modifiers.contains(KeyModifiers::NONE) => self.next_comment(),
            KeyCode::Char('[') if key.modifiers.contains(KeyModifiers::NONE) => self.prev_comment(),

            // Comments
            KeyCode::Char('c') => {
                self.state.mode = AppMode::Insert;
                self.state.is_file_comment = false;
                self.state.editor_content.clear();
                self.state.editor_cursor = 0;
            }
            KeyCode::Char('C') => {
                self.state.mode = AppMode::Insert;
                self.state.is_file_comment = true;
                self.state.editor_content.clear();
                self.state.editor_cursor = 0;
            }
            // Page up/down (check Ctrl modifiers first)
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => self.page_up(),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => self.page_down(),

            // Delete comment (x key, vim-like)
            KeyCode::Char('x') => self.delete_comment_at_line(),

            // Session
            KeyCode::Char('s') => self.state.set_message("Session saved"),

            _ => {}
        }
        Ok(())
    }

    /// Handle input in insert mode
    fn handle_insert_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
                self.state.editor_content.clear();
                self.state.editor_cursor = 0;
            }
            KeyCode::Enter => {
                if !self.state.editor_content.trim().is_empty() {
                    self.add_comment();
                }
                self.state.mode = AppMode::Normal;
                self.state.editor_content.clear();
                self.state.editor_cursor = 0;
            }
            KeyCode::Char(c) => {
                // editor_cursor is char position, convert to byte position for insert
                let byte_pos = self.char_to_byte_pos(self.state.editor_cursor);
                self.state.editor_content.insert(byte_pos, c);
                self.state.editor_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.state.editor_cursor > 0 {
                    self.state.editor_cursor -= 1;
                    let byte_pos = self.char_to_byte_pos(self.state.editor_cursor);
                    // Remove the character at this position
                    let char_len = self.state.editor_content[byte_pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                    self.state.editor_content.drain(byte_pos..byte_pos + char_len);
                }
            }
            KeyCode::Left => {
                if self.state.editor_cursor > 0 {
                    self.state.editor_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = self.state.editor_content.chars().count();
                if self.state.editor_cursor < char_count {
                    self.state.editor_cursor += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Convert character position to byte position
    fn char_to_byte_pos(&self, char_pos: usize) -> usize {
        self.state.editor_content
            .char_indices()
            .nth(char_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.state.editor_content.len())
    }

    /// Handle input in help mode
    fn handle_help_input(&mut self, _key: KeyEvent) -> Result<()> {
        self.state.mode = AppMode::Normal;
        Ok(())
    }

    /// Get total lines in current file
    fn current_file_line_count(&self) -> usize {
        self.session.diff_data.files
            .get(self.state.current_file)
            .map(|f| f.total_lines())
            .unwrap_or(0)
    }

    fn move_down(&mut self) {
        let max = self.current_file_line_count().saturating_sub(1);
        if self.state.current_line < max {
            self.state.current_line += 1;
            self.ensure_visible();
        }
    }

    fn move_up(&mut self) {
        if self.state.current_line > 0 {
            self.state.current_line -= 1;
            self.ensure_visible();
        }
    }

    fn page_up(&mut self) {
        self.state.current_line = self.state.current_line.saturating_sub(20);
        self.ensure_visible();
    }

    fn page_down(&mut self) {
        let max = self.current_file_line_count().saturating_sub(1);
        self.state.current_line = (self.state.current_line + 20).min(max);
        self.ensure_visible();
    }

    fn ensure_visible(&mut self) {
        // Keep cursor visible in viewport
        let viewport_height = 20; // approximate
        if self.state.current_line < self.state.scroll_offset {
            self.state.scroll_offset = self.state.current_line;
        } else if self.state.current_line >= self.state.scroll_offset + viewport_height {
            self.state.scroll_offset = self.state.current_line - viewport_height + 1;
        }
    }

    fn next_file(&mut self) {
        if self.state.current_file < self.session.diff_data.files.len().saturating_sub(1) {
            self.state.current_file += 1;
            self.state.current_line = 0;
            self.state.scroll_offset = 0;
            self.load_current_file();
        }
    }

    fn prev_file(&mut self) {
        if self.state.current_file > 0 {
            self.state.current_file -= 1;
            self.state.current_line = 0;
            self.state.scroll_offset = 0;
            self.load_current_file();
        }
    }

    /// Load current file if it's lazy
    fn load_current_file(&mut self) {
        if let Some(file) = self.session.diff_data.files.get_mut(self.state.current_file) {
            if file.needs_loading() {
                if let Err(e) = self.parser.load_lazy_file(file) {
                    self.state.set_message(format!("Failed to load file: {}", e));
                }
            }
        }
    }

    fn goto_top(&mut self) {
        self.state.current_line = 0;
        self.state.scroll_offset = 0;
    }

    fn goto_bottom(&mut self) {
        self.state.current_line = self.current_file_line_count().saturating_sub(1);
        self.ensure_visible();
    }

    fn next_comment(&mut self) {
        // Jump to next line with comment
        if let Some(file) = self.session.diff_data.files.get(self.state.current_file) {
            let file_comments = self.line_comments.get(&file.id);
            if let Some(fc) = file_comments {
                let mut line_idx = 0;
                for hunk in &file.hunks {
                    for line in &hunk.lines {
                        if line_idx > self.state.current_line && fc.contains_key(&line.id) {
                            self.state.current_line = line_idx;
                            self.ensure_visible();
                            return;
                        }
                        line_idx += 1;
                    }
                }
            }
        }
        self.state.set_message("No more comments");
    }

    fn prev_comment(&mut self) {
        // Jump to previous line with comment
        if let Some(file) = self.session.diff_data.files.get(self.state.current_file) {
            let file_comments = self.line_comments.get(&file.id);
            if let Some(fc) = file_comments {
                let mut line_idx = 0;
                let mut last_comment_line = None;
                for hunk in &file.hunks {
                    for line in &hunk.lines {
                        if line_idx < self.state.current_line && fc.contains_key(&line.id) {
                            last_comment_line = Some(line_idx);
                        }
                        line_idx += 1;
                    }
                }
                if let Some(idx) = last_comment_line {
                    self.state.current_line = idx;
                    self.ensure_visible();
                    return;
                }
            }
        }
        self.state.set_message("No previous comments");
    }

    fn add_comment(&mut self) {
        use cr_core::comment::builder::CommentBuilder;
        use cr_core::comment::model::DiffSide;

        if let Some(file) = self.session.diff_data.files.get(self.state.current_file) {
            let file_id = file.id.clone();
            let file_path = file.display_path().to_string_lossy().to_string();

            // Find the line at current cursor position
            let mut line_idx = 0;
            let mut target_line: Option<&DiffLine> = None;
            let mut line_number = 0;

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_idx == self.state.current_line {
                        target_line = Some(line);
                        line_number = line.new_line_num.or(line.old_line_num).unwrap_or(0);
                        break;
                    }
                    line_idx += 1;
                }
                if target_line.is_some() {
                    break;
                }
            }

            let line_id = target_line
                .map(|l| l.id.clone())
                .unwrap_or_else(|| LineId::from_string("file-comment"));

            let side = target_line
                .map(|l| match l.line_type {
                    LineType::Added => DiffSide::New,
                    LineType::Deleted => DiffSide::Old,
                    _ => DiffSide::New,
                })
                .unwrap_or(DiffSide::New);

            if let Ok(comment) = CommentBuilder::new(file_id.clone(), line_id.clone(), side)
                .content(&self.state.editor_content)
                .file_path(&file_path)
                .line_number(line_number)
                .build()
            {
                let comment_id = comment.id.clone();
                if self.session.comments.add(comment).is_ok() {
                    // Update line comments cache
                    self.line_comments
                        .entry(file_id)
                        .or_default()
                        .entry(line_id)
                        .or_default()
                        .push(comment_id);
                    self.state.set_message("Comment added");
                }
            }
        }
    }

    fn delete_comment_at_line(&mut self) {
        if let Some(file) = self.session.diff_data.files.get(self.state.current_file) {
            // Find line at current position
            let mut line_idx = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_idx == self.state.current_line {
                        // Find comments on this line
                        if let Some(fc) = self.line_comments.get_mut(&file.id) {
                            if let Some(comment_ids) = fc.get_mut(&line.id) {
                                if let Some(id) = comment_ids.pop() {
                                    if self.session.comments.delete(&id).is_ok() {
                                        self.state.set_message("Comment deleted");
                                        return;
                                    }
                                }
                            }
                        }
                        self.state.set_message("No comment on this line");
                        return;
                    }
                    line_idx += 1;
                }
            }
        }
    }

    /// Get a clone of the current session
    pub fn get_session(&self) -> Session {
        self.session.clone()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture);
        let _ = self.terminal.show_cursor();
    }
}

// Render functions

fn render_diff_only(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    file: Option<&FileDiff>,
    file_count: usize,
    comments: &[Comment],
    line_comments: &HashMap<FileId, HashMap<LineId, Vec<CommentId>>>,
    session_id: &str,
    highlighter: &Highlighter,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    // Title bar
    render_title_bar(frame, chunks[0], state, file, file_count);

    // Diff content with inline comments
    render_diff_with_comments(frame, chunks[1], state, file, comments, line_comments, highlighter);

    // Status bar
    render_status_bar(frame, chunks[2], state, file_count, comments.len(), session_id);
}

fn render_with_editor(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    file: Option<&FileDiff>,
    file_count: usize,
    comments: &[Comment],
    line_comments: &HashMap<FileId, HashMap<LineId, Vec<CommentId>>>,
    session_id: &str,
    highlighter: &Highlighter,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(3), Constraint::Length(1)])
        .split(area);

    render_title_bar(frame, chunks[0], state, file, file_count);
    render_diff_with_comments(frame, chunks[1], state, file, comments, line_comments, highlighter);
    render_comment_editor(frame, chunks[2], state);
    render_status_bar(frame, chunks[3], state, file_count, comments.len(), session_id);
}

fn render_title_bar(frame: &mut Frame, area: Rect, state: &AppState, file: Option<&FileDiff>, file_count: usize) {
    let title = if let Some(f) = file {
        let path = f.display_path().to_string_lossy();
        let mode_icon = match f.mode {
            cr_core::diff::FileMode::Added => "+",
            cr_core::diff::FileMode::Deleted => "-",
            cr_core::diff::FileMode::Modified => "~",
            cr_core::diff::FileMode::Renamed => ">",
            cr_core::diff::FileMode::Copied => "C",
            cr_core::diff::FileMode::Binary => "B",
        };
        format!(" {} {} [{}/{}]", mode_icon, path, state.current_file + 1, file_count)
    } else {
        " No files".to_string()
    };

    frame.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
        area
    );
}

fn render_diff_with_comments(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    file: Option<&FileDiff>,
    comments: &[Comment],
    line_comments: &HashMap<FileId, HashMap<LineId, Vec<CommentId>>>,
    highlighter: &Highlighter,
) {
    let Some(file) = file else {
        frame.render_widget(
            Paragraph::new("No diff to display").block(Block::default().borders(Borders::ALL)),
            area
        );
        return;
    };

    // Build comment lookup
    let file_line_comments = line_comments.get(&file.id);
    let comment_map: HashMap<CommentId, &Comment> = comments.iter().map(|c| (c.id.clone(), c)).collect();

    let mut lines_to_render: Vec<TextLine> = Vec::new();
    let mut line_idx = 0;

    // Get file path for syntax detection
    let file_path = file.display_path().to_string_lossy().to_string();

    for hunk in &file.hunks {
        // Hunk header
        lines_to_render.push(TextLine::from(Span::styled(
            &hunk.header,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM)
        )));

        for line in &hunk.lines {
            let is_current = line_idx == state.current_line;

            // Build line number display
            let line_num = match (line.old_line_num, line.new_line_num) {
                (Some(o), Some(n)) => format!("{:>4} {:>4}", o, n),
                (Some(o), None) => format!("{:>4}     ", o),
                (None, Some(n)) => format!("     {:>4}", n),
                (None, None) => "         ".to_string(),
            };

            // Line prefix and base style for diff markers
            let (prefix, diff_style) = match line.line_type {
                LineType::Added => ("+", Style::default().fg(Color::Green)),
                LineType::Deleted => ("-", Style::default().fg(Color::Red)),
                LineType::Context => (" ", Style::default()),
                LineType::NoNewline => ("\\", Style::default().fg(Color::DarkGray)),
            };

            // Build spans for the line
            let mut spans: Vec<Span> = vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(prefix.to_string(), diff_style),
            ];

            // Apply syntax highlighting for non-special lines
            if line.line_type != LineType::NoNewline {
                let highlighted = highlighter.highlight_line(&line.content, &file_path);
                for span in highlighted {
                    // Apply diff background color if needed
                    let mut span_style = span.style;
                    if is_current {
                        span_style = span_style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    } else {
                        // Tint syntax highlighting with diff color
                        match line.line_type {
                            LineType::Added => {
                                span_style = span_style.bg(Color::Rgb(0, 40, 0));
                            }
                            LineType::Deleted => {
                                span_style = span_style.bg(Color::Rgb(40, 0, 0));
                            }
                            _ => {}
                        }
                    }
                    spans.push(Span::styled(span.content.to_string(), span_style));
                }
            } else {
                // NoNewline marker - just show the content
                let mut style = diff_style;
                if is_current {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled(line.content.clone(), style));
            }

            lines_to_render.push(TextLine::from(spans));

            // Render inline comments for this line
            if let Some(fc) = file_line_comments {
                if let Some(comment_ids) = fc.get(&line.id) {
                    for cid in comment_ids {
                        if let Some(comment) = comment_map.get(cid) {
                            let severity_style = match comment.severity {
                                cr_core::comment::Severity::Critical => Style::default().fg(Color::Red),
                                cr_core::comment::Severity::Warning => Style::default().fg(Color::Yellow),
                                cr_core::comment::Severity::Info => Style::default().fg(Color::Blue),
                            };
                            let icon = comment.severity.emoji();
                            lines_to_render.push(TextLine::from(vec![
                                Span::raw("         "),
                                Span::styled(format!("│ {} ", icon), severity_style),
                                Span::styled(&comment.content, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                }
            }

            line_idx += 1;
        }
    }

    // Handle lazy files with no content
    if file.lazy && file.hunks.is_empty() {
        lines_to_render.push(TextLine::from(Span::styled(
            "  (Content not loaded - navigate to load)",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
        )));
    }

    let paragraph = Paragraph::new(lines_to_render)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .scroll((state.scroll_offset as u16, 0));

    frame.render_widget(paragraph, area);
}

fn render_comment_editor(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = if state.is_file_comment {
        "Add File Comment (Enter to confirm, Esc to cancel)"
    } else {
        "Add Line Comment (Enter to confirm, Esc to cancel)"
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let input = Paragraph::new(state.editor_content.as_str())
        .style(Style::default().fg(Color::White));
    frame.render_widget(input, inner);

    // Calculate display width for cursor position (handle CJK characters)
    let display_width: usize = state.editor_content
        .chars()
        .take(state.editor_cursor)
        .map(|c| if c.is_ascii() { 1 } else { 2 }) // CJK chars are 2 columns wide
        .sum();

    // Safe cursor position (clamp to inner area)
    let cursor_x = inner.x.saturating_add(display_width as u16).min(inner.x + inner.width.saturating_sub(1));
    let cursor_y = inner.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    file_count: usize,
    comment_count: usize,
    session_id: &str,
) {
    let mode = match state.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Insert => "INSERT",
        AppMode::Help => "HELP",
    };

    let line_info = format!("L{}", state.current_line + 1);

    let text = state.message.clone().unwrap_or_else(|| {
        format!(
            " {} | {} | {} comments | {} ",
            mode,
            line_info,
            comment_count,
            &session_id[..14.min(session_id.len())]
        )
    });

    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(Color::DarkGray).fg(Color::White)),
        area
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(Span::styled("cr-helper - Code Review", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Yellow))),
        Line::from("  j/k         Move cursor up/down"),
        Line::from("  g/G         Go to top/bottom"),
        Line::from("  Ctrl-u/d    Page up/down"),
        Line::from("  n/N         Next/Previous file"),
        Line::from("  ]/[         Next/Previous comment"),
        Line::from(""),
        Line::from(Span::styled("Comments", Style::default().fg(Color::Yellow))),
        Line::from("  c           Add comment on current line"),
        Line::from("  C           Add file-level comment"),
        Line::from("  x           Delete comment on current line"),
        Line::from(""),
        Line::from(Span::styled("Other", Style::default().fg(Color::Yellow))),
        Line::from("  s           Save session"),
        Line::from("  q           Quit"),
        Line::from("  ?           Show this help"),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let help_area = centered_rect(50, 70, area);
    frame.render_widget(Clear, help_area);
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        ),
        help_area
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_mode_default() {
        assert_eq!(AppMode::default(), AppMode::Normal);
    }

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert_eq!(state.mode, AppMode::Normal);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_app_state_message() {
        let mut state = AppState::new();
        assert!(state.message.is_none());
        state.set_message("Test");
        assert_eq!(state.message, Some("Test".to_string()));
        state.clear_message();
        assert!(state.message.is_none());
    }
}
