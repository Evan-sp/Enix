use std::io;
use std::process::Command;

fn main() {
    // Create a mutable string to store the input
    let mut input = String::new();

    while input.trim() != "q" {
        input.clear();

        let output = Command::new("ls")
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));

        println!("Please enter some input:");
        // Read from stdin and handle errors
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        // Output the input back to the user
        println!("You entered: {}", input.trim());
    }
}
