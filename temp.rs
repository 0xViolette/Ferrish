use std::path::PathBuf;
use std::process::Command;

fn main() {
    let path = PathBuf::from("/bin/ls");
    let mut ls = Command::new(path);

    ls.status().expect("process failed to execute");

    println!();
}
