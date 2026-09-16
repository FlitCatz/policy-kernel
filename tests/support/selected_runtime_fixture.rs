fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--version") {
        println!("4.0.0");
    } else {
        std::fs::write("executed-marker", "executed").unwrap();
    }
}
