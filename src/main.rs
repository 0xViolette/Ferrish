use std::{
    io::{self, Write},
    process::exit,
};

enum Command<'a> {
    Exit,
    Empty,
    Echo,
    Type,
    Invalid(&'a str),
}

struct ParsedInput<'a> {
    command: Command<'a>,
    arguments: Vec<&'a str>,
}

fn parse_command(s: &str) -> Command<'_> {
    match s {
        "exit" => Command::Exit,
        "echo" => Command::Echo,
        "type" => Command::Type,
        _ => Command::Invalid(s),
    }
}

fn parse(input: &str) -> ParsedInput<'_> {
    let mut parts = input.trim_end().split_whitespace();

    let command = match parts.next() {
        Some(s) => parse_command(s),
        None => Command::Empty,
    };

    ParsedInput {
        command,
        arguments: parts.collect(),
    }
}

fn handle_echo<'a>(arguments: Vec<&'_ str>) {
    println!("{}", arguments.join(" "));
}

fn handle_type<'a>(arguments: Vec<&'_ str>) {
    for arg in arguments.iter() {
        match parse_command(arg) {
            Command::Exit | Command::Type | Command::Echo => {
                println!("{arg} is a shell builtin");
            }
            Command::Invalid(_) => println!("{arg}: not found"),
            Command::Empty => {}
        }
    }
}

fn run(input: ParsedInput) {
    match input.command {
        Command::Invalid(name) => println!("{name}: command not found"),
        Command::Empty => {}
        Command::Exit => exit(0),
        Command::Echo => handle_echo(input.arguments),
        Command::Type => handle_type(input.arguments),
    }
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        // `unwrap_or(0)` == 0 when Ctrl+D (EOF signal) or there was some error in reading the
        // input
        if io::stdin().read_line(&mut input).unwrap_or(0) == 0 {
            break;
        }

        let parsed_input = parse(&input.as_str());
        run(parsed_input);
    }
}
