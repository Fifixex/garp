#[cfg(windows)]
pub mod recorder;
#[cfg(windows)]
use recorder::VideoRecorder;

pub type Result<T> = std::result::Result<T, &'static str>;

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
    if let Err(e) = VideoRecorder::new() {
        eprintln!("{e}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{sync::{Arc, Mutex, Condvar}, thread};
    use crate::recorder::VideoRecorder;

    #[test]
    fn recorder() {
        let video_recorder = Arc::new(VideoRecorder::new().expect("Failed to create VideoRecorder"));

        let finished = Arc::new((Mutex::new(false), Condvar::new()));
        let recorder_clone = Arc::clone(&video_recorder);
        let finished_clone = Arc::clone(&finished);

        thread::spawn(move || {
            recorder_clone
                .on_frame(move |frame| {
                    println!("frame: {:?}", frame.width);
                    let (lock, cvar) = &*finished_clone;
                    let mut done = lock.lock().unwrap();
                    *done = true;
                    cvar.notify_one();
                    Ok(())
                })
                .expect("Failed to process frame");
        });

        let (lock, cvar) = &*finished;
        let mut done = lock.lock().unwrap();
        while !*done {
            done = cvar.wait(done).unwrap();
        }

        assert!(video_recorder.has_frame(), "Expected a valid frame to be captured.");
    }
}
