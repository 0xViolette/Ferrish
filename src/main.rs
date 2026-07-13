#[allow(unused_imports)]
use std::io::{self, Write};

struct ParsedInput<'a> {
    command: &'a str,
    options: Option<Vec<&'a str>>,
}

fn parse_input<'a>(input: &'a str) -> ParsedInput<'a> {
    ParsedInput {
        command: input.trim_end(),
        options: None,
    }
}

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Could not read the command");

    let parsed_input = parse_input(&input.as_str());

    println!("{}: command not found", parsed_input.command);
}
