// State from https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#first-some-housekeeping-state
use wgpu::{self, Texture, Extent3d, CommandEncoderDescriptor, TextureUsages, TextureFormat};
struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    command_encoder: wgpu::CommandEncoder,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions{
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor{
                // We don't care about most of these since we're not supporting wasm
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None
            }, None).await.unwrap();
        
        let command_encoder = device.create_command_encoder(&CommandEncoderDescriptor{
            label: Some("Command Encoder")
        });

        State{device, queue, command_encoder} 
    }

    fn create_2d_texture(&self, width: u32, height: u32) -> Texture{
        return self.device.create_texture(
            &wgpu::TextureDescriptor { label: None, size: Extent3d { width: width, height: height, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: TextureFormat::Rgba8Uint, usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST }
        )
    }
}