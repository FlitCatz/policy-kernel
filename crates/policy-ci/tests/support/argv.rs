#![forbid(unsafe_code)]
use std::io::{self, Write};
fn main() {
    let mut output = io::stdout().lock();
    for arg in std::env::args().skip(1) {
        output.write_all(arg.as_bytes()).unwrap();
        output.write_all(&[0]).unwrap();
    }
}
