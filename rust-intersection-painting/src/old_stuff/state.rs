use image::GenericImageView;

use crate::{write_flat_texture, create_texture};
use winit::window::Window;

pub struct GpuState {
    // Device information
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    // Texture information
    //grid_texture_rgba: &'a image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, TODO: Get better at lifetimes.
    pub dimensions: (u32, u32),
    grid_texture: wgpu::Texture,
    pub grid_texture_view: wgpu::TextureView,
    pub line_texture: wgpu::Texture,
    pub line_texture_view: wgpu::TextureView,

    pub num_segments: u32,

    pub texture_sampler: wgpu::Sampler,

    // Bind Group Layouts
    pub compute_bind_group_layout: wgpu::BindGroupLayout,
    display_bind_group_layout: wgpu::BindGroupLayout,

    // Shaders
    compute_shader_module: wgpu::ShaderModule,
    display_shader_module: wgpu::ShaderModule,

    // Pipelines
    pub compute_pipeline: wgpu::ComputePipeline,
    display_pipeline: wgpu::RenderPipeline
}

async fn construct_gpu_state(num_segments: u32) -> GpuState {
    // Device Info
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

    // Texture Information
    // TODO: Make this configurable
    let grid_bytes = include_bytes!("test_images/Grid.png");
    let grid_image = image::load_from_memory(grid_bytes).unwrap();
    let grid_texture_rgba = grid_image.as_rgba8().unwrap();

    let dimensions = grid_image.dimensions();

    let grid_texture = create_texture("grid_texture", &device, wgpu::TextureFormat::Rgba8Unorm, wgpu::Extent3d{
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1
    });
    let grid_texture_view = grid_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let line_texture = create_texture("line_texture", &device, wgpu::TextureFormat::Rgba8UnormSrgb, wgpu::Extent3d{
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1
    });
    let line_texture_view = line_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Send grid_texture to the gpu
    write_flat_texture(&queue, &grid_texture, grid_texture_rgba, dimensions);

    // Texture Sampler
    let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,       // What if one fragment corrosponds to multiple pixels?
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    // Bind Group Layouts
    let compute_bind_group_layout = device.create_bind_group_layout(
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

    let display_bind_group_layout = device.create_bind_group_layout(
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
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None
                    }, 
                    count: None
                }
            ],
            label: Some("texture_bind_group_layout"),
        }
    );

    let compute_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Intersection_Compute_Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/intersection-compute.wgsl").into()),
    });

    let display_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Intersection_Display_Shader_Module"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/display-shaders.wgsl").into()),
    });

    // Pipelines
    let compute_pipeline_layout =
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&compute_bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Intersection Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader_module,
        entry_point: "intersection_computer"
    });

    let display_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
        label: Some("Display Pipeline Layout"),
        bind_group_layouts: &[&display_bind_group_layout],
        push_constant_ranges: &[]
    });

    let display_pipeline = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor{
            label: Some("Display Pipeline"),
            layout: Some(&display_pipeline_layout),
            vertex: wgpu::VertexState { 
                module: &display_shader_module, 
                entry_point: "vs_main", // TODO: Update entry_point names
                buffers: &[]        // TODO: Fill out this
            },
            fragment: Some(wgpu::FragmentState{
                module: &display_shader_module,
                entry_point: "fs_main", // TODO: Update entry_point names
                targets: &[]
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None
        }
    );

    GpuState {
        instance,
        adapter,
        device,
        queue,

//      grid_texture_rgba,
        dimensions,
        grid_texture,
        grid_texture_view,
        line_texture,
        line_texture_view,

        num_segments,

        texture_sampler,

        compute_bind_group_layout,
        display_bind_group_layout,

        compute_shader_module,
        display_shader_module,

        compute_pipeline,
        display_pipeline
    }
}