#![forbid(unsafe_code)]
use std::{env, fs, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    if env::args().nth(1).as_deref() == Some("--child-sleep") {
        thread::sleep(Duration::from_secs(2));
        return;
    }
    let executable = env::current_exe().unwrap();
    match fs::read_to_string(executable.with_extension("mode")).unwrap().as_str() {
        "args" => { for arg in env::args().skip(1) { println!("{arg}"); } print!("{{}}"); }
        "inherited" => {
            // Deliberately leave the output pipe inherited to exercise the
            // adapter deadline after the direct child has exited.
            let _child = Command::new(executable).arg("--child-sleep")
                .stdout(Stdio::inherit()).stderr(Stdio::inherit()).spawn().unwrap();
        }
        "overflow" => print!("0123456789"),
        "failure" => std::process::exit(7),
        _ => panic!("unknown process fixture mode"),
    }
}
