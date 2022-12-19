pub mod state;

use tokio;

use wgpu::util::DeviceExt;
use futures_intrusive;

#[tokio::main]
async fn main() {
    let result = execute_compute_shader_once(10).await.expect("This isn't a real app, just let me unwrap");
    print!("{:?}, len: {}", result, result.len())
}

fn create_texture(name: &str, device: &wgpu::Device, format: wgpu::TextureFormat, size: wgpu::Extent3d) -> wgpu::Texture {
    // We only need these options for now
    return device.create_texture(
        &wgpu::TextureDescriptor {
            // All wgpu textures are 3d, so we just have a 3D texture with depth 1
            size: size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format,

            // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(name),
        }
    );
}

fn write_flat_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, texture_rgba: &[u8], dimensions: (u32, u32), ){
    queue.write_texture(
        // Tells wgpu where to copy the pixel data
        wgpu::ImageCopyTexture {
            texture: texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // The actual pixel data
        texture_rgba,
        // The layout of the texture
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
            rows_per_image: std::num::NonZeroU32::new(dimensions.1),
        },
        wgpu::Extent3d{
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1
        },
    );
}

async fn execute_compute_shader_once_gpustate(gpustate: &state::GpuState) -> Option<Vec<f32>>{
    let line_bytes = include_bytes!("test_images/line.png"); // TODO: Fix
    let line_image = image::load_from_memory(line_bytes).unwrap();
    let line_rgba = line_image.as_rgba8().unwrap();

    
    write_flat_texture(&gpustate.queue, &gpustate.line_texture, line_rgba, gpustate.dimensions);

    // Let's instantiate our result buffer
    let result_buffer_size = (gpustate.num_segments*4) as usize;
    let result_buffer_contents = vec![0 as f32; result_buffer_size];
    let result_buffer_contents_slice = result_buffer_contents.as_slice();
    let result_buffer_byte_size = result_buffer_size as wgpu::BufferAddress * std::mem::size_of::<f32>() as wgpu::BufferAddress;

    let result_staging_buffer = gpustate.device.create_buffer(&wgpu::BufferDescriptor{
        label: None,
        size: result_buffer_byte_size as u64,//TODO
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false
    }
    );

    let result_buffer = gpustate.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Result Buffer"),
        contents: bytemuck::cast_slice(result_buffer_contents_slice), //TODO Fix
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    }
    );

    // Running the shader
    let mut encoder =
    gpustate.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_pipeline(&gpustate.compute_pipeline);
        cpass.set_bind_group(0, &gpustate.texture_bind_group, &[]);
        cpass.insert_debug_marker("Compute Image Intersections");
        cpass.dispatch_workgroups(1920 as u32, 1080 as u32, 1 as u32);  // Number of cells to run, the (x,y,z) size of item being processed
    }       // NOTE: Size = 1920x1080/20. Keep this in mind when choosing size

    // Will copy data from storage buffer on GPU to staging buffer on CPU.
    encoder.copy_buffer_to_buffer(&result_buffer, 0, &result_staging_buffer, 0, result_buffer_byte_size);

    // Submits command encoder for processing
    gpustate.queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = result_staging_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    gpustate.device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Some(Ok(())) = receiver.receive().await {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = bytemuck::cast_slice(&data).to_vec();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        result_staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        // Returns data from buffer
        Some(result)
    } else {
        panic!("failed to run compute on gpu!")
    }

    None
}

async fn execute_compute_shader_once(num_segments: u32) -> Option<Vec<f32>>{
            // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let adapter = instance.request_adapter(     // Adapter ~= gpu handler
            &wgpu::RequestAdapterOptions::default()
        ).await.unwrap();       // Returns none if no fitting adapter was found

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),      // What extra features do we need?
                limits: wgpu::Limits::default(),        // Set minimum limits the chosen device must have
                label: None,
            },
            None, // Trace path
        ).await.unwrap();
        
        // Test texture loading
        // TODO: Make these configurable
        let grid_bytes = include_bytes!("test_images/Grid.png");
        let grid_image = image::load_from_memory(grid_bytes).unwrap();
        let grid_rgba = grid_image.as_rgba8().unwrap();

        let line_bytes = include_bytes!("test_images/line.png"); // TODO: Fix
        let line_image = image::load_from_memory(line_bytes).unwrap();
        let line_rgba = line_image.as_rgba8().unwrap();

        use image::GenericImageView;
        let dimensions = grid_image.dimensions();

        // We don't want the grid_texture to be transformed, so let's pretend it was created in non-s RGBA (it's not gonna be displayed anyway)
        let grid_texture = create_texture("grid_texture", &device, wgpu::TextureFormat::Rgba8Unorm, wgpu::Extent3d{
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1
        });
        let line_texture = create_texture("line_texture", &device, wgpu::TextureFormat::Rgba8UnormSrgb, wgpu::Extent3d{
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1
        });
        write_flat_texture(&queue, &grid_texture, grid_rgba, dimensions);
        write_flat_texture(&queue, &line_texture, line_rgba, dimensions);

        // Let's instantiate our result buffer
        let result_buffer_size = (num_segments*4) as usize;
        let result_buffer_contents = vec![0 as f32; result_buffer_size];
        let result_buffer_contents_slice = result_buffer_contents.as_slice();
        let result_buffer_byte_size = result_buffer_size as wgpu::BufferAddress * std::mem::size_of::<f32>() as wgpu::BufferAddress;

        let result_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: None,
            size: result_buffer_byte_size as u64,//TODO
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false
        }
        );

        let result_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Result Buffer"),
            contents: bytemuck::cast_slice(result_buffer_contents_slice), //TODO Fix
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        }
        );

        // We still need a view into our texture and a way to sample from it though
        // We don't need to configure the texture view much, so let's
        // let wgpu define it.
        let grid_texture_view = grid_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let line_texture_view = line_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,       // What if one fragment corrosponds to multiple pixels?
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Bind group, e.g. a set of shader accessible resources
        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(
                            // SamplerBindingType::Comparison is only for TextureSampleType::Depth
                            // SamplerBindingType::Filtering if the sample_type of the texture is:
                            //     TextureSampleType::Float { filterable: true }
                            // Otherwise you'll get an error.
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry{
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None
                        }, 
                        count: None
                    }
                ],
                label: Some("texture_bind_group_layout"),
            }
        );
        
        let texture_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&grid_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&line_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&texture_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: result_buffer.as_entire_binding()
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );
        

        // Compute pipeline creation
        let intersection_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Intersection_Compute_Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/intersection-compute.wgsl").into()),
        });

        let intersection_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Intersection Compute Pipeline"),
            layout: Some(&intersection_pipeline_layout),
            module: &intersection_shader,
            entry_point: "intersection_computer"
        });

        // A command encoder executes one or many pipelines.
        // It is to WebGPU what a command buffer is to Vulkan.
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &texture_bind_group, &[]);
            cpass.insert_debug_marker("Compute Image Intersections");
            cpass.dispatch_workgroups(1920 as u32, 1080 as u32, 1 as u32);  // Number of cells to run, the (x,y,z) size of item being processed
        }       // NOTE: Size = 1920x1080/20. Keep this in mind when choosing size

            // Sets adds copy operation to command encoder.
    // Will copy data from storage buffer on GPU to staging buffer on CPU.
    encoder.copy_buffer_to_buffer(&result_buffer, 0, &result_staging_buffer, 0, result_buffer_byte_size);

    // Submits command encoder for processing
    queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = result_staging_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Some(Ok(())) = receiver.receive().await {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = bytemuck::cast_slice(&data).to_vec();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        result_staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        // Returns data from buffer
        Some(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
}