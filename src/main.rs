use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
};

use crossterm;
use is_executable;

struct CommandPipeline {
    commands: Vec<Command>,
}

struct Builtin {
    name: &'static str,
    handler: fn(&[&str]) -> Result<(), String>,
}

static BUILTINS: [Builtin; 5] = [
    Builtin {
        name: "exit",
        handler: handle_exit,
    },
    Builtin {
        name: "echo",
        handler: handle_echo,
    },
    Builtin {
        name: "pwd",
        handler: handle_pwd,
    },
    Builtin {
        name: "cd",
        handler: handle_cd,
    },
    Builtin {
        name: "type",
        handler: handle_type,
    },
];

fn cleanup() {
    if matches!(crossterm::terminal::is_raw_mode_enabled(), Ok(true)) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn handle_exit(_: &[&str]) -> Result<(), String> {
    cleanup();

    process::exit(0);
}

enum Command {
    Builtin(&'static Builtin),
    Executable(PathBuf),
}

struct ParsedInput<'a> {
    command: Command,
    arguments: Vec<&'a str>,
}

fn check_prefix(v: &[&str], p: &str) -> bool {
    v.iter().all(|s| s.starts_with(p))
}

fn longest_common_prefix(v: &[&str]) -> String {
    let mut pivot = String::new();
    for c in v[0].chars() {
        if !check_prefix(v, &pivot) {
            let mut ret = pivot.to_string();
            ret.pop();
            return ret;
        } else {
            pivot += &format!("{c}");
        }
    }
    if !check_prefix(v, &pivot) {
        let mut ret = pivot.to_string();
        ret.pop();
        return ret;
    } else {
        pivot.to_string()
    }
}

fn check_executable(s: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH").expect("PATH not set");

    env::split_paths(&path)
        .map(|dir| dir.join(s))
        .find(|path| is_executable::is_executable(path))
}

fn parse_command(s: &str) -> Option<Command> {
    BUILTINS
        .iter()
        .find(|b| b.name == s)
        .map(Command::Builtin)
        .or_else(|| check_executable(s).map(Command::Executable))
}

fn parse(input: &str) -> Result<ParsedInput<'_>, String> {
    let mut parts = input.split_whitespace();
    let cmd_string = parts.next().unwrap();

    let command = parse_command(cmd_string)
        .ok_or_else(|| format!("{cmd_string}: command not found").to_string())?;

    Ok(ParsedInput {
        command,
        arguments: parts.collect(),
    })
}

fn handle_echo(arguments: &[&str]) -> Result<(), String> {
    println!("{}", arguments.join(" "));
    Ok(())
}

fn handle_type(arguments: &[&str]) -> Result<(), String> {
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

    Ok(())
}

fn handle_executable(executable_path: &Path, args: &[&str]) {
    process::Command::new(executable_path.file_name().unwrap())
        .args(args)
        .status()
        .expect("Couldn't execute command");
}

fn handle_cd(args: &[&str]) -> Result<(), String> {
    match args {
        [] | ["~"] => std::env::home_dir()
            .ok_or_else(|| "Cannot find your home directory".to_string())
            .and_then(|home| std::env::set_current_dir(home).map_err(|e| e.to_string())),
        [dir] => std::env::set_current_dir(dir).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!("cd: {dir}: No such file or directory"),
            std::io::ErrorKind::PermissionDenied => format!("cd: {dir}: Permission denied"),
            _ => format!("cd: {dir}: {}", e.to_string()),
        }),
        [_, _, ..] => Err("cd: too many arguments".to_string()),
    }
}

fn handle_pwd(args: &[&str]) -> Result<(), String> {
    if !args.is_empty() {
        return Err("pwd: too many arguments".to_string());
    }
    println!(
        "{}",
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .display()
    );
    Ok(())
}

fn run(input: ParsedInput) -> Result<(), String> {
    match input.command {
        Command::Builtin(b) => (b.handler)(&input.arguments),
        Command::Executable(cmd) => {
            handle_executable(&cmd, &input.arguments);
            return Ok(());
        }
    }
}

