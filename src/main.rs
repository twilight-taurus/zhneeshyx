use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use std::time::Instant;
use wgpu::util::DeviceExt;

use crate::vertex::*;
use anyhow::*;

pub mod texture;
pub mod camera;
pub mod light;
pub mod model;
pub mod vertex;


struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,

    camera: camera::Camera,
    camera_config: camera::UniformBuffer,
    camera_buffer: wgpu::Buffer,
    camera_bindgroup: wgpu::BindGroup, 

    camera_controller: camera::CameraController,

    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    time: Instant,
    clear_color: wgpu::Color,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    num_indices: u32,

    bind_group_index: usize,

    texture_bind_group_layout: wgpu::BindGroupLayout,

    diffuse_bind_groups: Vec<wgpu::BindGroup>,

    obj_model: model::Model,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: &Window) -> Self {

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .filter(|adapter| {
                // Check if this adapter supports our surface
                surface.get_preferred_format(&adapter).is_some()
            })
            .next()
            .unwrap();

        // reguest the graphics card and message queue
        // Question: can we make request_device() async?
        let fut_device = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        );

        let size = window.inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        // block thread until completion.
        let (device, queue) = pollster::block_on( fut_device ).unwrap();

        surface.configure(&device, &config);

        let bytes_road = include_bytes!("road01.png");
        let bytes_gras = include_bytes!("dirt01.png");

        // include_bytes loads a file.
        let my_tex =
            texture::Texture::from_bytes( bytes_road, &device, &queue, "road texture").unwrap();

        let my_tex2 =
            texture::Texture::from_bytes( bytes_gras, &device, &queue, "gras texture").unwrap();

        // bing group describes set of ressources, and they can be accessed
        // by a shader
        // -> the layout:
        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            // This is only for TextureSampleType::Depth
                            comparison: false,
                            // This should be true if the sample_type of the texture is:
                            //     TextureSampleType::Float { filterable: true }
                            // Otherwise you'll get an error.
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        // -> the bind group
        // : more specific declaration of a bind group layout.
        // -> seperation allows switching between bind groups out efficiently
        // : as along as they share the same bind group layout.
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&my_tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&my_tex.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );
        
        let diffuse_bind_group2 = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&my_tex2.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&my_tex2.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        let mut groups = std::vec::Vec::<wgpu::BindGroup>::new();

        groups.push(diffuse_bind_group);
        groups.push(diffuse_bind_group2);

        // camera
        let camera = camera::Camera::new(&config);

        let mut camera_config = camera::UniformBuffer::new();
        camera_config.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_config]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
