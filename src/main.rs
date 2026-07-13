use std::{
    io::{self, Write},
    process::exit,
};

enum Command<'a> {
    Exit,
    External(&'a str),
}

struct ParsedInput<'a> {
    command: Command<'a>,
    options: Vec<&'a str>,
}

fn parse_input(input: &str) -> ParsedInput<'_> {
    let command = match input.trim_end() {
        "exit" => Command::Exit,
        invalid => Command::External(invalid),
    };

    ParsedInput {
        command,
        options: vec![],
    }
}

fn run(input: ParsedInput) {
    match input.command {
        Command::External(name) => println!("{name}: command not found"),
        Command::Exit => exit(0),
    }
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Could not read the command");

        let parsed_input = parse_input(&input.as_str());
        run(parsed_input);
    }
}
