use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::exit,
};

use is_executable;

enum Command<'a> {
    Exit,
    Empty,
    Echo,
    Type,
    Executable(PathBuf),
    Invalid(&'a str),
}

struct ParsedInput<'a> {
    command: Command<'a>,
    arguments: Vec<&'a str>,
}

fn check_executable(s: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH").expect("PATH not set");

    for dir in env::split_paths(&path) {
        let base_path = dir.as_path();

        let full_path = base_path.join(s);
        if is_executable::is_executable(&full_path) {
            return Some(full_path);
        }
    }

    None
}

fn parse_command(s: &str) -> Command<'_> {
    match s {
        "exit" => Command::Exit,
        "echo" => Command::Echo,
        "type" => Command::Type,
        _ => {
            if let Some(full_path) = check_executable(s) {
                Command::Executable(full_path)
            } else {
                Command::Invalid(s)
            }
        }
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
            Command::Executable(path) => println!("{arg} is {}", path.display()),
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
        Command::Executable(_) => {}
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
