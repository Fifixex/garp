use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
    running: Arc<AtomicBool>,
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
                    running: Arc::new(AtomicBool::new(false)),
                });
            }
        }
    }

    #[track_caller]
    pub fn on_frame<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(Frame) -> Result<()> + Send + 'static,
    {
        if self.running.load(Ordering::SeqCst) {
            panic!("Capture is already running.");
        }

        self.running.store(true, Ordering::SeqCst);
        let running = Arc::clone(&self.running);
        let duplication = self.duplication.clone();

        while running.load(Ordering::SeqCst) {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource: Option<IDXGIResource> = None;
            unsafe {
                if let Err(err) = duplication.AcquireNextFrame(200, &mut frame_info, &mut resource)
                {
                    eprintln!("{err}");
                } else {
                    let resource = resource.unwrap();
                    let source_texture = resource.cast::<ID3D11Texture2D>()?;

                    let frame = Frame {
                        width: 1920,
                        height: 1080,
                    };
                    callback(frame).unwrap();

                    duplication.ReleaseFrame().unwrap();
                }
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
        Ok(())
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
    fn recorder_on_frame() {
        let recorder = VideoRecorder::new().unwrap();
        let callback = |frame: Frame| {
            assert_eq!(frame.width, 1920);
            assert_eq!(frame.height, 1080);
            Ok(())
        };

        let recorder_clone = Arc::new(recorder);
        let recorder_running = Arc::clone(&recorder_clone.running);

        thread::spawn(move || {
            recorder_clone.on_frame(callback).unwrap();
        });

        thread::sleep(std::time::Duration::from_millis(1));

        assert_eq!(recorder_running.load(Ordering::SeqCst), true);

        recorder_running.store(false, Ordering::SeqCst);
        thread::sleep(std::time::Duration::from_millis(1));
    }
}
