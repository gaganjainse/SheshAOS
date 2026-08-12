use shesh_gui::terminal::{TermPerformer, CellAttr, TermColor};
use vte::Parser;
use image::{ImageBuffer, Rgba};

/// Visual regression test for terminal rendering
#[test]
fn test_terminal_rendering_basic() {
    let mut performer = TermPerformer::new(24, 80);
    let mut parser = Parser::new();
    
    // Feed some text with ANSI colors
    let input = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    
    // Verify grid state
    // R(0) e(1) d(2) [reset] (3=space) G(4) r(5) e(6) e(7) n(8) [reset] (9=space) B(10) l(11) u(12) e(13)
    assert_eq!(performer.grid[0][0].ch, 'R');
    assert_eq!(performer.grid[0][0].attr.fg, TermColor::Indexed(1)); // Red
    assert_eq!(performer.grid[0][3].ch, ' ');
    assert_eq!(performer.grid[0][4].ch, 'G');
    assert_eq!(performer.grid[0][4].attr.fg, TermColor::Indexed(2)); // Green
    assert_eq!(performer.grid[0][10].ch, 'B');
    assert_eq!(performer.grid[0][10].attr.fg, TermColor::Indexed(4)); // Blue
}

/// Test cursor positioning rendering
#[test]
fn test_cursor_positioning() {
    let mut performer = TermPerformer::new(24, 80);
    let mut parser = Parser::new();
    
    // Move cursor and write
    let input = "Hello\x1b[10;20HWorld";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    
    // After writing "Hello" (5 chars), cursor at col 5
    // Then \x1b[10;20H moves to row 10, col 20 (1-indexed) = row 9, col 19 (0-indexed)
    // Then "World" is written starting at col 19, cursor ends at col 24
    assert_eq!(performer.cursor_row, 9);  // 0-indexed
    assert_eq!(performer.cursor_col, 24); // 19 + 5 chars of "World"
    assert_eq!(performer.grid[9][19].ch, 'W');
}

/// Test scrollback rendering
#[test]
fn test_scrollback_rendering() {
    let mut performer = TermPerformer::new(5, 10); // Small grid
    let mut parser = Parser::new();
    
    // Fill more lines than grid height
    for i in 0..10 {
        let line = format!("Line {}\r\n", i);
        for byte in line.bytes() {
            parser.advance(&mut performer, byte);
        }
    }
    
    // With 5-row grid and 10 input lines, scrollback contains earlier lines
    // The exact count depends on when scroll triggers
    assert!(performer.scrollback.len() > 0, "Should have scrollback entries");
    assert_eq!(performer.grid.len(), 5);
}

/// Test SGR attribute rendering (bold, italic, underline)
#[test]
fn test_sgr_attributes() {
    let mut performer = TermPerformer::new(24, 80);
    let mut parser = Parser::new();
    
    let input = "\x1b[1mBold\x1b[0m \x1b[3mItalic\x1b[0m \x1b[4mUnderline\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    
    // Check bold
    assert!(performer.grid[0][0].attr.bold);
    assert!(!performer.grid[0][5].attr.bold); // After reset
    
    // Check italic
    assert!(performer.grid[0][6].attr.italic);
    assert!(!performer.grid[0][12].attr.italic); // After reset
    
    // Check underline
    assert!(performer.grid[0][13].attr.underline);
    assert!(!performer.grid[0][22].attr.underline); // After reset
}

/// Test 24-bit true color rendering
#[test]
fn test_true_color() {
    let mut performer = TermPerformer::new(24, 80);
    let mut parser = Parser::new();
    
    // RGB foreground: \x1b[38;2;R;G;Bm
    let input = "\x1b[38;2;255;128;64mOrange\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    
    match performer.grid[0][0].attr.fg {
        TermColor::Rgb(r, g, b) => {
            assert_eq!(r, 255);
            assert_eq!(g, 128);
            assert_eq!(b, 64);
        }
        _ => panic!("Expected RGB color"),
    }
}

/// Generate reference images for visual regression testing
#[test]
#[ignore] // Run manually to generate references
fn generate_reference_images() {
    let mut performer = TermPerformer::new(24, 80);
    let mut parser = Parser::new();
    
    // Test case 1: Basic colors
    let input = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    save_grid_as_image(&performer, "tests/visual/references/basic_colors.png");
    
    // Test case 2: Bold/italic/underline
    performer = TermPerformer::new(24, 80);
    parser = Parser::new();
    let input = "\x1b[1mBold\x1b[0m \x1b[3mItalic\x1b[0m \x1b[4mUnderline\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    save_grid_as_image(&performer, "tests/visual/references/attributes.png");
    
    // Test case 3: True color
    performer = TermPerformer::new(24, 80);
    parser = Parser::new();
    let input = "\x1b[38;2;255;128;64mOrange\x1b[0m \x1b[48;2;0;255;0mGreenBG\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    save_grid_as_image(&performer, "tests/visual/references/true_color.png");
}

/// Compare rendered output with reference image
#[test]
#[ignore] // Requires reference image file
fn test_visual_regression_basic_colors() {
    let mut performer = TermPerformer::new(24, 80);
    let mut parser = Parser::new();
    
    let input = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m";
    for byte in input.bytes() {
        parser.advance(&mut performer, byte);
    }
    
    let rendered = render_grid_to_image(&performer);
    let reference = load_reference_image("tests/visual/references/basic_colors.png");
    
    assert_images_equal(&rendered, &reference, 0.99); // 99% similarity threshold
}

/// Helper: Render grid to image
fn render_grid_to_image(performer: &TermPerformer) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let cell_w = 8;
    let cell_h = 16;
    let width = performer.cols * cell_w;
    let height = performer.rows * cell_h;
    let mut img = ImageBuffer::new(width as u32, height as u32);
    
    for (r, row) in performer.grid.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let x = c * cell_w;
            let y = r * cell_h;
            let color = cell_attr_to_rgba(&cell.attr, cell.ch != ' ');
            for dy in 0..cell_h {
                for dx in 0..cell_w {
                    img.put_pixel((x + dx) as u32, (y + dy) as u32, color);
                }
            }
        }
    }
    img
}

fn cell_attr_to_rgba(_attr: &CellAttr, has_char: bool) -> Rgba<u8> {
    // Simplified - in real implementation would use actual colors
    if has_char {
        Rgba([255, 255, 255, 255])
    } else {
        Rgba([0, 0, 0, 255])
    }
}

fn save_grid_as_image(performer: &TermPerformer, path: &str) {
    let img = render_grid_to_image(performer);
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    img.save(path).unwrap();
}

fn load_reference_image(path: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    image::open(path).unwrap().to_rgba8()
}

fn assert_images_equal(img1: &ImageBuffer<Rgba<u8>, Vec<u8>>, img2: &ImageBuffer<Rgba<u8>, Vec<u8>>, threshold: f32) {
    assert_eq!(img1.dimensions(), img2.dimensions());
    let mut matches: f32 = 0.0;
    let total = (img1.width() * img1.height()) as f32;
    
    for (p1, p2) in img1.pixels().zip(img2.pixels()) {
        if p1 == p2 {
            matches += 1.0;
        }
    }
    
    let similarity = matches / total;
    assert!(similarity >= threshold, "Image similarity {} below threshold {}", similarity, threshold);
}