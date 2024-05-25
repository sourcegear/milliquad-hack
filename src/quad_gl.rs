//! Legacy module, code should be either removed or moved to different modules

use miniquad::*;

pub use miniquad::{TextureId as MiniquadTexture};

use crate::{
    Color, 
    texture::Texture2D, 
    };

use std::{collections::BTreeMap, };

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrawMode {
    Triangles,
    Lines,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlPipeline(usize);

struct DrawCall {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,

    vertices_count: usize,
    indices_count: usize,

    clip: Option<(i32, i32, i32, i32)>,
    viewport: Option<(i32, i32, i32, i32)>,
    texture: Option<miniquad::TextureId>,

    model: glam::Mat4,

    draw_mode: DrawMode,
    pipeline: GlPipeline,
    uniforms: Option<Vec<u8>>,
    render_pass: Option<RenderPass>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
    pub(crate) pos: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [u8; 4],
}

pub type VertexInterop = ([f32; 3], [f32; 2], [f32; 4]);

impl Into<VertexInterop> for Vertex {
    fn into(self) -> VertexInterop {
        (
            self.pos,
            self.uv,
            [
                self.color[0] as f32 / 255.0,
                self.color[1] as f32 / 255.0,
                self.color[2] as f32 / 255.0,
                self.color[3] as f32 / 255.0,
            ],
        )
    }
}
impl Into<Vertex> for VertexInterop {
    fn into(self) -> Vertex {
        Vertex {
            pos: self.0,
            uv: self.1,
            color: [
                ((self.2)[0] * 255.) as u8,
                ((self.2)[1] * 255.) as u8,
                ((self.2)[2] * 255.) as u8,
                ((self.2)[3] * 255.) as u8,
            ],
        }
    }
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32, color: Color) -> Vertex {
        Vertex {
            pos: [x, y, z],
            uv: [u, v],
            color: [
                (color.r * 255.) as u8,
                (color.g * 255.) as u8,
                (color.b * 255.) as u8,
                (color.a * 255.) as u8,
            ],
        }
    }
}

impl DrawCall {
    fn new(
        texture: Option<miniquad::TextureId>,
        model: glam::Mat4,
        draw_mode: DrawMode,
        pipeline: GlPipeline,
        uniforms: Option<Vec<u8>>,
        render_pass: Option<RenderPass>,
        max_vertices: usize,
        max_indices: usize,
    ) -> DrawCall {
        DrawCall {
            vertices: vec![
                Vertex::new(0., 0., 0., 0., 0., Color::new(0.0, 0.0, 0.0, 0.0));
                max_vertices
            ],
            indices: vec![0; max_indices],
            vertices_count: 0,
            indices_count: 0,
            viewport: None,
            clip: None,
            texture,
            model,
            draw_mode,
            pipeline,
            uniforms,
            render_pass,
        }
    }

    fn vertices(&self) -> &[Vertex] {
        &self.vertices[0..self.vertices_count]
    }

    fn indices(&self) -> &[u16] {
        &self.indices[0..self.indices_count]
    }
}

struct GlState {
    texture: Option<miniquad::TextureId>,
    draw_mode: DrawMode,
    clip: Option<(i32, i32, i32, i32)>,
    viewport: Option<(i32, i32, i32, i32)>,
    model_stack: Vec<glam::Mat4>,
    pipeline: Option<GlPipeline>,

    break_batching: bool,

    render_pass: Option<RenderPass>,
}

impl GlState {
    fn model(&self) -> glam::Mat4 {
        *self.model_stack.last().unwrap()
    }
}

#[derive(Clone, Debug)]
struct Uniform {
    name: String,
    uniform_type: UniformType,
    byte_offset: usize,
}

#[derive(Clone)]
struct PipelineExt {
    pipeline: miniquad::Pipeline,
    uniforms: Vec<Uniform>,
    uniforms_data: Vec<u8>,
    textures: Vec<String>,
    textures_data: BTreeMap<String, MiniquadTexture>,
}

