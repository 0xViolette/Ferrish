use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
};

use is_executable;

enum ParseError {
    CommandNotFound(String),
}

enum Command {
    Exit,
    Echo,
    Type,
    Executable(PathBuf),
}

struct ParsedInput<'a> {
    command: Command,
    arguments: Vec<&'a str>,
}

fn check_executable(s: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH").expect("PATH not set");

    env::split_paths(&path)
        .map(|dir| dir.join(s))
        .find(|path| is_executable::is_executable(path))
}

fn parse_command(s: &str) -> Option<Command> {
    match s {
        "exit" => Some(Command::Exit),
        "echo" => Some(Command::Echo),
        "type" => Some(Command::Type),
        _ => {
            if let Some(full_path) = check_executable(s) {
                Some(Command::Executable(full_path))
            } else {
                None
            }
        }
    }
}

fn parse(input: &str) -> Result<ParsedInput<'_>, ParseError> {
    let mut parts = input.split_whitespace();
    let cmd_string = parts.next().unwrap();

    let command = parse_command(cmd_string)
        .ok_or_else(|| ParseError::CommandNotFound(cmd_string.to_string()))?;

    Ok(ParsedInput {
        command,
        arguments: parts.collect(),
    })
}

fn handle_echo<'a>(arguments: Vec<&'_ str>) {
    println!("{}", arguments.join(" "));
}

fn handle_type(arguments: Vec<&str>) {
    for arg in arguments.iter() {
        if let Some(cmd) = parse_command(arg) {
            match cmd {
                Command::Executable(path) => println!("{arg} is {}", path.display()),
                _ => println!("{arg} is a shell builtin"),
            }
        } else {
            println!("{arg}: not found");
        }
    }
}

fn handle_executable(executable_path: &Path, args: &[&str]) {
    process::Command::new(executable_path.file_name().unwrap())
        .args(args)
        .status()
        .expect("process failed to execute");
}

fn run(input: ParsedInput) {
    match input.command {
        Command::Exit => process::exit(0),
        Command::Echo => handle_echo(input.arguments),
        Command::Type => handle_type(input.arguments),
        Command::Executable(cmd) => handle_executable(&cmd, &input.arguments),
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
        if input.trim().is_empty() {
            continue;
        }

        match parse(&input) {
            Ok(parsed) => run(parsed),
            Err(ParseError::CommandNotFound(cmd)) => {
                println!("{cmd}: command not found");
            }
        }
    }
}
