use std::io;

fn main() {
    println!("Enter your name:");

    let mut name: String = String::new();

    io::stdin()
        .read_line(&mut name)
        .expect("Failed to read line");

    let name = name.trim();

    println!("Hello, {name}!");
}