fn get_dir_children(path: &Path) -> std::io::Result<Vec<String>> {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let files = std::fs::read_dir(absolute_path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter_map(|path| {
            if path.is_file() {
                path.file_name()
                    .and_then(|os_str| os_str.to_str())
                    .map(|s| s.to_string().to_lowercase() + " ")
            } else {
                path.file_name()
                    .and_then(|os_str| os_str.to_str())
                    .map(|s| s.to_string().to_lowercase() + "/")
            }
        })
        .collect();

    Ok(files)
}

fn read_loop() -> std::io::Result<()> {
    let mut line_buffer = String::new();
    let mut tab_count = 0;

    while let Ok(event) = crossterm::event::read() {
        let Some(event) = event.as_key_press_event() else {
            continue;
        };

        // 1. Check for Exit Commands (Ctrl+C / Ctrl+D)
        if event
            .modifiers
            .contains(crossterm::event::KeyModifiers::CONTROL)
        {
            if event.code == crossterm::event::KeyCode::Char('c') {
                print!("\r\n$ ");
                line_buffer.clear();
                io::stdout().flush()?;
            } else if event.code == crossterm::event::KeyCode::Char('d') {
                break;
            }
        }

        // 2. The Bulletproof Enter Check
        // Different environments send Enter differently in raw mode (\n, \r, or Ctrl+J/M)
        let is_enter = match event.code {
            crossterm::event::KeyCode::Enter => true,
            crossterm::event::KeyCode::Char('\n') | crossterm::event::KeyCode::Char('\r') => true,
            crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Char('m')
                if event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                true
            }
            _ => false,
        };

        if is_enter {
            crossterm::terminal::disable_raw_mode()?;
            println!(); // Move cursor to the next line

            if !line_buffer.trim().is_empty() {
                if let Err(e) = parse(&line_buffer).and_then(run) {
                    eprintln!("{e}");
                }
            }

            crossterm::terminal::enable_raw_mode()?;
            print!("\r$ ");
            io::stdout().flush()?;
            line_buffer.clear();
            continue;
        }

        match event.code {
            crossterm::event::KeyCode::Tab => {
                let input = if line_buffer.ends_with(" ") {
                    ""
                } else {
                    line_buffer.split_whitespace().last().unwrap_or("")
                };
                let input_path = Path::new(input);
                let parent_dir = if input.ends_with("/") {
                    input_path
                } else {
                    input_path.parent().unwrap_or(Path::new(""))
                };
                let prefix = input_path
                    .strip_prefix(parent_dir)
                    .unwrap_or(Path::new(""))
                    .to_str()
                    .unwrap_or_default()
                    .to_lowercase();

                let mut matches = vec![];
                if let Ok(mut file_names) = get_dir_children(parent_dir) {
                    file_names.sort();
                    file_names
                        .iter()
                        .filter(|s| s.starts_with(&prefix))
                        .for_each(|s| matches.push(s.clone()));
                }
                match matches.as_slice() {
                    [] => {
                        print!("\x07");
                        io::stdout().flush()?;
                    }
                    [m] => {
                        let remainder = m.strip_prefix(&prefix).unwrap_or("");
                        line_buffer += remainder;
                        print!("{}", remainder);
                        io::stdout().flush()?;
                    }
                    _ => {
                        if tab_count == 0 {
                            let lcp = longest_common_prefix(
                                &matches
                                    .iter()
                                    .map(|s| s.trim_end_matches('/').trim())
                                    .collect::<Vec<&str>>(),
                            );
                            if lcp.len() > prefix.len() {
                                let remainder = lcp.strip_prefix(&prefix).unwrap_or(" ");
                                line_buffer += remainder;
                                print!("{}", remainder);
                            } else {
                                tab_count += 1;
                                print!("\x07");
                            }
                            io::stdout().flush()?;
                        } else {
                            print!("\n\r");
                            matches.iter().for_each(|s| print!("{}  ", s));
                            print!("\n\r$ {}", line_buffer);
                            io::stdout().flush()?;
                            tab_count = 0;
                        }
                    }
                }
            }
            crossterm::event::KeyCode::Backspace => {
                if !line_buffer.is_empty() {
                    line_buffer.pop();
                    print!("\x08 \x08");
                    io::stdout().flush()?;
                }
            }
            crossterm::event::KeyCode::Char(c) => {
                if !event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    line_buffer.push(c);
                    print!("{c}");
                    io::stdout().flush()?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    print!("$ ");
    io::stdout().flush()?;
    read_loop()?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
