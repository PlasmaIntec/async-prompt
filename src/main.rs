use libc::STDIN_FILENO;
use terminal::Terminal;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};

pub mod terminal;
pub mod line_buffer;

fn set_raw_mode() {
    let termios = Termios::from_fd(STDIN_FILENO).unwrap();
    let mut new_termios = termios.clone();  // make a mutable copy of termios 
                                            // that we will modify
    new_termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(STDIN_FILENO, TCSANOW, &mut new_termios).unwrap();
}

fn unset_raw_mode() {
    let termios = Termios::from_fd(STDIN_FILENO).unwrap();
    tcsetattr(STDIN_FILENO, TCSANOW, & termios).unwrap();  // reset the stdin to 
                                                    // original termios data
}

#[tokio::main]
async fn main() {
    set_raw_mode();

    let mut terminal = Terminal::new().await;

    terminal.begin_loop().await;

    unset_raw_mode();
}