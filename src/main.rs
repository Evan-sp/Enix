use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::env;

fn main() {
    loop {
        let path = env::current_dir().expect("Error getting current directory");
        print!("{}? ", path.display());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                if input == "exit" || input == "quit" {
                    println!("Goodbye");
                    break;
                }
                parse(input);
            }
            Err(error) => {
                println!("Error reading input: {}", error);
            }
        }
    }
}

fn parse(input: &str) {
    let words: Vec<&str> = input.split_whitespace().collect();
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
        println!("File {} does not exist", command);
        return;
    }
    if !path.is_file() {
        println!("'{}' is not a file", command);
        return;
    }
   
    let mut child = Command::new(path)
        .args(arguments)
        .spawn()
        .expect("Failed to execute command");
    child.wait().expect("Failed to wait on child");

}

fn builtin(command: &str, arguments: &[&str]) -> bool {
    match command {
        "cd" => { 
            println!("Match for cd builtin");
            if let Some(arg) = arguments.get(0) {
                let new_dir = Path::new(arg);
                if !env::set_current_dir(&new_dir).is_ok() {
                    println!("Failed to change directory");
                }
            } else {
                println!("No arguments to cd");
            }
            return true
        }
        _ => false,
    }
}

