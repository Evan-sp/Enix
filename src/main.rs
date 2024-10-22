use std::io::{self, Write};
use std::fs;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn main() {
    loop {
        print!("? ");
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
    launch(first_word);
}

fn launch(command: &str) {
    let path = Path::new(command);
    if !path.exists() {
        println!("File {} does not exist", command);
        return;
    }
    if !path.is_file() {
        println!("'{}' is not a file", command);
        return;
    }
    
    let metadata = fs::metadata(path).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode();
    let user_read = mode & 0o400 != 0;
    let user_write = mode & 0o200 != 0;
    let user_exec = mode & 0o100 != 0;

    println!("File '{}':", command);
    println!("  Read permission: {}", user_read);
    println!("  Write permission: {}", user_write);
    println!("  Execute permission: {}", user_exec);
    
    let output = Command::new(path)
        .output()
        .expect("Failed to execute the command");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout.trim());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error output:\n {}", stderr);
    }
}

