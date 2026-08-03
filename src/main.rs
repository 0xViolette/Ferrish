use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
};

use crossterm;
use is_executable;

struct Builtin {
    name: &'static str,
    handler: fn(&[&str]) -> Result<Option<String>, String>,
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

fn handle_exit(_: &[&str]) -> Result<Option<String>, String> {
    cleanup();

    process::exit(0);
}

enum Program {
    Builtin(&'static Builtin),
    External(PathBuf),
}

struct ParsedCommand<'a> {
    program: Program,
    args: Vec<&'a str>,
}

fn run_pipeline(pipeline: Vec<ParsedCommand>) -> Result<(), String> {
    enum PipelineOutput {
        Process(process::ChildStdout),
        Builtin(String),
    }

    let mut previous_out: Option<PipelineOutput> = None;
    let mut children = Vec::new();

    for (i, command) in pipeline.iter().enumerate() {
        let is_last = i == pipeline.len() - 1;

        match &command.program {
            Program::External(path) => {
                let program_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| format!("invalid program path: {}", path.display()))?;

                let mut proc = std::process::Command::new(program_name);
                proc.args(&command.args);

                match previous_out.take() {
                    Some(PipelineOutput::Process(stdout)) => {
                        proc.stdin(stdout);
                    }
                    Some(PipelineOutput::Builtin(s)) => {
                        proc.stdin(process::Stdio::piped());

                        let mut child = proc
                            .spawn()
                            .map_err(|e| format!("{}: {}", path.display(), e))?;

                        let stdin = child
                            .stdin
                            .as_mut()
                            .ok_or_else(|| format!("{}: failed to open stdin", path.display()))?;

                        stdin.write_all(s.as_bytes()).map_err(|e| {
                            format!("{}: stdin write failed: {}", path.display(), e)
                        })?;

                        // Close stdin so the child sees EOF.
                        drop(child.stdin.take());

                        if !is_last {
                            previous_out = child.stdout.take().map(PipelineOutput::Process);
                        }

                        children.push(child);
                        continue; // We've already spawned this child.
                    }
                    None => {}
                }

                if !is_last {
                    proc.stdout(std::process::Stdio::piped());
                }

                let mut child = proc
                    .spawn()
                    .map_err(|e| format!("{}: {}", path.display(), e))?;

                if !is_last {
                    previous_out = child.stdout.take().map(PipelineOutput::Process);
                }

                children.push(child);
            }
            Program::Builtin(b) => {
                let result = (b.handler)(&command.args).map_err(|e| format!("{}: {}", b.name, e));
                match result {
                    Err(e) => eprintln!("{}", e.to_string()),
                    Ok(None) => {}
                    Ok(Some(s)) => {
                        if is_last {
                            println!("{s}");
                        } else {
                            previous_out = Some(PipelineOutput::Builtin(s + "\n"));
                        }
                    }
                }
            }
        }
    }

    for mut child in children {
        let _ = child
            .wait()
            .map_err(|e| format!("child wait failed: {}", e))?;
    }

    Ok(())
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

fn find_in_path(s: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH").expect("PATH not set");

    env::split_paths(&path)
        .map(|dir| dir.join(s))
        .find(|path| is_executable::is_executable(path))
}

fn resolve_program(s: &str) -> Option<Program> {
    BUILTINS
        .iter()
        .find(|b| b.name == s)
        .map(Program::Builtin)
        .or_else(|| find_in_path(s).map(Program::External))
}

fn parse(input: &str) -> Result<ParsedCommand<'_>, String> {
    let mut parts = input.split_whitespace();
    let cmd_string = parts.next().unwrap();

    let command = resolve_program(cmd_string)
        .ok_or_else(|| format!("{cmd_string}: command not found").to_string())?;

    Ok(ParsedCommand {
        program: command,
        args: parts
            .map(|args| args.trim_matches(|c| c == '"' || c == '\''))
            .collect(),
    })
}

fn parse_pipeline(input: &str) -> Result<Vec<ParsedCommand<'_>>, String> {
    let parts = input.split("|");

    let mut cmds = vec![];

    for part in parts {
        let cmd = parse(part.trim())?;
        cmds.push(cmd);
    }
    return Ok(cmds);
}

fn handle_echo(arguments: &[&str]) -> Result<Option<String>, String> {
    Ok(Some(format!("{}", arguments.join(" "))))
}

fn handle_type(arguments: &[&str]) -> Result<Option<String>, String> {
    let mut v: Vec<String> = Vec::new();
    for arg in arguments.iter() {
        if let Some(cmd) = resolve_program(arg) {
            match cmd {
                Program::External(path) => v.push(format!("{arg} is {}", path.display())),
                _ => v.push(format!("{arg} is a shell builtin")),
            }
        } else {
            v.push(format!("{arg}: not found"));
        };
    }

    Ok(Some(v.join("\n")))
}

fn handle_cd(args: &[&str]) -> Result<Option<String>, String> {
    match args {
        [] | ["~"] => {
            if let Err(e) = std::env::home_dir()
                .ok_or_else(|| "Cannot find your home directory".to_string())
                .and_then(|home| std::env::set_current_dir(home).map_err(|e| e.to_string()))
            {
                return Err(e);
            }
            Ok(None)
        }

        [dir] => {
            if let Err(e) = std::env::set_current_dir(dir).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => format!("cd: {dir}: No such file or directory"),
                std::io::ErrorKind::PermissionDenied => format!("cd: {dir}: Permission denied"),
                _ => format!("cd: {dir}: {}", e.to_string()),
            }) {
                return Err(e);
            }
            Ok(None)
        }
        [_, _, ..] => Err("cd: too many arguments".to_string()),
    }
}

fn handle_pwd(args: &[&str]) -> Result<Option<String>, String> {
    if !args.is_empty() {
        return Err("pwd: too many arguments".to_string());
    }
    Ok(Some(
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .display()
            .to_string(),
    ))
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
                if let Err(e) = parse_pipeline(&line_buffer).and_then(run_pipeline) {
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
