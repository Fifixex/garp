use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = parse_config(&args);
    println!("Host: {}\nPort: {}", config.host, config.port);
}

// I know this is improvable but I'm following the rust book I/O project
// plz wait without complaining until I finish the book
struct Config {
    host: String,
    port: String
}

fn parse_config(args: &[String]) -> Config {
    let host = args[1].clone();
    let port = args[2].clone();

    Config {
        host,
        port
    }
}
