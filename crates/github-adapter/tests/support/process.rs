#![forbid(unsafe_code)]
use std::{
    env, fs,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
fn main() {
    if env::args().nth(1).as_deref() == Some("--child-sleep") {
        // Publish the pid atomically, then outlive every adapter deadline so
        // the test can observe group cleanup instead of racing a timer.
        let executable = env::current_exe().unwrap();
        let pending = executable.with_extension("started-pending");
        fs::write(&pending, std::process::id().to_string()).unwrap();
        fs::rename(pending, executable.with_extension("started")).unwrap();
        thread::sleep(Duration::from_secs(60));
        return;
    }
    let executable = env::current_exe().unwrap();
    match fs::read_to_string(executable.with_extension("mode"))
        .unwrap()
        .as_str()
    {
        "args" => {
            for arg in env::args().skip(1) {
                println!("{arg}");
            }
            print!("{{}}");
        }
        "inherited" => {
            // Deliberately leave the output pipe inherited to exercise the
            // adapter deadline after the direct child has exited.
            let _child = Command::new(executable)
                .arg("--child-sleep")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
        }
        "success-descendant" | "failure-descendant" | "overflow-descendant" | "blocked-input" => {
            let mode = fs::read_to_string(executable.with_extension("mode")).unwrap();
            let _child = Command::new(&executable)
                .arg("--child-sleep")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            while !executable.with_extension("started").exists() {
                thread::sleep(Duration::from_millis(1));
            }
            match mode.as_str() {
                "failure-descendant" => std::process::exit(7),
                "overflow-descendant" => print!("0123456789"),
                // Outlive the adapter deadline; group cleanup kills this sleep.
                "blocked-input" => thread::sleep(Duration::from_secs(60)),
                _ => print!("{{}}"),
            }
        }
        "overflow" => print!("0123456789"),
        "failure" => std::process::exit(7),
        _ => panic!("unknown process fixture mode"),
    }
}
