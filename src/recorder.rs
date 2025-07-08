use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use image::RgbaImage;
use windows::{
    Win32::Foundation::HMODULE,
    Win32::Graphics::{Direct3D::D3D_DRIVER_TYPE_HARDWARE, Direct3D11::*, Dxgi::*},
    core::{Interface, Result},
};

#[inline]
fn save_frame_to_img(frame: &Frame, data: &[u8], filename: &str) -> Result<()> {
    let mut dst = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(4) {
        dst.push(chunk[2]);
        dst.push(chunk[1]);
        dst.push(chunk[0]);
        dst.push(chunk[3]);
    }

    let img = RgbaImage::from_raw(frame.width, frame.height, dst.to_vec()).ok_or("Failed to create image buffer from raw data").unwrap();
    img.save(filename).unwrap();
    Ok(())
}

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

    unsafe fn get_frame_data(&self, source_texture: &ID3D11Texture2D) -> Result<Vec<u8>> {
      unsafe {
        let mut source_desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut source_desc);
        source_desc.BindFlags = 0;
        source_desc.MiscFlags = 0;
        source_desc.Usage = D3D11_USAGE_STAGING;
        source_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

        let copy_texture = {
              let mut texture = None;
              self.d3d_device.CreateTexture2D(&source_desc, None, Some(&mut texture))?;
              texture.ok_or("CreateTexture2D failed").unwrap()
        };
        self.d3d_context.CopyResource(Some(&copy_texture.cast()?), Some(&source_texture.cast()?));
        let resource: ID3D11Resource = copy_texture.cast()?;

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        self.d3d_context.Map(
            Some(&resource.clone()),
            0,
            D3D11_MAP_READ,
            0,
            Some(&mut mapped),
        )?;

        let data_size = (mapped.RowPitch as usize) * (source_desc.Height as usize);
        let src_ptr = mapped.pData.cast();
        let data = std::slice::from_raw_parts(src_ptr, data_size).to_vec();

        self.d3d_context.Unmap(Some(&resource), 0);

        Ok(data)
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
                    duplication.ReleaseFrame().unwrap();
                } else {
                    if frame_info.LastPresentTime != 0 {
                        let resource = resource.unwrap();
                        let source_texture = resource.cast::<ID3D11Texture2D>()?;
                        let buffer = self.get_frame_data(&source_texture)?;
                        let frame = Frame { width: 1920, height: 1080 };
                        let filename = format!("frame-{}.png", chrono::Utc::now().timestamp_millis());
                        let _ = save_frame_to_img(&frame, &buffer, &filename);
                        callback(frame).unwrap();
                    }
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
