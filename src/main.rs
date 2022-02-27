use std::io;
use std::io::Write;
use std::os::unix::prelude::RawFd;
use std::sync::Arc;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use tokio::io::AsyncReadExt;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

const STDIN_FILENO: RawFd = libc::STDIN_FILENO;

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

    let notify = Arc::new(Notify::new());
    let notify_2 = notify.clone();
    let notify_3 = notify.clone();

    let t1 = tokio::spawn(async move {
        loop {
            let stdout = io::stdout();
            let mut reader = tokio::io::stdin();
            let mut buffer = [0;1];  // read exactly one byte
            print!("Hit a key! ");
            stdout.lock().flush().unwrap();
            reader.read_exact(&mut buffer).await.unwrap();
            notify_2.notify_one();
            println!("You have hit: {:?}", buffer);
        }
    });

    let t2 = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(1000)).await;
            notify_3.notify_one();
            println!("You have slept");
        }
    });
    
    loop {
        notify.notified().await;
        println!("notified");
    }

    unset_raw_mode();
}