use image::GenericImageView;
use smaa::{SmaaMode, SmaaTarget};
use std::path::Path;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn main() {
    // Use the folder the .exe is sitting in as the image source.
    let exe = std::env::current_exe().expect("cannot locate own path");
    let base = exe.parent().expect("cannot find containing folder").to_path_buf();
    let in_dir = base.clone();
    let out_dir = base.join("output");
    std::fs::create_dir_all(&out_dir).unwrap();

    println!("SMAA batch processor");
    println!("Source folder: {}", in_dir.display());
    println!("Output folder: {}\n", out_dir.display());

    // ---- one-time wgpu setup ----
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("no GPU adapter");
    let (device, queue) = pollster::block_on(
        adapter.request_device(&Default::default()),
    ).expect("no device");

    // fullscreen-blit pipeline: draws the input image into SMAA's color buffer
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
    });
    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(FORMAT.into())],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // reuse the SMAA target across images of identical size
    let mut cached: Option<(u32, u32, SmaaTarget)> = None;

use std::io::Write;

    let exts = ["png", "jpg", "jpeg", "webp", "bmp"];

    // Gather the image files first so we know the total up front.
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&in_dir).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect();
    files.sort();

    let total = files.len();
    let mut failures = 0;

    for (i, path) in files.iter().enumerate() {
        // \r returns the cursor to the start of the line so it overwrites in place.
        print!("\rProcessing image {} of {} ...   ", i + 1, total);
        std::io::stdout().flush().ok();

        if let Err(e) = process(&device, &queue, &pipeline, &bind_layout, &sampler,
                                &mut cached, path, &out_dir) {
            // A skip prints on its own line so it isn't erased by the counter.
            eprintln!("\nskip {} ({e})", path.display());
            failures += 1;
        }
    }

    println!("\rProcessed {} image(s). {} skipped.               ", total, failures);
}

fn process(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    bind_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    cached: &mut Option<(u32, u32, SmaaTarget)>,
    path: &Path,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(path)?.to_rgba8();
    let (w, h) = img.dimensions();

    // input texture
    let input = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        input.as_image_copy(),
        &img,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    let input_view = input.create_view(&Default::default());
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: bind_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&input_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    });

    // output texture SMAA resolves into (must be readable)
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let output_view = output.create_view(&Default::default());

    // (re)build SMAA target if the resolution changed
    if !matches!(cached, Some((cw, ch, _)) if *cw == w && *ch == h) {
        let t = SmaaTarget::new(device, queue, w, h, FORMAT, SmaaMode::Smaa1X);
        *cached = Some((w, h, t));
    }
    let smaa = &mut cached.as_mut().unwrap().2;

// --- STEP 1: draw the image into SMAA's buffer, and SUBMIT it to the GPU ---
    let frame = smaa.start_frame(device, queue, &output_view);
    {
        let mut blit_encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = blit_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame,           // render the image INTO SMAA's color buffer
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        } // render pass ends here
        queue.submit([blit_encoder.finish()]);   // <-- drawing is now on the GPU
    }

    // --- STEP 2: NOW run SMAA (reads the drawn image, writes to output_view) ---
    frame.resolve();

    // --- STEP 3: copy the finished output texture back so we can save it ---
    let align = 256u32;
    let unpadded = w * 4;
    let padded = ((unpadded + align - 1) / align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut copy_encoder = device.create_command_encoder(&Default::default());
    copy_encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    queue.submit([copy_encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
     device.poll(wgpu::PollType::wait_indefinitely()).ok();
    let data = slice.get_mapped_range();

    let mut pixels = Vec::with_capacity((unpadded * h) as usize);
    for row in 0..h {
        let start = (row * padded) as usize;
        pixels.extend_from_slice(&data[start..start + unpadded as usize]);
    }

    let name = path.file_stem().unwrap().to_string_lossy();
    let out = out_dir.join(format!("{name}_smaa.webp"));
    image::RgbaImage::from_raw(w, h, pixels).unwrap().save(out)?;
    Ok(())
}

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;

struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VOut {
    var p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var o: VOut;
    let xy = p[i];
    o.pos = vec4(xy, 0.0, 1.0);
    o.uv = vec2((xy.x + 1.0) * 0.5, (1.0 - xy.y) * 0.5);
    return o;
}

@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    return textureSample(t, s, in.uv);
}
"#;