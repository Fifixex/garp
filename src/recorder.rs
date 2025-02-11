use std::sync::{Arc, Mutex, Condvar};
use std::thread;

use windows::{
    Win32::Foundation::HMODULE,
    Win32::Graphics::{Direct3D::D3D_DRIVER_TYPE_HARDWARE, Direct3D11::*, Dxgi::*},
    core::{Interface, Result},
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VideoRecorder {
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,

    frame_ready: Arc<(Mutex<bool>, Condvar)>,
}

impl VideoRecorder {
    #[allow(unused_variables, unused_assignments)]
    pub fn new() -> Result<Self> {
        unsafe {
            let mut d3d_device = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_SINGLETHREADED,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                None,
            )?;
            let d3d_device = d3d_device.unwrap();
            let dxgi_device = d3d_device.cast::<IDXGIDevice>()?;
            let d3d_context = d3d_device.GetImmediateContext()?;

            let adapter = dxgi_device.GetAdapter()?;
            let mut output_index = 0;
            loop {
                let output = adapter.EnumOutputs(output_index)?;
                output_index += 1;
                let output_desc = output.GetDesc()?;

                let output1 = output.cast::<IDXGIOutput1>()?;
                let duplication = output1.DuplicateOutput(&dxgi_device)?;

                return Ok(Self {
                    d3d_device,
                    d3d_context,
                    duplication,

                    frame_ready: Arc::new((Mutex::new(false), Condvar::new())),
                });
            }
        }
    }

    #[track_caller]
    pub fn on_frame<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Frame) -> Result<()> + Send + 'static,
    {
        let frame_ready = Arc::clone(&self.frame_ready);

        thread::spawn(move || {
            loop {
                let frame = Frame {
                    width: 1920,
                    height: 1080,
                };

                callback(frame).unwrap();

                let (lock, cvar) = &*frame_ready;
                let mut done = lock.lock().unwrap();
                *done = true;
                cvar.notify_one();

                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Ok(())
    }

    pub fn has_frame(&self) -> bool {
        let (lock, _) = &*self.frame_ready;
        let done = lock.lock().unwrap();
        *done
    }
}

#[derive(Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

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