impl PipelineExt {
    fn set_uniform<T>(&mut self, name: &str, uniform: T) {
        let uniform_meta = self.uniforms.iter().find(
            |Uniform {
                 name: uniform_name, ..
             }| uniform_name == name,
        );
        if uniform_meta.is_none() {
            //warn!("Trying to set non-existing uniform: {}", name);
            return;
        }
        let uniform_meta = uniform_meta.unwrap();
        let uniform_format = uniform_meta.uniform_type;
        let uniform_byte_size = uniform_format.size();
        let uniform_byte_offset = uniform_meta.byte_offset;

        if std::mem::size_of::<T>() != uniform_byte_size {
            /*
            warn!(
                "Trying to set uniform {} sized {} bytes value of {} bytes",
                name,
                uniform_byte_size,
                std::mem::size_of::<T>()
            );
            */
            return;
        }
        macro_rules! transmute_uniform {
            ($uniform_size:expr, $byte_offset:expr, $n:expr) => {
                if $uniform_size == $n {
                    let data: [u8; $n] = unsafe { std::mem::transmute_copy(&uniform) };

                    for i in 0..$uniform_size {
                        self.uniforms_data[$byte_offset + i] = data[i];
                    }
                }
            };
        }
        transmute_uniform!(uniform_byte_size, uniform_byte_offset, 4);
        transmute_uniform!(uniform_byte_size, uniform_byte_offset, 8);
        transmute_uniform!(uniform_byte_size, uniform_byte_offset, 12);
        transmute_uniform!(uniform_byte_size, uniform_byte_offset, 16);
        transmute_uniform!(uniform_byte_size, uniform_byte_offset, 64);
    }
}

struct PipelinesStorage {
    pipelines: [Option<PipelineExt>; Self::MAX_PIPELINES],
    pipelines_amount: usize,
}

impl PipelinesStorage {
    const MAX_PIPELINES: usize = 32;
    const TRIANGLES_PIPELINE: GlPipeline = GlPipeline(0);
    const LINES_PIPELINE: GlPipeline = GlPipeline(1);

    fn new(ctx: &mut dyn RenderingBackend) -> PipelinesStorage {
        let shader = ctx
            .new_shader(
                match ctx.info().backend {
                    Backend::OpenGl => ShaderSource::Glsl {
                        vertex: shader::VERTEX,
                        fragment: shader::FRAGMENT,
                    },
                    Backend::Metal => ShaderSource::Msl {
                        program: shader::METAL,
                    },
                },
                shader::meta(),
            )
            .unwrap_or_else(|e| panic!("Failed to load shader: {}", e));

        let params = PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..Default::default()
        };

        let mut storage = PipelinesStorage {
            pipelines: Default::default(),
            pipelines_amount: 0,
        };

        let triangles_pipeline = storage.make_pipeline(
            ctx,
            shader,
            PipelineParams {
                primitive_type: PrimitiveType::Triangles,
                ..params
            },
            vec![],
            vec![],
        );
        assert_eq!(triangles_pipeline, Self::TRIANGLES_PIPELINE);

        let lines_pipeline = storage.make_pipeline(
            ctx,
            shader,
            PipelineParams {
                primitive_type: PrimitiveType::Lines,
                ..params
            },
            vec![],
            vec![],
        );
        assert_eq!(lines_pipeline, Self::LINES_PIPELINE);

        storage
    }

    fn make_pipeline(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        shader: ShaderId,
        params: PipelineParams,
        mut uniforms: Vec<(String, UniformType)>,
        textures: Vec<String>,
    ) -> GlPipeline {
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("texcoord", VertexFormat::Float2),
                VertexAttribute::new("color0", VertexFormat::Byte4),
            ],
            shader,
            params,
        );

        let id = self
            .pipelines
            .iter()
            .position(|p| p.is_none())
            .unwrap_or_else(|| panic!("Pipelines amount exceeded"));

        let mut max_offset = 0;

        for (name, kind) in shader::uniforms().into_iter().rev() {
            uniforms.insert(0, (name.to_owned(), kind));
        }

        let uniforms = uniforms
            .iter()
            .scan(0, |offset, uniform| {
                let uniform_byte_size = uniform.1.size();
                let uniform = Uniform {
                    name: uniform.0.clone(),
                    uniform_type: uniform.1,
                    byte_offset: *offset,
                };
                *offset += uniform_byte_size;
                max_offset = *offset;

                Some(uniform)
            })
            .collect();

        self.pipelines[id] = Some(PipelineExt {
            pipeline,
            uniforms,
            uniforms_data: vec![0; max_offset],
            textures,
            textures_data: BTreeMap::new(),
        });
        self.pipelines_amount += 1;

        GlPipeline(id)
    }

    fn get(&self, draw_mode: DrawMode) -> GlPipeline {
        // depth_enabled is always false
        // draw_mode is always triangles
        match draw_mode {
            DrawMode::Triangles => Self::TRIANGLES_PIPELINE,
            DrawMode::Lines => Self::LINES_PIPELINE,
        }
    }

    fn get_quad_pipeline_mut(&mut self, pip: GlPipeline) -> &mut PipelineExt {
        self.pipelines[pip.0].as_mut().unwrap()
    }

}

