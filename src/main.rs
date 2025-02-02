use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args)?;
    println!("Host: {}\nPort: {}", config.host, config.port);
    Ok(())
}

// I know this is improvable but I'm following the rust book I/O project
// plz wait without complaining until I finish the book
struct Config {
    host: String,
    port: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }

        let host = args[1].clone();
        let port = args[2].clone();

        Ok(Config { host, port })
    }
}
