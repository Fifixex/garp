use std::thread::sleep;
use std::time::Duration;
use windows::{
    Win32::Graphics::{Direct3D::*, Direct3D11::*, Direct3D12::*, Dxgi::*},
    core::{HRESULT, Interface, Result},
};

const MAX_RETRIES: u32 = 10;
const DUPLICATE_OUTPUT_WAITTIME: u64 = 100;

fn create_device() -> Result<(ID3D12Device, IDXGIAdapter1)> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        let adapter = factory.EnumAdapters1(0)?;

        let mut device: Option<ID3D12Device> = None;
        D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device)?;
        let device = device.unwrap();

        Ok((device, adapter))
    }
}

fn duplicate_output(
    device: &ID3D12Device,
    adapter: &IDXGIAdapter1,
) -> Result<IDXGIOutputDuplication> {
    unsafe {
        let output: IDXGIOutput = adapter.EnumOutputs(0)?;
        let output1: IDXGIOutput1 = output.cast()?;

        for retry in 1..=MAX_RETRIES {
            match output1.DuplicateOutput(device) {
                Ok(duplication) => {
                    println!("Screen duplication started successfully");
                    return Ok(duplication);
                }
                Err(e) => {
                    println!(
                        "Error duplicating output, attempt {}/{}: {:?}",
                        retry, MAX_RETRIES, e
                    );

                    if retry < MAX_RETRIES {
                        sleep(Duration::from_millis(DUPLICATE_OUTPUT_WAITTIME));
                    }
                }
            }
        }

        Err(HRESULT(-1).into())
    }
}

pub fn dx12() -> Result<()> {
    unsafe {
        let (device, adapter) = create_device()?;
        println!("DirectX devices created successfully");

        let duplication = duplicate_output(&device, &adapter)?;

        let mut frame_info: DXGI_OUTDUPL_FRAME_INFO = std::mem::zeroed();
        let mut desktop_resource: Option<IDXGIResource> = None;

        println!("{:?}", duplication);
        match duplication.AcquireNextFrame(35, &mut frame_info, &mut desktop_resource) {
            Ok(_) => {
                let resource = desktop_resource.unwrap();
                let texture: ID3D11Texture2D = resource.cast()?;

                let mut desc = D3D11_TEXTURE2D_DESC::default();
                texture.GetDesc(&mut desc);

                println!("Texture Description:");
                println!("  Width: {}", desc.Width);
                println!("  Height: {}", desc.Height);
                println!("  Format: {:?}", desc.Format);
                println!("  MipLevels: {}", desc.MipLevels);
                println!("  ArraySize: {}", desc.ArraySize);
                println!("  SampleCount: {}", desc.SampleDesc.Count);
                println!("  SampleQuality: {}", desc.SampleDesc.Quality);
                println!("  Usage: {:?}", desc.Usage);
                println!("  BindFlags: {:?}", desc.BindFlags);

                duplication.ReleaseFrame()?;
            }
            Err(e) => {
                eprintln!("Failed to acquire frame: {}", e);
            }
        }

        Ok(())
    }
}
