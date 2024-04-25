//! Tohle je program vytvořený v Rustu na základě kurzu

use slug::slugify;
use std::{env, io};

fn main() {
    println!("Zadejte vstup: ");

    let mut input: String = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input = input.trim().to_string();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Input: {input}");
        return;
    }

    println!("Args: {}", args[1]);

    if args[1] == "lowercase" {
        input = input.to_lowercase();
    } else if args[1] == "uppercase" {
        input = input.to_uppercase();
    } else if args[1] == "no-spaces" {
        input = input.replace(" ", "");
    } else if args[1] == "slugify" {
        input = slugify(&input);
    } else if args[1] == "reverse" {
        input = input.chars().rev().collect();
    } else if args[1] == "snake_case" {
        input = input.to_lowercase().replace(" ", "_");
    }

    println!("Input: {input}");
}