pub(crate) struct QuadGl {
    pipelines: PipelinesStorage,

    draw_calls: Vec<DrawCall>,
    draw_calls_bindings: Vec<Bindings>,
    draw_calls_count: usize,
    state: GlState,
    start_time: f64,

    pub(crate) white_texture: miniquad::TextureId,
    max_vertices: usize,
    max_indices: usize,
}

impl QuadGl {
    pub fn new(ctx: &mut dyn miniquad::RenderingBackend) -> QuadGl {
        let white_texture = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);

        QuadGl {
            pipelines: PipelinesStorage::new(ctx),
            state: GlState {
                clip: None,
                viewport: None,
                texture: None,
                model_stack: vec![glam::Mat4::IDENTITY],
                draw_mode: DrawMode::Triangles,
                pipeline: None,
                break_batching: false,
                render_pass: None,
            },
            draw_calls: Vec::with_capacity(200),
            draw_calls_bindings: Vec::with_capacity(200),
            draw_calls_count: 0,
            start_time: miniquad::date::now(),

            white_texture: white_texture,
            max_vertices: 10000,
            max_indices: 5000,
        }
    }

    pub(crate) fn clear(&mut self, ctx: &mut dyn miniquad::RenderingBackend, color: Color) {
        let clear = PassAction::clear_color(color.r, color.g, color.b, color.a);

        if let Some(current_pass) = self.state.render_pass {
            ctx.begin_pass(Some(current_pass), clear);
        } else {
            ctx.begin_default_pass(clear);
        }
        ctx.end_render_pass();

        self.clear_draw_calls();
    }

    /// Reset only draw calls state
    pub fn clear_draw_calls(&mut self) {
        self.draw_calls_count = 0;
    }

    /// Reset internal state to known default
    pub fn reset(&mut self) {
        self.state.clip = None;
        self.state.texture = None;
        self.state.model_stack = vec![glam::Mat4::IDENTITY];

        self.draw_calls_count = 0;
    }

    pub(crate) fn draw(
        &mut self, 
        ctx: &mut dyn miniquad::RenderingBackend, 
        projection: glam::Mat4
        ) 
    {
        let white_texture = self.white_texture;

        for _ in 0..self.draw_calls.len() - self.draw_calls_bindings.len() {
            let vertex_buffer = ctx.new_buffer(
                BufferType::VertexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<Vertex>(self.max_vertices),
            );
            let index_buffer = ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Stream,
                BufferSource::empty::<u16>(self.max_indices),
            );
            let bindings = Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: vec![white_texture, white_texture],
            };

            self.draw_calls_bindings.push(bindings);
        }
        assert_eq!(self.draw_calls_bindings.len(), self.draw_calls.len());

        let (screen_width, screen_height) = miniquad::window::screen_size();
        //let scale = miniquad::window::dpi_scale();
        //let screen_width = screen_width / scale;
        //let screen_height = screen_height / scale;
        let time = (miniquad::date::now() - self.start_time) as f32;
        let time = glam::vec4(time, time.sin(), time.cos(), 0.);

        for (dc, bindings) in self.draw_calls[0..self.draw_calls_count]
            .iter_mut()
            .zip(self.draw_calls_bindings.iter_mut())
        {
            let pipeline = self.pipelines.get_quad_pipeline_mut(dc.pipeline);

            let (width, height) = if let Some(render_pass) = dc.render_pass {
                let render_texture = ctx.render_pass_texture(render_pass);
                let (width, height) = ctx.texture_size(render_texture);
                (width, height)
            } else {
                (screen_width as u32, screen_height as u32)
            };

            if let Some(render_pass) = dc.render_pass {
                ctx.begin_pass(Some(render_pass), PassAction::Nothing);
            } else {
                ctx.begin_default_pass(PassAction::Nothing);
            }

            ctx.buffer_update(
                bindings.vertex_buffers[0],
                BufferSource::slice(dc.vertices()),
            );
            ctx.buffer_update(bindings.index_buffer, BufferSource::slice(dc.indices()));

            bindings.images[0] = dc.texture.unwrap_or(white_texture);
            bindings.images[1] = white_texture;
            bindings
                .images
                .resize(2 + pipeline.textures.len(), white_texture);

            for (pos, name) in pipeline.textures.iter().enumerate() {
                if let Some(texture) = pipeline.textures_data.get(name).copied() {
                    bindings.images[2 + pos] = texture;
                }
            }

            ctx.apply_pipeline(&pipeline.pipeline);
            if let Some((x, y, w, h)) = dc.viewport {
                ctx.apply_viewport(x, y, w, h);
            } else {
                ctx.apply_viewport(0, 0, width as i32, height as i32);
            }
            if let Some(clip) = dc.clip {
                ctx.apply_scissor_rect(clip.0, height as i32 - (clip.1 + clip.3), clip.2, clip.3);
            } else {
                ctx.apply_scissor_rect(0, 0, width as i32, height as i32);
            }
            ctx.apply_bindings(bindings);

            if let Some(ref uniforms) = dc.uniforms {
                for i in 0..uniforms.len() {
                    pipeline.uniforms_data[i] = uniforms[i];
                }
            }
            pipeline.set_uniform("Projection", projection);
            pipeline.set_uniform("Model", dc.model);
            pipeline.set_uniform("_Time", time);
            ctx.apply_uniforms_from_bytes(
                pipeline.uniforms_data.as_ptr(),
                pipeline.uniforms_data.len(),
            );
            ctx.draw(0, dc.indices_count as i32, 1);
            ctx.end_render_pass();

            dc.vertices_count = 0;
            dc.indices_count = 0;
        }

        self.draw_calls_count = 0;
    }

    pub(crate) fn texture_none(
        &mut self, 
        ) 
    {
        self.state.texture = None;
    }

    pub(crate) fn texture(
        &mut self, 
        texture: Option<&Texture2D>
        ) 
    {
        self.state.texture = texture.map(|t| crate::texture::raw_miniquad_id(&t.texture));
    }

    pub(crate) fn draw_mode(&mut self, mode: DrawMode) {
        self.state.draw_mode = mode;
    }

    pub(crate) fn geometry(&mut self, vertices: &[impl Into<VertexInterop> + Copy], indices: &[u16]) {
        if vertices.len() >= self.max_vertices || indices.len() >= self.max_indices {
            //warn!("geometry() exceeded max drawcall size, clamping");
        }

        let vertices = &vertices[0..self.max_vertices.min(vertices.len())];
        let indices = &indices[0..self.max_indices.min(indices.len())];

        let pip = self.state.pipeline.unwrap_or(
            self.pipelines
                .get(self.state.draw_mode),
        );

        let previous_dc_ix = if self.draw_calls_count == 0 {
            None
        } else {
            Some(self.draw_calls_count - 1)
        };
        let previous_dc = previous_dc_ix.and_then(|ix| self.draw_calls.get(ix));

        if previous_dc.map_or(true, |draw_call| {
            draw_call.texture != self.state.texture
                || draw_call.clip != self.state.clip
                || draw_call.viewport != self.state.viewport
                || draw_call.model != self.state.model()
                || draw_call.pipeline != pip
                || draw_call.render_pass != self.state.render_pass
                || draw_call.draw_mode != self.state.draw_mode
                || draw_call.vertices_count >= self.max_vertices - vertices.len()
                || draw_call.indices_count >= self.max_indices - indices.len()
                || self.state.break_batching
        }) {
            let uniforms = self.state.pipeline.map_or(None, |pipeline| {
                Some(
                    self.pipelines
                        .get_quad_pipeline_mut(pipeline)
                        .uniforms_data
                        .clone(),
                )
            });

            if self.draw_calls_count >= self.draw_calls.len() {
                self.draw_calls.push(DrawCall::new(
                    self.state.texture.clone(),
                    self.state.model(),
                    self.state.draw_mode,
                    pip,
                    uniforms.clone(),
                    self.state.render_pass,
                    self.max_vertices,
                    self.max_indices,
                ));
            }
            self.draw_calls[self.draw_calls_count].texture = self.state.texture.clone();
            self.draw_calls[self.draw_calls_count].uniforms = uniforms;
            self.draw_calls[self.draw_calls_count].vertices_count = 0;
            self.draw_calls[self.draw_calls_count].indices_count = 0;
            self.draw_calls[self.draw_calls_count].clip = self.state.clip;
            self.draw_calls[self.draw_calls_count].viewport = self.state.viewport;
            self.draw_calls[self.draw_calls_count].model = self.state.model();
            self.draw_calls[self.draw_calls_count].pipeline = pip;
            self.draw_calls[self.draw_calls_count].render_pass = self.state.render_pass;

            self.draw_calls_count += 1;
            self.state.break_batching = false;
        };
        let dc = &mut self.draw_calls[self.draw_calls_count - 1];

        for i in 0..vertices.len() {
            dc.vertices[dc.vertices_count + i] = vertices[i].into().into();
        }

        for i in 0..indices.len() {
            dc.indices[dc.indices_count + i] = indices[i] + dc.vertices_count as u16;
        }
        dc.vertices_count += vertices.len();
        dc.indices_count += indices.len();
        dc.texture = self.state.texture.clone();
    }

}

