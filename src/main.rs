use std::{env, io::{self, Write}, path::Path};

fn main() {
    let mut input = String::new();
    loop {
        input.clear();

        print!("? ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        println!("You entered: {}", input.trim());
        if !parse(&input) {
            break;
        }
    }
}

fn parse(input: &str) -> bool {
    let input = input.trim();
    if input == "q" || input == "exit" || input == "quit" {
        return false;
    }

    /*let words: Vec<&str> = input.split_whitespace().collect();
    let first_word = match words.get(0) {
        Some(word) => word,
        None => "No words found"
    };*/
    let mut split = input.splitn(2, ' ');
    let command = split.next().unwrap();
    let args = split.next().unwrap_or("");

    //println!("{}", first_word);
    //println!("{:?}", &words[1..]);

    launch(command, args);
    return true;
}

fn launch(command: &str, args: &str) -> bool {
    if builtin(command, args) {
        return true;
    }
    return true;
}

fn builtin(command: &str, args: &str) -> bool {
    println!("builtin: {}", command);
    match command {
        "cd" => {
            if !env::set_current_dir(Path::new(args)).is_ok() {
                println!("failed");
                return false;
            }
            println!("success");
            return true;
        }
        _ => false,
    }
}
