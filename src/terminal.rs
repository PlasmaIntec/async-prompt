use std::str::{FromStr, from_utf8};
use std::{io};
use std::io::{Write, Error};
use std::mem::zeroed;
use std::sync::{Mutex};
use libc::STDIN_FILENO;
use tokio::io::AsyncReadExt;
use tokio::{select, time};
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::sync::{mpsc};
use tokio::task::JoinHandle;

use crate::{line_buffer::LineBuffer};

nix::ioctl_read_bad!(win_size, libc::TIOCGWINSZ, libc::winsize);

const MAX_TRANSACTION_TIME: u64 = 30;

enum TerminalState {
    TIMER(usize),
    NORMAL
}

#[allow(dead_code)]
pub struct Terminal {
    state: TerminalState,
    prompt_thread: Option<JoinHandle<Result<(), Error>>>,
    prompt_tx: Sender<u64>,
    prompt_rx: Receiver<u64>,
    line_thread: JoinHandle<Result<(), Error>>,
    line_tx: Sender<String>,
    line_rx: Receiver<String>,
    prompt: LineBuffer,
    line: LineBuffer
}

impl Terminal {
    pub async fn new() -> Terminal {
        let (prompt_tx, prompt_rx) = mpsc::channel(1);
        let (line_tx, line_rx) = mpsc::channel(1);
        let prompt_thread = None;
        let line_thread = Terminal::subscribe_to_stdin(line_tx.clone()).await;
        let prompt = LineBuffer { string: Mutex::new(String::from_str("> ").unwrap()) };
        let line = LineBuffer { string: Mutex::new(String::new()) };

        Terminal { 
            state: TerminalState::NORMAL,  
            prompt_thread,
            prompt_tx,
            prompt_rx,
            line_thread,
            line_tx,
            line_rx,
            prompt, 
            line
        }
    }

    pub async fn begin_loop(&mut self) {    
        loop {
            self.refresh_line();
            select! {
                new_time = self.prompt_rx.recv() => {
                    if let Some(new_time) = new_time {
                        if new_time > 0 {
                            self.prompt.replace_string(format!("{}> ", new_time.to_string()));
                        } else {
                            self.prompt.replace_string("> ".to_string());
                        }
                    }
                }
                new_char = self.line_rx.recv() => {
                    if let Some(new_char) = new_char {
                        if new_char == "\n" {
                            self.handle_line().await;
                        } else if new_char == "\u{7f}" && self.line.get_string().len() > 0 {
                            let len = self.line.get_string().len();
                            self.line.replace_string(self.line.get_string()[..len-1].to_string());
                        } else {
                            self.line.append_string(new_char);
                        }
                    }
                }
            }
        }
    }

    // currently clears only line cursor is on
    // TODO: deal with line wrapping
    fn clear_old_rows(&self, buffer: &mut String) {
        buffer.push_str("\r\x1b[0K");
    }

    fn refresh_line(&mut self) {
        let mut buffer = String::new();

        {
            self.clear_old_rows(&mut buffer);
        }

        buffer.push_str(&self.prompt.get_string().to_owned());
        buffer.push_str(&self.line.get_string().to_owned());

        io::stdout().write_all(buffer.as_bytes()).unwrap();
        io::stdout().flush().unwrap();
    }

    async fn handle_line(&mut self) {
        let line = self.line.get_string();
        if line == "start" {
            self.state = TerminalState::TIMER(30);
            if let Some(prompt_thread) = self.prompt_thread.take() {
                prompt_thread.abort();
            }
            self.prompt_thread = Some(Terminal::subscribe_to_timer(self.prompt_tx.clone()).await);
        } else if line == "stop" {
            self.state = TerminalState::NORMAL;
            self.prompt.replace_string("> ".to_string());
            if let Some(prompt_thread) = self.prompt_thread.take() {
                prompt_thread.abort();
            }
        }
        println!("\n{}", line);
        self.line.replace_string(String::new());
    }

    async fn subscribe_to_timer(tx: Sender<u64>) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(async move {
            let start = time::Instant::now();
            let mut interval = time::interval(time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let elapsed = MAX_TRANSACTION_TIME - start.elapsed().as_secs();
                tx.send(elapsed).await.unwrap();
                if elapsed <= 0 {
                    break;
                }
            }
            Ok(())
        })
    }

    async fn subscribe_to_stdin(tx: Sender<String>) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(async move {
            loop {
                let stdout = io::stdout();
                let mut reader = tokio::io::stdin();
                let mut buffer = [0;1];  // read exactly one byte
                stdout.lock().flush().unwrap();
                reader.read_exact(&mut buffer).await.unwrap();
                tx.send(from_utf8(&vec![buffer[0]]).unwrap().to_string()).await.unwrap();
            }
        })
    }

    // TODO: use in conjunction with clear_old_rows
    #[allow(dead_code)]
    fn get_terminal_size() -> (usize, usize) {
        unsafe {
            let mut size: libc::winsize = zeroed();
            match win_size(STDIN_FILENO, &mut size) {
                Ok(0) => {
                    // In linux pseudo-terminals are created with dimensions of
                    // zero. If host application didn't initialize the correct
                    // size before start we treat zero size as 80 columns and
                    // infinite rows
                    let cols = if size.ws_col == 0 {
                        80
                    } else {
                        size.ws_col as usize
                    };
                    let rows = if size.ws_row == 0 {
                        usize::MAX
                    } else {
                        size.ws_row as usize
                    };
                    (cols, rows)
                }
                _ => (80, 24),
            }
        }
    }
}