/*
        let light_config = light::UniformBuffer::new();

        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_config]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });
 
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });
*/
        // create shader
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Basic Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("basic_shader.wgsl").into()),
        });
        
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout, 
                    &camera_bind_group_layout
                ],
                push_constant_ranges: &[],
            });

        // TODO: ...

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            // vertex shader stage
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main", // 1.
                buffers: &[vertex::MVertex::desc()], // 2.
            },
            // fragment shader stage
            fragment: Some(wgpu::FragmentState { // 3.
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState { // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            // rasterizer stage
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.

                cull_mode: Some(wgpu::Face::Back),
//                cull_mode: None,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                clamp_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
        });
/*
        const VERTICES: &[Vertex] = &[
            Vertex { position: [-0.5, 0.5, 1.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-1.0, -0.5, 1.0], color: [0.0, 0.0, 1.0] },
            Vertex { position: [0.0, -0.5, 1.0], color: [0.0, 1.0, 0.0] },
        ];
*/

        const VERTICES_TEX: &[SVertex] = &[
            SVertex { position: [-0.0868241, 0.49240386, 0.0], uv: [0.4131759, 0.99240386], }, // A
            SVertex { position: [-0.49513406, 0.06958647, 0.0], uv: [0.0048659444, 0.56958646], }, // B
            SVertex { position: [-0.21918549, -0.44939706, 0.0], uv: [0.28081453, 0.050602943], }, // C
            SVertex { position: [0.35966998, -0.3473291, 0.0], uv: [0.85967, 0.15267089], }, // D
            SVertex { position: [0.44147372, 0.2347359, 0.0], uv: [0.9414737, 0.7347359], }, // E
        ];

        const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4, /* padding */ 0];

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES_TEX),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indices = INDICES.len() as u32;

        // camera controller
        let camera_controller = camera::CameraController::new();

        let res_dir = std::path::Path::new( env!("OUT_DIR") ).join("res");
        let obj_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("terrain01.obj"),
        ).expect("Unable to create Model.");

        Self {
            surface,
            device,
            queue,

            camera,
            camera_config: camera_config,
            camera_buffer,
            camera_bindgroup,

            camera_controller,

            config,
            size,
            time: Instant::now(),
            clear_color: wgpu::Color::BLACK,

            render_pipeline,

            vertex_buffer,
            index_buffer,

            num_indices,

            bind_group_index: 0 as usize,

            texture_bind_group_layout,
            diffuse_bind_groups: groups,

            obj_model
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    // has an event been processed?
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                /*
                self.clear_color = wgpu::Color {
                    r: position.x as f64 / self.size.width as f64,
                    g: position.y as f64 / self.size.height as f64,
                    b: 1.0,
                    a: 1.0,
                };
                 */
                true
            }
            WindowEvent::KeyboardInput { device_id: _, input, ..} => {
                // if space was pressed
                if input.virtual_keycode.unwrap() == VirtualKeyCode::Space {
                    // switch index.
                    self.bind_group_index = (self.bind_group_index + 1) % self.diffuse_bind_groups.len();
                    true
                } else {
                    if input.state == ElementState::Pressed {
                        println!("Pressed!");
                        self.camera_controller.process_keydown(input.virtual_keycode.unwrap() );
                    } else if input.state == ElementState::Released {
                        println!("Released!");
                        self.camera_controller.process_keyup(input.virtual_keycode.unwrap() );
                    }
                    true      
                }
            }
            
            _ => false,
        }
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_config.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice( &[self.camera_config] ));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {    
        // get current texture will wait for surface to provide a new SurfaceTexture
        let output = self.surface.get_current_texture()?;

        let view =
            output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view, // frame.view : what texture to save the colors to.
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // background clear color
                        load: wgpu::LoadOp::Clear( wgpu::Color {
                      //      r: 1.0,
                            r: self.time.elapsed().as_secs_f64().sin().abs(),
                            g: 1.0,
                            b: self.time.elapsed().as_secs_f64().cos().abs(),
                        //    b: 1.0,
                            a: 1.0,
                        }),
                        store: true, // whether to store render results in the view field above.
                    },
                }],
                depth_stencil_attachment: None,
            });

            // set rendering pipeline created in new()
            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_bind_group( 0, self.diffuse_bind_groups.get(self.bind_group_index).unwrap(), &[] );
            render_pass.set_bind_group(1, &self.camera_bindgroup, &[]);

            use model::DrawModel;

            render_pass.draw_mesh(&self.obj_model.meshes[0]);
//            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//            render_pass.draw(0..self.num_vertices, 0..1);
//            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
//            render_pass.draw(0..3, 0..1); // 3 vertices, once instance.

        } // -->
            // need to drop value _render_pass, since begin_render_pass borrows mutably and
            // we need to call encoder.finish()

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once( encoder.finish() ));
        output.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_title("sneesh-x graphics");

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = pollster::block_on( State::new(&window) );

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { ref event, window_id } => {
            if window_id == window.id() {
                if !state.input(event) {
                    match event {
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }    
            }
        }
        Event::RedrawRequested(_) => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => eprintln!("{:?}", e),
            }
        },
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        },
        _ => {}
    });
}
