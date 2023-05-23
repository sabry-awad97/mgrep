use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <search_term> <search_dir>", args[0]);
        return;
    }
}
