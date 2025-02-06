#[cfg(windows)]
pub mod dx12;
#[cfg(windows)]
use dx12::dx12;

type Result<T> = std::result::Result<T, &'static str>;

pub struct Config {
    pub host: String,
    pub port: String,
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config> {
        let host = args.next().ok_or("Host not provided")?;
        let port = args.next().ok_or("Port not provided")?;

        Ok(Config { host, port })
    }
}

pub fn run(config: Config) -> Result<()> {
    #[cfg(feature = "non-local")]
    if !matches!(config.host.as_str(), "localhost" | "127.0.0.1") {
        return Err("Non-local hosts are not allowed");
    }

    if config.port.parse::<u16>().is_err() || config.port.len() < 4 {
        return Err("Port is not a number or is too short");
    }

    println!("Host: {}\nPort: {}", config.host, config.port);
    #[cfg(windows)]
    if let Err(e) = dx12() {
        eprintln!("{e}");
    }

    Ok(())
}
