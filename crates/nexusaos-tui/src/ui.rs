//! Ratatui UI layout rendering engine for JetBrains IDE styled interface.
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::app::{ActiveToolWindow, AppMode, App};

pub fn render_ui(frame: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 1. Top JetBrains Main Menu & Action Bar (Run / Debug / Git)
            Constraint::Min(12),   // 2. Center Workspace (Project Tree + Editor/AI)
            Constraint::Length(7), // 3. Bottom Tool Window (Terminal / Run / Problems / Git)
            Constraint::Length(3), // 4. Input Bar
            Constraint::Length(1), // 5. JetBrains Bottom Status Strip
        ])
        .split(frame.area());

    // 1. Top JetBrains Main Menu & Action Bar
    render_jetbrains_action_bar(frame, app, main_chunks[0]);

    // 2. Center Workspace Split (Left: Project Tree | Right: Editor / AI Assistant)
    let center_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24), // Left Tool Window: Project Tree (Alt+1)
            Constraint::Percentage(76), // Main Editor & AI Panel
        ])
        .split(main_chunks[1]);

    render_project_tree(frame, app, center_chunks[0]);
    render_editor_ai_panel(frame, app, center_chunks[1]);

    // 3. Bottom Tool Window (Terminal / Run / Vault / Audit Log)
    render_bottom_tool_windows(frame, app, main_chunks[2]);

    // 4. Input Bar
    let input_title = match app.active_tool_window {
        ActiveToolWindow::AiAssistant | ActiveToolWindow::Editor => {
            " 🤖 [CLICK] AI Assistant / Prompt (Type request or /explain, /vault, /clear) "
        }
        ActiveToolWindow::Terminal => " 🖥️ [CLICK] Terminal PTY Command ($ shell execution) ",
        ActiveToolWindow::CommandVault => " ⚡ [CLICK] Search Command Vault ",
        ActiveToolWindow::ProjectTree => " 📁 [CLICK] Search Project Explorer ",
        ActiveToolWindow::GitStatus => " 🌿 [CLICK] Git Command / Commit ",
    };

    let input_text = format!("> {}", app.input_buffer);
    let input_widget = Paragraph::new(input_text)
        .style(Style::default().bg(Color::Rgb(34, 34, 34)).fg(Color::Rgb(88, 193, 66)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(input_title)
                .border_style(Style::default().fg(Color::Rgb(88, 193, 66))),
        );
    frame.render_widget(input_widget, main_chunks[3]);

    // 5. JetBrains Bottom Status Strip with Clickable Buttons
    let status_text = format!(
        " 🌿 {}  |  [📁 Project (Alt+1)]  |  [🖥️ Terminal (Alt+F12)]  |  [⚡ Vault]  |  RAM: {}/{} MB  |  {}",
        app.git_branch, app.ram_used_mb, app.ram_total_mb, app.status_message
    );
    let status_bar = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Rgb(34, 34, 34)).fg(Color::Rgb(88, 193, 66)));
    frame.render_widget(status_bar, main_chunks[4]);

    // 6. Security Approval Modal Overlay
    if let AppMode::ApprovalModal { ref action, ref details } = app.mode {
        let area = centered_rect(60, 40, frame.area());
        frame.render_widget(Clear, area);

        let modal_text = format!(
            "⚠️ JETBRAINS SECURITY AUDIT\n\nProposed Action: {}\nDetails: {}\n\nPress [Y] to Allow execution, [N] or [Esc] to Deny.",
            action, details
        );

        let modal = Paragraph::new(modal_text)
            .style(Style::default().fg(Color::Rgb(226, 165, 0))) // JetBrains Warning Amber
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Policy Confirmation ")
                    .border_style(Style::default().fg(Color::Rgb(226, 165, 0))),
            );

        frame.render_widget(modal, area);
    }
}

fn render_jetbrains_action_bar(frame: &mut Frame, app: &App, area: Rect) {
    let bar_text = format!(
        " File  Edit  View  Navigate  Code  Refactor  Run  Tools  |  ▶ {}  |  🌿 {}  |  RAM: {}MB Free",
        app.run_config,
        app.git_branch,
        app.ram_total_mb - app.ram_used_mb
    );

    let action_bar = Paragraph::new(bar_text)
        .style(Style::default().fg(Color::Rgb(73, 156, 84)).add_modifier(Modifier::BOLD)) // IntelliJ Green
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" JetBrains RustRover / IntelliJ IDEA — NexusAOS AI IDE ")
                .border_style(Style::default().fg(Color::Rgb(53, 116, 240))),
        );
    frame.render_widget(action_bar, area);
}

fn render_project_tree(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .project_tree
        .iter()
        .map(|node| {
            ListItem::new(node.as_str()).style(Style::default().fg(Color::Rgb(187, 187, 187)))
        })
        .collect();

    let is_focused = app.active_tool_window == ActiveToolWindow::ProjectTree;
    let border_color = if is_focused { Color::Rgb(53, 116, 240) } else { Color::Rgb(60, 63, 65) };

    let tree = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 📁 1: Project ")
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(tree, area);
}

fn render_editor_ai_panel(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .history
        .iter()
        .map(|line| {
            let style = if line.starts_with('>') {
                Style::default().fg(Color::Rgb(204, 120, 50)).add_modifier(Modifier::BOLD) // Darcula Keyword Orange
            } else if line.starts_with("===") {
                Style::default().fg(Color::Rgb(53, 116, 240)).add_modifier(Modifier::BOLD)
            } else if line.starts_with("⚠️") || line.starts_with("[ERROR]") {
                Style::default().fg(Color::Red)
            } else if line.starts_with("✓") || line.starts_with("[SUCCESS]") {
                Style::default().fg(Color::Rgb(73, 156, 84)) // IntelliJ Green
            } else {
                Style::default().fg(Color::Rgb(187, 187, 187))
            };
            ListItem::new(line.as_str()).style(style)
        })
        .collect();

    let is_focused = app.active_tool_window == ActiveToolWindow::AiAssistant
        || app.active_tool_window == ActiveToolWindow::Editor;
    let border_color = if is_focused { Color::Rgb(53, 116, 240) } else { Color::Rgb(60, 63, 65) };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 🤖 AI Assistant / Editor [{}] ", app.current_file))
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(list, area);
}

fn render_bottom_tool_windows(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        " 🖥️ Terminal (Alt+F12) ",
        " ▶ Run / Output ",
        " ⚡ Command Vault ",
        " 📊 System Audit ",
    ];

    let selected_index = match app.active_tool_window {
        ActiveToolWindow::Terminal => 0,
        ActiveToolWindow::AiAssistant
        | ActiveToolWindow::Editor
        | ActiveToolWindow::ProjectTree => 1,
        ActiveToolWindow::CommandVault => 2,
        ActiveToolWindow::GitStatus => 3,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(4)])
        .split(area);

    let tabs = Tabs::new(titles)
        .select(selected_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default().fg(Color::Rgb(53, 116, 240)).add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    let items: Vec<ListItem> = app
        .pty_output
        .iter()
        .map(|line| {
            ListItem::new(line.as_str()).style(Style::default().fg(Color::Rgb(73, 156, 84)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 63, 65))),
    );
    frame.render_widget(list, chunks[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
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
        .split(popup_layout[1])[1]
}
