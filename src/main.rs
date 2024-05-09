//! Tohle je program vytvořený v Rustu na základě kurzu

use csv::ReaderBuilder;
use std::error::Error;
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Error: No operation specified");
        return;
    }

    let operation = &args[1];

    let mut input = String::new();
    println!("Zadejte víceřádkový vstup (Ctrl+D pro ukončení):");
    if io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("Failed to read input");
        return;
    }

    let result = match operation.as_str() {
        "lowercase" => lowercase(&input),
        "uppercase" => uppercase(&input),
        "no-spaces" => no_spaces(&input),
        "slugify" => slugify(&input),
        "reverse" => reverse(&input),
        "snake_case" => snake_case(&input),
        "csv" => csv(&input),
        _ => {
            eprintln!("Error: Invalid operation");
            return;
        }
    };

    match result {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn lowercase(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(input.to_lowercase())
}

fn uppercase(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(input.to_uppercase())
}

fn no_spaces(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(input.replace(" ", ""))
}

fn slugify(input: &String) -> Result<String, Box<dyn Error>> {
    Ok(slug::slugify(input))
}

fn reverse(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(input.chars().rev().collect())
}

fn snake_case(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(input.to_lowercase().replace(" ", "_"))
}

fn csv(input: &str) -> Result<String, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().from_reader(input.as_bytes());
    let headers = rdr.headers()?.clone();
    let mut table = String::new();
    for record in rdr.records() {
        let record = record?;
        for (_header, field) in headers.iter().zip(record.iter()) {
            table.push_str(&format!("{:<20}", field)); // Přidáno zobrazení hlaviček
        }
        table.push('\n');
    }
    Ok(table)
}
