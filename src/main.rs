use std::io::{self, Write};

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
    println!("Parsing: {}", input);
}
