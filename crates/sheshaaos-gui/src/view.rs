use iced::widget::canvas::{self, Canvas, Frame, Geometry, Program};
use iced::widget::{column, container, mouse_area, row, rule, text, Space};
use iced::mouse::Cursor;
use iced::{Alignment, Color, Element, Length, Padding, Point, Rectangle, Size};
use std::cell::RefCell;

use crate::app::{Message, NexusApp, Tab};
use crate::terminal::{CellAttr, TermColor};
use crate::theme;

/// A span of contiguous cells with the same attributes.
struct CellSpan {
    start_col: usize,
    end_col: usize,
    text: String,
    fg: Color,
    bg: Color,
}

/// Dirty state tracking for a single terminal line.
struct LineCache {
    dirty: RefCell<bool>,
}

fn term_color_to_iced(c: TermColor, is_fg: bool) -> Color {
    match c {
        TermColor::Default => {
            if is_fg {
                theme::TEXT
            } else {
                theme::BASE
            }
        }
        TermColor::Indexed(idx) => {
            match idx {
                0 => Color::from_rgb8(69, 71, 90),     // black
                1 => Color::from_rgb8(243, 139, 168),  // red
                2 => Color::from_rgb8(166, 227, 161),  // green
                3 => Color::from_rgb8(249, 226, 175),  // yellow
                4 => Color::from_rgb8(137, 180, 250),  // blue
                5 => Color::from_rgb8(245, 194, 231),  // magenta
                6 => Color::from_rgb8(148, 226, 213),  // cyan
                7 => Color::from_rgb8(186, 194, 222),  // white
                8 => Color::from_rgb8(88, 91, 112),    // bright black
                9 => Color::from_rgb8(243, 139, 168),  // bright red
                10 => Color::from_rgb8(166, 227, 161), // bright green
                11 => Color::from_rgb8(249, 226, 175), // bright yellow
                12 => Color::from_rgb8(137, 180, 250), // bright blue
                13 => Color::from_rgb8(245, 194, 231), // bright magenta
                14 => Color::from_rgb8(148, 226, 213), // bright cyan
                15 => Color::from_rgb8(166, 173, 200), // bright white
                _ => {
                    if is_fg {
                        theme::TEXT
                    } else {
                        theme::BASE
                    }
                }
            }
        }
        TermColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
    }
}

fn cell_attr_to_colors(attr: &CellAttr) -> (Color, Color) {
    let mut fg = term_color_to_iced(attr.fg, true);
    let mut bg = term_color_to_iced(attr.bg, false);

    if attr.reverse {
        std::mem::swap(&mut fg, &mut bg);
    }
    (fg, bg)
}

/// Group a row of cells into spans of contiguous cells with identical attributes.
fn row_to_spans(row: &[crate::terminal::Cell]) -> Vec<CellSpan> {
    let mut spans = Vec::new();
    let mut current_span: Option<CellSpan> = None;

    for (col, cell) in row.iter().enumerate() {
        let (fg, bg) = cell_attr_to_colors(&cell.attr);
        let ch = if cell.ch == '\0' { ' ' } else { cell.ch };

        match &mut current_span {
            Some(span) if span.fg == fg && span.bg == bg => {
                // Continue current span
                span.end_col = col;
                span.text.push(ch);
            }
            _ => {
                // Finish previous span
                if let Some(span) = current_span.take() {
                    spans.push(span);
                }
                // Start new span
                current_span = Some(CellSpan {
                    start_col: col,
                    end_col: col,
                    text: ch.to_string(),
                    fg,
                    bg,
                });
            }
        }
    }

    if let Some(span) = current_span {
        spans.push(span);
    }

    spans
}

struct TerminalView<'a> {
    app: &'a NexusApp,
}

impl<'a> Program<Message> for TerminalView<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut geometries = Vec::new();

        let p = &self.app.terminal.performer;
        let cell_w = crate::terminal::CELL_W;
        let cell_h = crate::terminal::CELL_H;

        // Draw background as a single geometry
        let mut bg_frame = Frame::new(renderer, bounds.size());
        bg_frame.fill_rectangle(Point::ORIGIN, bounds.size(), theme::BASE);
        geometries.push(bg_frame.into_geometry());

        // Draw all lines — render each line at its correct Y position
        for (r, row) in p.grid.iter().enumerate() {
            let y = r as f32 * cell_h;
            let mut line_frame = Frame::new(renderer, Size::new(bounds.width, cell_h));

            let spans = row_to_spans(row);

            for span in spans {
                let x = span.start_col as f32 * cell_w;
                let span_width = (span.end_col - span.start_col + 1) as f32 * cell_w;

                if span.bg != theme::BASE {
                    line_frame.fill_rectangle(
                        Point::new(x, y),
                        Size::new(span_width, cell_h),
                        span.bg,
                    );
                }

                if !span.text.trim().is_empty() {
                    let text = canvas::Text {
                        content: span.text,
                        position: Point::new(x, y),
                        color: span.fg,
                        size: iced::Pixels(14.0),
                        font: iced::Font::MONOSPACE,
                        ..Default::default()
                    };
                    line_frame.fill_text(text);
                }
            }

            geometries.push(line_frame.into_geometry());
        }

        // Draw cursor at its actual position (always on top)
        let cx = p.cursor_col as f32 * cell_w;
        let cy = p.cursor_row as f32 * cell_h;
        let mut cursor_frame = Frame::new(renderer, Size::new(cell_w, cell_h));
        cursor_frame.fill_rectangle(
            Point::new(cx, cy),
            Size::new(cell_w, cell_h),
            Color::from_rgba(1.0, 1.0, 1.0, 0.5),
        );
        geometries.push(cursor_frame.into_geometry());

        geometries
    }
}

