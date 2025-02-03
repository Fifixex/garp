use std::env;
type Result<T> = std::result::Result<T, &'static str>;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args)?;

    if let Err(e) = run(config) {
        return Err(e);
    }

    Ok(())
}

fn run(config: Config) -> Result<()> {
    #[cfg(feature = "non-local")]
    if !matches!(config.host.as_str(), "localhost" | "127.0.0.1") {
        return Err("Non-local hosts are not allowed");
    }

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
    fn build(args: &[String]) -> Result<Config> {
        if args.len() < 3 {
            return Err("Not enough arguments");
        }

        let host = args[1].clone();
        let port = args[2].clone();

        Ok(Config { host, port })
    }
}
