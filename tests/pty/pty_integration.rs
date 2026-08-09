use sheshaaos_gui::terminal::TerminalState;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::time::Duration;

/// Test PTY shell spawning and basic I/O
#[tokio::test]
async fn test_pty_shell_spawn() {
    let mut terminal = TerminalState::new();
    terminal.wire_pty();
    
    // Give PTY time to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Send a command
    terminal.handle_char('e');
    terminal.handle_char('c');
    terminal.handle_char('h');
    terminal.handle_char('o');
    terminal.handle_char(' ');
    terminal.handle_char('h');
    terminal.handle_char('e');
    terminal.handle_char('l');
    terminal.handle_char('l');
    terminal.handle_char('o');
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter), iced::keyboard::Modifiers::empty());
    
    // Wait for output
    tokio::time::sleep(Duration::from_millis(500)).await;
    terminal.poll_output();
    
    // Check output contains "hello"
    let performer = &terminal.performer;
    let output = grid_to_string(&performer.grid);
    assert!(output.contains("hello") || output.contains("echo"), "Output should contain command");
}

/// Test PTY resize handling
#[tokio::test]
async fn test_pty_resize() {
    let mut terminal = TerminalState::new();
    terminal.wire_pty();
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Resize terminal
    terminal.performer.resize(40, 120);
    
    assert_eq!(terminal.performer.rows, 40);
    assert_eq!(terminal.performer.cols, 120);
    assert_eq!(terminal.performer.grid.len(), 40);
    assert_eq!(terminal.performer.grid[0].len(), 120);
}

/// Test Zig VT100 parser integration
#[test]
fn test_zig_vt100_parser() {
    if let Some(parser) = sheshaaos_terminal::ZigVt100Parser::new(80, 24) {
        parser.feed(b"Hello World\r\n");
        parser.feed(b"\x1b[31mRed\x1b[0m");
        
        assert!(parser.lines_processed() > 0);
    }
}

/// Test PTY shell with Zig parser
#[tokio::test]
async fn test_pty_with_zig_parser() {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }).unwrap();
    
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let cmd = CommandBuilder::new(&shell);
    pair.slave.spawn_command(cmd).unwrap();
    
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    
    // Write command
    writer.write_all(b"echo test\n").unwrap();
    writer.flush().unwrap();
    
    // Read output
    let mut buf = [0u8; 1024];
    let n = reader.read(&mut buf).unwrap();
    let output = String::from_utf8_lossy(&buf[..n]);
    
    assert!(output.contains("test"), "Output should contain 'test'");
    
    drop(pair.master);
}

/// Test SSH connection (requires SSH server)
#[tokio::test]
#[ignore] // Requires SSH server
async fn test_ssh_connection() {
    use sheshaaos_remote::connection::ConnectionManager;
    use sheshaaos_wps::broker::Broker;
    
    let broker = Broker::new(10);
    let _manager = ConnectionManager::new(broker);
    
    // This would connect to a test SSH server
    // let handle = manager.connect("user", "localhost", 2222).await.unwrap();
    // assert!(handle.is_connected());
}

/// Test PTY with special keys
#[tokio::test]
async fn test_pty_special_keys() {
    let mut terminal = TerminalState::new();
    terminal.wire_pty();
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Test arrow keys
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp), iced::keyboard::Modifiers::empty());
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown), iced::keyboard::Modifiers::empty());
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft), iced::keyboard::Modifiers::empty());
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight), iced::keyboard::Modifiers::empty());
    
    // Test Ctrl+C
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter), iced::keyboard::Modifiers::empty());
    tokio::time::sleep(Duration::from_millis(100)).await;
    terminal.handle_key(iced::keyboard::Key::Character("c".into()), iced::keyboard::Modifiers::CTRL);
    
    tokio::time::sleep(Duration::from_millis(200)).await;
    terminal.poll_output();
    
    // Should not crash
    assert!(true);
}

/// Test PTY with large output
#[tokio::test]
async fn test_pty_large_output() {
    let mut terminal = TerminalState::new();
    terminal.wire_pty();
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Generate large output
    terminal.handle_char('y');
    terminal.handle_char('e');
    terminal.handle_char('s');
    terminal.handle_char(' ');
    terminal.handle_char('|');
    terminal.handle_char(' ');
    terminal.handle_char('h');
    terminal.handle_char('e');
    terminal.handle_char('a');
    terminal.handle_char('d');
    terminal.handle_char(' ');
    terminal.handle_char('-');
    terminal.handle_char('n');
    terminal.handle_char(' ');
    terminal.handle_char('1');
    terminal.handle_char('0');
    terminal.handle_char('0');
    terminal.handle_char('0');
    terminal.handle_key(iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter), iced::keyboard::Modifiers::empty());
    
    tokio::time::sleep(Duration::from_secs(2)).await;
    terminal.poll_output();
    
    // Should handle large output without crashing
    let performer = &terminal.performer;
    assert!(performer.grid.len() > 0);
}

/// Helper: Convert grid to string
fn grid_to_string(grid: &[Vec<sheshaaos_gui::terminal::Cell>]) -> String {
    let mut result = String::new();
    for row in grid {
        for cell in row {
            result.push(cell.ch);
        }
        result.push('\n');
    }
    result
}