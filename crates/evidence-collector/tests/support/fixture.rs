use std::{
    env, fs,
    io::{self, Write},
    process::Command,
    thread,
    time::Duration,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("") {
        "--version" => println!("fixture 1"),
        "argv0" => println!("{}", args[0]),
        "ok" => {
            println!("ok");
            eprintln!("note");
        }
        "fail" => std::process::exit(7),
        "sleep" => thread::sleep(Duration::from_secs(5)),
        "bytes" => {
            let n: usize = args[2].parse().unwrap();
            let bytes = vec![b'x'; n];
            io::stdout().write_all(&bytes).unwrap();
            io::stderr().write_all(&bytes).unwrap();
        }
        "binary" => io::stdout().write_all(&[0, 255, 128, 10]).unwrap(),
        "args" => {
            for arg in &args[2..] {
                println!("{arg}");
            }
        }
        "env" => {
            println!("{}", env::var("COLLECTOR_VISIBLE").unwrap_or_default());
            println!(
                "{}",
                env::var("COLLECTOR_SECRET").unwrap_or_else(|_| "absent".into())
            );
            println!("{}", env::current_dir().unwrap().display());
        }
        "artifact" => fs::write("result.txt", b"abc").unwrap(),
        "marker" => {
            thread::sleep(Duration::from_millis(500));
            fs::write(&args[2], "leaked").unwrap();
        }
        "descendant" | "orphan" | "orphan-closed" | "flood-descendant" => {
            let mut child = Command::new(env::current_exe().unwrap());
            child.args(["marker", &args[2]]);
            if args[1] == "orphan-closed" {
                child
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
            }
            let _child = child.spawn().unwrap();
            if args[1] == "flood-descendant" {
                io::stdout().write_all(&vec![b'x'; 4096]).unwrap();
            }
            if args[1] == "descendant" {
                thread::sleep(Duration::from_secs(5));
            }
        }
        "stderr" => io::stderr().write_all(&vec![b'e'; 4096]).unwrap(),
        "delay" => thread::sleep(Duration::from_millis(args[2].parse().unwrap())),
        "signal" => {
            let _ = Command::new("/bin/kill")
                .args(["-TERM", &std::process::id().to_string()])
                .status();
        }
        _ => std::process::exit(9),
    }
}