pub fn render(app: &NexusApp) -> Element<'_, Message> {
    // --- Sidebar: icon rail ---
    let sidebar = container(
        column![
            // Logo
            text("N").size(28).color(theme::LAVENDER),
            Space::new().height(16),
            rule::horizontal(1),
            Space::new().height(16),
            // Terminal icon
            mouse_area(
                container(text("▣").size(20).color(if app.active_tab == Tab::Terminal {
                    theme::BLUE
                } else {
                    theme::SUBTEXT0
                }))
                .padding(8)
            )
            .on_press(Message::SwitchTab(Tab::Terminal)),
            Space::new().height(4),
            // AI Chat icon
            mouse_area(
                container(text("⚡").size(20).color(if app.active_tab == Tab::AiChat {
                    theme::PEACH
                } else {
                    theme::SUBTEXT0
                }))
                .padding(8)
            )
            .on_press(Message::SwitchTab(Tab::AiChat)),
            Space::new().height(Length::Fill),
            // Settings
            text("⚙").size(18).color(theme::SURFACE1),
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(56.0))
    .height(Length::Fill)
    .padding(Padding::from([16, 12]))
    .style(theme::sidebar_style);

    // --- Main content ---
    let main_content: Element<'_, Message> = match app.active_tab {
        Tab::Terminal => render_terminal(app),
        Tab::AiChat => render_ai_chat(app),
    };

    // --- Root layout ---
    row![sidebar, main_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn render_terminal(app: &NexusApp) -> Element<'_, Message> {
    // --- Tab bar ---
    let tab_title = format!("● {}", app.terminal.title());
    let tab_bar = container(
        row![
            text(tab_title).size(13).color(theme::GREEN),
            Space::new().width(20),
            text("SheshaAOS v0.1.7").size(11).color(theme::SURFACE1),
            Space::new().width(Length::Fill),
            text(format!("{}×{}", app.terminal.performer.cols, app.terminal.performer.rows)).size(11).color(theme::SUBTEXT0),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(Padding::from([8, 16]))
    .style(theme::tab_bar_style);

    // --- Terminal Canvas ---
    let terminal_canvas = Canvas::new(TerminalView { app })
        .width(Length::Fill)
        .height(Length::Fill);

    let terminal_pane = container(terminal_canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::terminal_pane_style);

    // --- Status bar ---
    let status_bar = container(
        row![
            text("NORMAL").size(11).color(theme::BLUE),
            Space::new().width(16),
            text(app.terminal.title()).size(11).color(theme::SUBTEXT0),
            Space::new().width(Length::Fill),
            text("UTF-8  LF").size(11).color(theme::SUBTEXT0),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(Padding::from([4, 16]))
    .style(theme::status_bar_style);

    // --- Assemble ---
    container(column![tab_bar, terminal_pane, status_bar])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::main_area_style)
        .into()
}

fn render_ai_chat(app: &NexusApp) -> Element<'_, Message> {
    let mut messages_col = iced::widget::Column::new().spacing(8);
    for msg in &app.ai_messages {
        let is_user = msg.role == "user";
        let color = if is_user { theme::BLUE } else { theme::PEACH };
        let prefix = if is_user { "You: " } else { "AI: " };
        let content = if msg.is_streaming {
            format!("{}{}▊", prefix, msg.content) // Show cursor while streaming
        } else {
            format!("{}{}", prefix, msg.content)
        };
        messages_col = messages_col.push(
            container(text(content).size(14).color(color))
                .padding(8)
                .style(theme::tab_bar_style)
        );
    }

    let chat_log = iced::widget::scrollable(messages_col)
        .width(Length::Fill)
        .height(Length::Fill);

    let chat_input = iced::widget::text_input("Type your message...", &app.ai_input)
        .on_input(Message::AiInputChanged)
        .on_submit(Message::AiSubmit)
        .padding(10)
        .size(14);

    container(
        column![
            container(text("AI Chat").size(13).color(theme::PEACH))
                .width(Length::Fill)
                .padding(Padding::from([8, 16]))
                .style(theme::tab_bar_style),
            container(
                column![
                    chat_log,
                    Space::new().width(Length::Fill).height(16),
                    chat_input,
                ]
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16)
            .style(theme::terminal_pane_style),
            container(text("Ready").size(11).color(theme::SUBTEXT0))
                .width(Length::Fill)
                .padding(Padding::from([4, 16]))
                .style(theme::status_bar_style),
        ]
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::main_area_style)
    .into()
}