mod shader {
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const VERTEX: &str = r#"#version 100
    attribute vec3 position;
    attribute vec2 texcoord;
    attribute vec4 color0;

    varying lowp vec2 uv;
    varying lowp vec4 color;

    uniform mat4 Model;
    uniform mat4 Projection;

    void main() {
        gl_Position = Projection * Model * vec4(position, 1);
        color = color0 / 255.0;
        uv = texcoord;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec4 color;
    varying lowp vec2 uv;

    uniform sampler2D Texture;

    void main() {
        gl_FragColor = color * texture2D(Texture, uv) ;
    }"#;

    pub const METAL: &str = r#"
#include <metal_stdlib>
    using namespace metal;

    struct Uniforms
    {
        float4x4 Model;
        float4x4 Projection;
    };

    struct Vertex
    {
        float3 position    [[attribute(0)]];
        float2 texcoord    [[attribute(1)]];
        float4 color0      [[attribute(2)]];
    };

    struct RasterizerData
    {
        float4 position [[position]];
        float4 color [[user(locn0)]];
        float2 uv [[user(locn1)]];
    };

    vertex RasterizerData vertexShader(Vertex v [[stage_in]], constant Uniforms& uniforms [[buffer(0)]])
    {
        RasterizerData out;

        out.position = uniforms.Model * uniforms.Projection * float4(v.position, 1);
        out.color = v.color0 / 255.0;
        out.uv = v.texcoord;

        return out;
    }

    fragment float4 fragmentShader(RasterizerData in [[stage_in]], texture2d<float> tex [[texture(0)]], sampler texSmplr [[sampler(0)]])
    {
        return in.color * tex.sample(texSmplr, in.uv);
    }
    "#;
    pub fn uniforms() -> Vec<(&'static str, UniformType)> {
        vec![
            ("Projection", UniformType::Mat4),
            ("Model", UniformType::Mat4),
            ("_Time", UniformType::Float4),
        ]
    }

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["Texture".to_string(), "_ScreenTexture".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: uniforms()
                    .into_iter()
                    .map(|(name, kind)| UniformDesc::new(name, kind))
                    .collect(),
            },
        }
    }
}
