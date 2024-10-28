use std::io::{self, Read, Write};
use termion::raw::IntoRawMode;
use termion::event::Key;
use termion::input::TermRead;
use std::process;
use std::path::{Path, PathBuf};
use std::env;
use std::thread;

fn main() {
    //set raw mode for tty
    let _stdout = Some(io::stdout().into_raw_mode().unwrap());

    print!("\r");
    print!("? ");
    io::stdout().flush().unwrap();

    let mut input_line = String::new();
    for key_result in io::stdin().keys() {
        let key = key_result.unwrap();
        match key {
            Key::Char('\n') => {
                parse(&input_line);
                print!("? ");
                io::stdout().flush().unwrap();
                input_line.clear();
            }
            Key::Char(key) => {
                input_line.push(key);
                print!("{}", key);
                io::stdout().flush().unwrap();
            }
            Key::Backspace => {
                if !input_line.is_empty() {
                    input_line.pop();
                    print!("\x08 \x08");
                    io::stdout().flush().unwrap();
                }
            }
            Key::Ctrl('c') => {
                print!("\r\n");
                print!("? ");
                io::stdout().flush().unwrap();
                input_line.clear();
            }
            Key::Ctrl('l') => {
                print!("\x1B[2J\x1B[H");
                print!("? ");
                io::stdout().flush().unwrap();
            }
            _ => {}
        }
    }
}

fn parse(line: &str) {
    let line = line.trim();
    if line.len() == 0 {
        print!("\r\n");
        return;
    }
    if line == "q" || line == "exit" || line == "quit" {
        process::exit(0);
    }

    let words: Vec<&str> = line.split_whitespace().collect();
    let first_word = match words.get(0) {
        Some(word) => word,
        None => "No words found",
    };

    launch(first_word, &words[1..]);
}

fn launch(command: &str, arguments: &[&str]) {
    if builtin(command, arguments) {
        return;
    }

    let mut path = PathBuf::from(command);
    if let Ok(path_var) = env::var("PATH") {
        for split_path in env::split_paths(&path_var) {
            let full_path = split_path.join(command);
            if full_path.exists() {
                path = full_path;
            }
        }
    }
    if !path.exists() {
        print!("\r\n");
        print!("File {} does not exist", command);
        print!("\r\n");
        return;
    }
    if !path.is_file() {
        print!("\r\n");
        print!("'{}' is not a file", command);
        print!("\r\n");
        return;
    }

    spawn_with_tty();

    print!("\r\n");
}

fn builtin(command: &str, arguments: &[&str]) -> bool {
    match command {
        "cd" => {
            if let Some(arg) = arguments.get(0) {
                let new_dir = Path::new(arg);
                if !env::set_current_dir(&new_dir).is_ok() {
                    print!("\n\r");
                    print!("Failed to change directory");
                }
                print!("\n\r");
            } else {
                print!("\n\r");
                print!("No arguments to cd");
                print!("\n\r");
            }
            return true;
        }
        _ => false,
    }
}

fn spawn_with_tty() {
    let (current_cols, current_rows) = termion::terminal_size().unwrap();
    let pty_system = portable_pty::native_pty_system();
    let pty_size: portable_pty::PtySize = portable_pty::PtySize {
        rows: current_rows,
        cols: current_cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system.openpty(pty_size).unwrap();
    let mut cmd = portable_pty::CommandBuilder::new("sleep");
    cmd.arg("5");

    let mut child = pair.slave.spawn_command(cmd).unwrap();
    let mut reader = pair
        .master
        .try_clone_reader()
        .expect("Failed to clone PTY reader");

    let read_handle = thread::spawn(move || {
        let mut buf = [0; 1024];
        while let Ok(size) = reader.read(&mut buf) {
            if size == 0 {
                break;
            }
            
            io::stdout().write_all(&buf[..size]).unwrap();
            io::stdout().flush().unwrap();
        }
    });

    let write_handle = thread::spawn(move || {
        let mut master_for_writing = pair.master.take_writer().unwrap();
        let mut buf = [0; 1024];
        while let Ok(size) = io::stdin().read(&mut buf) {
            if size == 0 {
                break;
            }

            io::stdout().flush().unwrap();
            master_for_writing.write_all(&buf[..size]).unwrap();
            master_for_writing.flush().unwrap();
        }
    });

    child.wait().unwrap();
    read_handle.join().unwrap();
    write_handle.join().unwrap();
}
