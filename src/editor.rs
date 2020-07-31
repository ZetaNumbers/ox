// Editor.rs - For controling the current editor
use termion::{color, style};
use termion::input::TermRead;
use termion::event::Key;
use crate::Terminal;
use crate::Buffer;
use std::time::Duration;
use std::cmp::min;
use std::thread;
use std::env;

// Get the version of Ox
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(0, 175, 135));
const FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(38, 38, 38));

// For holding the position and directions of the cursor
pub struct Cursor {
    x: u16,
    y: u16,
}

// For holding our editor information
pub struct Editor {
    terminal: Terminal,
    kill: bool,
    cursor: Cursor,
    buffer: Buffer,
    offset: u64,
    command_bar: String,
}

impl Editor {
    pub fn new() -> Self {
        // Create a new editor instance
        let args: Vec<String> = env::args().collect();
        let buffer: Buffer;
        if args.len() <= 1 { 
            buffer = Buffer::new();
        } else {
            buffer = Buffer::open(args[1].trim());
        }
        Self {
            terminal: Terminal::new(),
            kill: false,
            cursor: Cursor { x: 0, y: 0 },
            buffer,
            offset: 0,
            command_bar: String::from("Welcome to Ox!"),
        }

    }
    pub fn run(&mut self) {
        let mut stdin = termion::async_stdin().keys();
        // Run our editor
        loop {
            // Exit if required
            if self.kill {
                self.terminal.clear_all();
                self.terminal.move_cursor(0, 0);
                break; 
            }
            // Render our interface
            self.render();
            // Read a key
            match stdin.next() {
                Some(key) => match key.unwrap() {
                    Key::Ctrl('q') => self.kill = true, // Exit
                    Key::Left => {
                        // Move cursor to the left
                        let current = self.cursor.y + self.offset as u16;
                        if self.cursor.x == 0 && current != 0 {
                            if self.cursor.y == 0 { 
                                self.offset = self.offset.saturating_sub(1); 
                            }
                            self.cursor.x = self.terminal.width;
                            self.cursor.y = self.cursor.y.saturating_sub(1);
                            self.correct_line();
                        } else {
                            self.cursor.x = self.cursor.x.saturating_sub(1);
                        }
                    }
                    Key::Right => {
                        // Move cursor to the right
                        let index = self.cursor.y + self.offset as u16;
                        if self.buffer.lines.is_empty() {
                            continue;
                        }
                        let current = &self.buffer.lines[index as usize];
                        let size = [
                            &self.terminal.width,
                            &self.terminal.height,
                        ];
                        if current.len() as u16 == self.cursor.x && 
                           self.buffer.lines.len() as u16 != index + 1 {
                            if self.cursor.y == size[1] - 3 { 
                                self.offset = self.offset.saturating_add(1); 
                            } else {
                                self.cursor.y = self.cursor.y.saturating_add(1);
                            }
                            self.cursor.x = 0;
                        } else if self.cursor.x < size[0].saturating_sub(1) {
                            self.cursor.x = self.cursor.x.saturating_add(1);
                            self.correct_line();
                        }
                    }
                    Key::Up => {
                        // Move cursor up
                        if self.cursor.y != 0 {
                            self.cursor.y = self.cursor.y.saturating_sub(1);
                            self.correct_line();
                        } else {
                            self.offset = self.offset.saturating_sub(1);
                        }
                    }
                    Key::Down => {
                        // Move cursor down
                        let buff_len = self.buffer.lines.len() as u64;
                        let proposed = self.cursor.y.saturating_add(1) as u64;
                        let max = self.terminal.height.saturating_sub(3);
                        if proposed.saturating_add(self.offset) < buff_len {
                            if self.cursor.y < max {
                                self.cursor.y = proposed as u16;
                                self.correct_line();
                            } else {
                                self.offset = self.offset.saturating_add(1);
                            }
                        }
                    }
                    Key::PageUp => {
                        // Move the cursor to the top of the terminal
                        self.cursor.y = 0;
                        self.correct_line();
                    }
                    Key::PageDown => {
                        // Move the cursor to the bottom of the buffer / terminal
                        let t = self.terminal.height.saturating_sub(3) as u16;
                        let b = self.buffer.lines.len().saturating_sub(1) as u16;
                        self.cursor.y = min(t, b);
                        self.correct_line();
                    }
                    Key::Home => {
                        // Move to the start of the current line
                        self.cursor.x = 0;
                    }
                    Key::End => {
                        // Move to the end of the current line
                        self.cursor.x = self.terminal.width.saturating_sub(1);
                        self.correct_line();
                    }
                    Key::Char(c) => {
                        self.insert(c);
                    }
                    Key::Backspace => {
                        self.delete();
                    }
                    _ => (), // Unbound key
                }
                None => {
                    self.terminal.check_resize(); // Check for resize
                    // FPS cap to stop greedy CPU usage
                    thread::sleep(Duration::from_millis(24));
                }
            }
        }
    }
    fn insert(&mut self, c: char) {
        self.buffer.lines[
            (self.cursor.y + self.offset as u16) as usize
        ].push(c);
        self.cursor.x = self.cursor.x.saturating_add(1);
    }
    fn delete(&mut self) {
        if self.cursor.x != 0 {
          self.cursor.x = self.cursor.x.saturating_sub(1);
          let index = self.cursor.y + self.offset as u16;
          let start = self.cursor.x.saturating_sub(1) as usize;
          let end = self.cursor.x.saturating_add(1) as usize;
          let start = self.buffer.lines[index as usize][..=start].to_string();
          let end = self.buffer.lines[index as usize][end..].to_string();
          self.buffer.lines[index as usize] = start + &end;
        }
    }
    fn correct_line(&mut self) {
        // Ensure that the cursor isn't out of bounds
        if self.buffer.lines.is_empty() { 
            self.cursor.x = 0;
        } else {
            let current = self.buffer.lines[
                (self.cursor.y + self.offset as u16) as usize
            ].clone();
            if self.cursor.x > current.len() as u16 {
                self.cursor.x = current.len() as u16;
            }
        }
    }
    fn render(&mut self) {
        // Render the rows
        let term_length = self.terminal.height;
        let mut frame: Vec<String> = Vec::new();
        for row in 0..self.terminal.height {
            if row == self.terminal.height / 3 && self.buffer.lines.is_empty() {
                let welcome = format!("Ox editor v{}", VERSION);
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!("{}{}{}", "~", pad, welcome));
            } else if row == (self.terminal.height / 3) + 2 && 
                self.buffer.lines.is_empty()  {
                let welcome = "A speedy editor built with Rust";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!("{}{}{}", "~", pad, welcome));
            } else if row == (self.terminal.height / 3) + 3 && 
                self.buffer.lines.is_empty()  {
                let welcome = "by curlpipe";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!("{}{}{}", "~", pad, welcome));
            } else if row == (self.terminal.height / 3) + 5 && 
                self.buffer.lines.is_empty()  {
                let welcome = "Ctrl + Q:  Exit";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!(
                    "{}{}{}{}{}", "~", 
                    pad, 
                    color::Fg(color::Blue),
                    welcome,
                    color::Fg(color::Reset),
                ));
            } else if row == term_length - 2 {
                let status_line = format!(
                    " Ox: {} | x: {} | y: {}", 
                    VERSION,
                    self.cursor.x, 
                    self.cursor.y,
                );
                let pad = self.terminal.width as usize - status_line.len();
                let pad = " ".repeat(pad);
                frame.push(format!(
                    "{}{}{}{}{}{}{}{}", 
                    FG, BG, style::Bold,
                    status_line, pad,
                    color::Fg(color::Reset), color::Bg(color::Reset), style::Reset,
                ));
            } else if row == term_length - 1 {
                frame.push(self.command_bar.clone());
            } else if row < self.buffer.lines.len() as u16 {
                let index = self.offset as usize + row as usize;
                frame.push(self.buffer.lines[index].clone());
            } else {
                frame.push(String::from("~"));
            }
        }
        self.terminal.clear_all();
        self.terminal.move_cursor(0, 0);
        print!("{}", frame.join("\r\n"));
        self.terminal.move_cursor(self.cursor.x, self.cursor.y);
        self.terminal.flush();
    }
}

