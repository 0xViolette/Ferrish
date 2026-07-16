use std::{
    io::{self, Write},
    process::exit,
};

enum Command<'a> {
    Exit,
    Empty,
    Echo,
    Invalid(&'a str),
}

struct ParsedInput<'a> {
    command: Command<'a>,
    arguments: Vec<&'a str>,
}

fn parse(input: &str) -> ParsedInput<'_> {
    // let command = match input.trim_end() {
    //     "exit" => Command::Exit,
    //     "" => Command::Empty,
    //     invalid => Command::Invalid(invalid),
    // };

    let mut parts = input.trim_end().split_whitespace();
    let command_string = parts.next();
    let args_vector: Vec<&'_ str> = parts.collect();

    let command = match command_string {
        Some("exit") => Command::Exit,
        Some("echo") => Command::Echo,
        None => Command::Empty,
        _ => Command::Invalid(command_string.unwrap()),
    };

    ParsedInput {
        command,
        arguments: args_vector,
    }
}

fn handle_echo<'a>(arguments: Vec<&'_ str>) {
    println!("{}", arguments.join(" "));
}

fn run(input: ParsedInput) {
    match input.command {
        Command::Invalid(name) => println!("{name}: command not found"),
        Command::Empty => {}
        Command::Exit => exit(0),
        Command::Echo => handle_echo(input.arguments),
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
