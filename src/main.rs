extern crate termion;

use std::env;
use std::process;
use std::io::{self, Write};
use termion::raw::IntoRawMode;
use termion::event::Key;
use termion::input::TermRead;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().into_raw_mode().unwrap();
    
    let mut input_line = String::new();
    print!("\r");
    print!("? ");
    stdout.flush().unwrap();
    
    for key_result in stdin.keys() {
        let key = key_result.unwrap(); 
        match key {
            Key::Char('\n') => {
                parse(&input_line);
                print!("? ");
                stdout.flush().unwrap();
                input_line.clear();
            }
            Key::Char(key) => {
                input_line.push(key);
                print!("{}", key);
                stdout.flush().unwrap();
            }
            Key::Backspace => {
                if !input_line.is_empty() {
                    input_line.pop();
                    print!("\x08 \x08");
                    stdout.flush().unwrap();
                }
            }
            Key::Ctrl('c') => {
                print!("\r\n");
                print!("? ");
                stdout.flush().unwrap();
                input_line.clear();
            }
            _ => {
            }
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

    print!("\r\n");
    let mut child = Command::new(path)
        .args(arguments)
        .spawn()
        .expect("Failed to execute command");
    child.wait().expect("Failed to wait on child");
    print!("\r");
}

fn builtin(command: &str, arguments: &[&str]) -> bool {
    match command {
        "cd" => {
            if let Some(arg) = arguments.get(0) {
                let new_dir = Path::new(arg);
                if !env::set_current_dir(&new_dir).is_ok() {
                    println!("Failed to change directory");
                }
            } else {
                print!("\r\nNo arguments to cd");
            }
            return true;
        }
        _ => false,
    }
}

