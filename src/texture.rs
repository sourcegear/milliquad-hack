//! Loading and rendering textures. Also render textures, per-pixel image manipulations.

use crate::{
    Color,
    math::Rect,
    text::atlas::SpriteKey, 
    //Error,
};

use crate::quad_gl::{DrawMode, Vertex};
use crate::quad_gl::QuadGl;
use glam::{vec2, Vec2};

pub use crate::quad_gl::FilterMode;

use slotmap::SlotMap;
use std::sync::Arc;

slotmap::new_key_type! {
    pub(crate) struct TextureSlotId;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextureSlotGuarded(pub TextureSlotId);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TextureHandle {
    // texture that belongs to macroquad and follows normal garbage collection rules
    Managed(Arc<TextureSlotGuarded>),
    ManagedWeak(TextureSlotId),
    // raw miniquad texture, there are no guarantees that this texture is not yet deleted
    Unmanaged(miniquad::TextureId),
}

pub(crate) struct TexturesContext {
    textures: SlotMap<crate::texture::TextureSlotId, miniquad::TextureId>,
    removed: Vec<TextureSlotId>,
}
impl TexturesContext {
    pub fn new() -> TexturesContext {
        TexturesContext {
            textures: SlotMap::with_key(),
            removed: Vec::with_capacity(200),
        }
    }
    fn schedule_removed(&mut self, texture: TextureSlotId) {
        self.removed.push(texture);
    }
    fn store_texture(&mut self, texture: miniquad::TextureId) -> TextureHandle {
        TextureHandle::Managed(Arc::new(TextureSlotGuarded(self.textures.insert(texture))))
    }
    pub fn texture(&self, texture: TextureSlotId) -> Option<miniquad::TextureId> {
        self.textures.get(texture).copied()
    }
    fn remove(&mut self, texture: TextureSlotId) {
        self.textures.remove(texture);
    }
    pub fn len(&self) -> usize {
        self.textures.len()
    }
    pub fn garbage_collect(&mut self, ctx: &mut miniquad::Context) {
        for texture in self.removed.drain(0..) {
            if let Some(texture) = self.textures.get(texture).copied() {
                ctx.delete_texture(texture);
            }
            self.textures.remove(texture);
        }
    }
}

/// Image, data stored in CPU memory
#[derive(Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes.len()", &self.bytes.len())
            .finish()
    }
}

impl Image {
    /// Creates an empty Image.
    ///
    /// ```
    /// # use macroquad::prelude::*;
    /// let image = Image::empty();
    /// ```
    pub fn empty() -> Image {
        Image {
            width: 0,
            height: 0,
            bytes: vec![],
        }
    }

    /// Creates an Image from a slice of bytes that contains an encoded image.
    ///
    /// If `format` is None, it will make an educated guess on the
    /// [ImageFormat][image::ImageFormat].
    ///
    /// # Example
    ///
    /// ```
    /// # use macroquad::prelude::*;
    /// let icon = Image::from_file_with_format(
    ///     include_bytes!("../examples/rust.png"),
    ///     Some(ImageFormat::Png),
    ///     );
    /// ```
    pub fn from_file_with_format(
        bytes: &[u8],
        format: Option<image::ImageFormat>,
    ) -> Result<Image, i32> 
    {
        let img = if let Some(fmt) = format {
            image::load_from_memory_with_format(bytes, fmt).map_err(|x| -1)?.to_rgba8()
        } else {
            image::load_from_memory(bytes).map_err(|x| -1)?.to_rgba8()
        };
        let width = img.width() as u16;
        let height = img.height() as u16;
        let bytes = img.into_raw();

        Ok(Image {
            width,
            height,
            bytes,
        })
    }

    /// Creates an Image filled with the provided [Color].
    pub fn gen_image_color(width: u16, height: u16, color: Color) -> Image {
        let mut bytes = vec![0; width as usize * height as usize * 4];
        for i in 0..width as usize * height as usize {
            bytes[i * 4 + 0] = (color.r * 255.) as u8;
            bytes[i * 4 + 1] = (color.g * 255.) as u8;
            bytes[i * 4 + 2] = (color.b * 255.) as u8;
            bytes[i * 4 + 3] = (color.a * 255.) as u8;
        }
        Image {
            width,
            height,
            bytes,
        }
    }

    /// Updates this image from a slice of [Color]s.
    pub fn update(&mut self, colors: &[Color]) {
        assert!(self.width as usize * self.height as usize == colors.len());

        for i in 0..colors.len() {
            self.bytes[i * 4] = (colors[i].r * 255.) as u8;
            self.bytes[i * 4 + 1] = (colors[i].g * 255.) as u8;
            self.bytes[i * 4 + 2] = (colors[i].b * 255.) as u8;
            self.bytes[i * 4 + 3] = (colors[i].a * 255.) as u8;
        }
    }

    /// Returns the width of this image.
    pub fn width(&self) -> usize {
        self.width as usize
    }

    /// Returns the height of this image.
    pub fn height(&self) -> usize {
        self.height as usize
    }

    /// Returns this image's data as a slice of 4-byte arrays.
    pub fn get_image_data(&self) -> &[[u8; 4]] {
        use std::slice;

        unsafe {
            slice::from_raw_parts(
                self.bytes.as_ptr() as *const [u8; 4],
                self.width as usize * self.height as usize,
            )
        }
    }

    /// Returns this image's data as a mutable slice of 4-byte arrays.
    pub fn get_image_data_mut(&mut self) -> &mut [[u8; 4]] {
        use std::slice;

        unsafe {
            slice::from_raw_parts_mut(
                self.bytes.as_mut_ptr() as *mut [u8; 4],
                self.width as usize * self.height as usize,
            )
        }
    }

    /// Modifies a pixel [Color] in this image.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        let width = self.width;

        self.get_image_data_mut()[(y * width as u32 + x) as usize] = color.into();
    }

    /// Returns a pixel [Color] from this image.
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        self.get_image_data()[(y * self.width as u32 + x) as usize].into()
    }

    /// Returns an Image from a rect inside this image.
    pub fn sub_image(&self, rect: Rect) -> Image {
        let width = rect.w as usize;
        let height = rect.h as usize;
        let mut bytes = vec![0; width * height * 4];

        let x = rect.x as usize;
        let y = rect.y as usize;
        let mut n = 0;
        for y in y..y + height {
            for x in x..x + width {
                bytes[n] = self.bytes[y * self.width as usize * 4 + x * 4 + 0];
                bytes[n + 1] = self.bytes[y * self.width as usize * 4 + x * 4 + 1];
                bytes[n + 2] = self.bytes[y * self.width as usize * 4 + x * 4 + 2];
                bytes[n + 3] = self.bytes[y * self.width as usize * 4 + x * 4 + 3];
                n += 4;
            }
        }
        Image {
            width: width as u16,
            height: height as u16,
            bytes,
        }
    }

    /// Blends this image with another image (of identical dimensions)
    /// Inspired by  OpenCV saturated blending
    pub fn blend(&mut self, other: &Image) {
        assert!(
            self.width as usize * self.height as usize
                == other.width as usize * other.height as usize
        );

        for i in 0..self.bytes.len() / 4 {
            let c1: Color = Color {
                r: self.bytes[i * 4] as f32 / 255.,
                g: self.bytes[i * 4 + 1] as f32 / 255.,
                b: self.bytes[i * 4 + 2] as f32 / 255.,
                a: self.bytes[i * 4 + 3] as f32 / 255.,
            };
            let c2: Color = Color {
                r: other.bytes[i * 4] as f32 / 255.,
                g: other.bytes[i * 4 + 1] as f32 / 255.,
                b: other.bytes[i * 4 + 2] as f32 / 255.,
                a: other.bytes[i * 4 + 3] as f32 / 255.,
            };
            let new_color: Color = Color {
                r: f32::min(c1.r * c1.a + c2.r * c2.a, 1.),
                g: f32::min(c1.g * c1.a + c2.g * c2.a, 1.),
                b: f32::min(c1.b * c1.a + c2.b * c2.a, 1.),
                a: f32::max(c1.a, c2.a) + (1. - f32::max(c1.a, c2.a)) * f32::min(c1.a, c2.a),
            };
            self.bytes[i * 4] = (new_color.r * 255.) as u8;
            self.bytes[i * 4 + 1] = (new_color.g * 255.) as u8;
            self.bytes[i * 4 + 2] = (new_color.b * 255.) as u8;
            self.bytes[i * 4 + 3] = (new_color.a * 255.) as u8;
        }
    }

    /// Overlays an image on top of this one.
    /// Slightly different from blending two images,
    /// overlaying a completely transparent image has no effect
    /// on the original image, though blending them would.
    pub fn overlay(&mut self, other: &Image) {
        assert!(
            self.width as usize * self.height as usize
                == other.width as usize * other.height as usize
        );

        for i in 0..self.bytes.len() / 4 {
            let c1: Color = Color {
                r: self.bytes[i * 4] as f32 / 255.,
                g: self.bytes[i * 4 + 1] as f32 / 255.,
                b: self.bytes[i * 4 + 2] as f32 / 255.,
                a: self.bytes[i * 4 + 3] as f32 / 255.,
            };
            let c2: Color = Color {
                r: other.bytes[i * 4] as f32 / 255.,
                g: other.bytes[i * 4 + 1] as f32 / 255.,
                b: other.bytes[i * 4 + 2] as f32 / 255.,
                a: other.bytes[i * 4 + 3] as f32 / 255.,
            };
            let new_color: Color = Color {
                r: f32::min(c1.r * (1. - c2.a) + c2.r * c2.a, 1.),
                g: f32::min(c1.g * (1. - c2.a) + c2.g * c2.a, 1.),
                b: f32::min(c1.b * (1. - c2.a) + c2.b * c2.a, 1.),
                a: f32::min(c1.a + c2.a, 1.),
            };

            self.bytes[i * 4] = (new_color.r * 255.) as u8;
            self.bytes[i * 4 + 1] = (new_color.g * 255.) as u8;
            self.bytes[i * 4 + 2] = (new_color.b * 255.) as u8;
            self.bytes[i * 4 + 3] = (new_color.a * 255.) as u8;
        }
    }

    /// Saves this image as a PNG file.
    /// This method is not supported on web and will panic.
    pub fn export_png(&self, path: &str) {
        let mut bytes = vec![0; self.width as usize * self.height as usize * 4];

        // flip the image before saving
        for y in 0..self.height as usize {
            for x in 0..self.width as usize * 4 {
                bytes[y * self.width as usize * 4 + x] =
                    self.bytes[(self.height as usize - y - 1) * self.width as usize * 4 + x];
            }
        }

        image::save_buffer(
            path,
            &bytes[..],
            self.width as _,
            self.height as _,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct RenderPass {
    pub color_texture: Texture2D,
    pub depth_texture: Option<Texture2D>,
    pub(crate) render_pass: Arc<miniquad::RenderPass>,
}

impl RenderPass {
    fn new(
        quad_context: &mut dyn miniquad::RenderingBackend, 
        textures: &TexturesContext, 
        gl: &QuadGl,
        color_texture: Texture2D, 
        depth_texture: Option<Texture2D>
        ) -> RenderPass {
        let render_pass = quad_context.new_render_pass(
            color_texture.raw_miniquad_id(textures, gl),
            depth_texture.as_ref().map(|t| t.raw_miniquad_id(textures, gl)),
        );
        RenderPass {
            color_texture,
            depth_texture: depth_texture.map(|t| t.clone()),
            render_pass: Arc::new(render_pass),
        }
    }
    /// Returns the miniquad handle for this render pass.
    pub fn raw_miniquad_id(&self) -> miniquad::RenderPass {
        *self.render_pass
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        if Arc::strong_count(&self.render_pass) < 2 {
            //let context = get_quad_context();
            //context.delete_render_pass(*self.render_pass);
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderTarget {
    pub texture: Texture2D,
    pub render_pass: RenderPass,
}

fn render_pass(
    quad_context: &mut dyn miniquad::RenderingBackend,
    textures: &mut TexturesContext, 
    gl: &QuadGl, 
    color_texture: Texture2D, 
    depth_texture: Option<Texture2D>
    ) -> RenderPass 
{
    RenderPass::new(quad_context, textures, gl, color_texture, depth_texture)
}

pub fn render_target(
    quad_context: &mut dyn miniquad::RenderingBackend, 
    textures: &mut TexturesContext, 
    gl: &mut QuadGl, 
    width: u32, 
    height: u32
    ) -> RenderTarget 
{
    let texture_id = quad_context.new_render_texture(miniquad::TextureParams {
        width,
        height,
        ..Default::default()
    });

    let texture = Texture2D {
        texture: textures.store_texture(texture_id),
    };

    let render_pass = render_pass(quad_context, textures, gl, texture.clone(), None);

    RenderTarget {
        texture,
        render_pass,
    }
}

#[derive(Debug, Clone)]
pub struct DrawTextureParams {
    pub dest_size: Option<Vec2>,

    /// Part of texture to draw. If None - draw the whole texture.
    /// Good use example: drawing an image from texture atlas.
    /// Is None by default
    pub source: Option<Rect>,

    /// Rotation in radians
    pub rotation: f32,

    /// Mirror on the X axis
    pub flip_x: bool,

    /// Mirror on the Y axis
    pub flip_y: bool,

    /// Rotate around this point.
    /// When `None`, rotate around the texture's center.
    /// When `Some`, the coordinates are in screen-space.
    /// E.g. pivot (0,0) rotates around the top left corner of the screen, not of the
    /// texture.
    pub pivot: Option<Vec2>,
}

impl Default for DrawTextureParams {
    fn default() -> DrawTextureParams {
        DrawTextureParams {
            dest_size: None,
            source: None,
            rotation: 0.,
            pivot: None,
            flip_x: false,
            flip_y: false,
        }
    }
}

pub fn draw_texture(
    gl: &mut QuadGl, 
    quad_context: &mut dyn miniquad::RenderingBackend, 
    textures: &mut TexturesContext,
    texture_batcher: &mut Batcher, 
    texture: &Texture2D, x: f32, y: f32, color: Color) 
{
    draw_texture_ex(gl, quad_context, textures, texture_batcher, texture, x, y, color, Default::default());
}

pub fn draw_texture_ex(
    gl: &mut QuadGl,
    quad_context: &mut dyn miniquad::RenderingBackend, 
    textures: &TexturesContext,
    texture_batcher: &mut Batcher,
    texture: &Texture2D,
    x: f32,
    y: f32,
    color: Color,
    params: DrawTextureParams,
    ) 
{
    let [mut width, mut height] = texture.size(quad_context, textures, gl).to_array();

    let Rect {
        x: mut sx,
        y: mut sy,
        w: mut sw,
        h: mut sh,
    } = params.source.unwrap_or(Rect {
        x: 0.,
        y: 0.,
        w: width,
        h: height,
    });

    let texture_opt =
        texture_batcher
        .get(quad_context, textures, gl, texture)
        .map(|(batched_texture, uv)| {
            let [batched_width, batched_height] = batched_texture.size(quad_context, textures, gl).to_array();
            sx = ((sx / width) * uv.w + uv.x) * batched_width;
            sy = ((sy / height) * uv.h + uv.y) * batched_height;
            sw = (sw / width) * uv.w * batched_width;
            sh = (sh / height) * uv.h * batched_height;

            width = batched_width;
            height = batched_height;

            batched_texture
        });
    let texture = texture_opt.as_ref().unwrap_or(texture);

    let (mut w, mut h) = match params.dest_size {
        Some(dst) => (dst.x, dst.y),
        _ => (sw, sh),
    };
    let mut x = x;
    let mut y = y;
    if params.flip_x {
        x = x + w;
        w = -w;
    }
    if params.flip_y {
        y = y + h;
        h = -h;
    }

    let pivot = params.pivot.unwrap_or(vec2(x + w / 2., y + h / 2.));
    let m = pivot;
    let p = [
        vec2(x, y) - pivot,
        vec2(x + w, y) - pivot,
        vec2(x + w, y + h) - pivot,
        vec2(x, y + h) - pivot,
    ];
    let r = params.rotation;
    let p = [
        vec2(
            p[0].x * r.cos() - p[0].y * r.sin(),
            p[0].x * r.sin() + p[0].y * r.cos(),
        ) + m,
        vec2(
            p[1].x * r.cos() - p[1].y * r.sin(),
            p[1].x * r.sin() + p[1].y * r.cos(),
        ) + m,
        vec2(
            p[2].x * r.cos() - p[2].y * r.sin(),
            p[2].x * r.sin() + p[2].y * r.cos(),
        ) + m,
        vec2(
            p[3].x * r.cos() - p[3].y * r.sin(),
            p[3].x * r.sin() + p[3].y * r.cos(),
        ) + m,
    ];
    #[rustfmt::skip]
    let vertices = [
        Vertex::new(p[0].x, p[0].y, 0.,  sx      /width,  sy      /height, color),
        Vertex::new(p[1].x, p[1].y, 0., (sx + sw)/width,  sy      /height, color),
        Vertex::new(p[2].x, p[2].y, 0., (sx + sw)/width, (sy + sh)/height, color),
        Vertex::new(p[3].x, p[3].y, 0.,  sx      /width, (sy + sh)/height, color),
    ];
    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

    gl.texture(textures, Some(&texture));
    gl.draw_mode(DrawMode::Triangles);
    gl.geometry(&vertices, &indices);
}

/// Get pixel data from screen buffer and return an Image (screenshot)
pub fn get_screen_data(
    quad_context: &mut dyn miniquad::RenderingBackend, 
    gl: &mut QuadGl,
    textures: &mut TexturesContext,
    mat: glam::Mat4
    ) -> Image 
{
    gl.draw(quad_context, mat);

    let (w, h) = miniquad::window::screen_size();
    let texture_id = quad_context.new_render_texture(miniquad::TextureParams 
    {
        width: w as _,
        height: h as _,
        ..Default::default()
    });

    let texture = Texture2D {
        texture: textures.store_texture(texture_id),
    };

    texture.grab_screen(quad_context, textures, gl);

    texture.get_texture_data(quad_context, textures, gl)
}

/// Texture, data stored in GPU memory
#[derive(Clone, Debug, PartialEq)]
pub struct Texture2D {
    pub(crate) texture: TextureHandle,
}

impl Drop for TextureSlotGuarded {
    fn drop(&mut self) {
        //let ctx = get_context();
        //ctx.textures.schedule_removed(self.0);
    }
}

impl Texture2D {
    pub fn weak_clone(&self) -> Texture2D {
        match &self.texture {
            TextureHandle::Unmanaged(id) => Texture2D::unmanaged(*id),
            TextureHandle::Managed(t) => Texture2D {
                texture: TextureHandle::ManagedWeak((**t).0),
            },
            TextureHandle::ManagedWeak(t) => Texture2D {
                texture: TextureHandle::ManagedWeak(t.clone()),
            },
        }
    }
    pub(crate) fn unmanaged(texture: miniquad::TextureId) -> Texture2D {
        Texture2D {
            texture: TextureHandle::Unmanaged(texture),
        }
    }
    /// Creates an empty Texture2D.
    ///
    /// # Example
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// let texture = Texture2D::empty();
    /// # }
    /// ```
    pub fn empty(gl: &QuadGl) -> Texture2D {
        Texture2D::unmanaged(gl.white_texture)
    }

    /// Creates a Texture2D from a slice of bytes that contains an encoded image.
    ///
    /// If `format` is None, it will make an educated guess on the
    /// [ImageFormat][image::ImageFormat].
    ///
    /// # Example
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// let texture = Texture2D::from_file_with_format(
    ///     include_bytes!("../examples/rust.png"),
    ///     None,
    ///     );
    /// # }
    /// ```
    pub fn from_file_with_format<'a>(
        quad_context: &mut dyn miniquad::RenderingBackend,
        textures: &mut TexturesContext,
        texture_batcher: &mut Batcher,
        bytes: &[u8],
        format: Option<image::ImageFormat>,
    ) -> Texture2D {
        let img = if let Some(fmt) = format {
            image::load_from_memory_with_format(bytes, fmt)
                .unwrap_or_else(|e| panic!("{}", e))
                .to_rgba8()
        } else {
            image::load_from_memory(bytes)
                .unwrap_or_else(|e| panic!("{}", e))
                .to_rgba8()
        };
        let width = img.width() as u16;
        let height = img.height() as u16;
        let bytes = img.into_raw();

        Self::from_rgba8(quad_context, textures, texture_batcher, width, height, &bytes)
    }

    /// Creates a Texture2D from an [Image].
    pub fn from_image(
        quad_context: &mut dyn miniquad::RenderingBackend,
        textures: &mut TexturesContext,
        texture_batcher: &mut Batcher,
        image: &Image) -> Texture2D 
    {
        Texture2D::from_rgba8(quad_context, textures, texture_batcher, image.width, image.height, &image.bytes)
    }

    /// Creates a Texture2D from a miniquad
    /// [Texture](https://docs.rs/miniquad/0.3.0-alpha/miniquad/graphics/struct.Texture.html)
    pub fn from_miniquad_texture(texture: miniquad::TextureId) -> Texture2D {
        Texture2D {
            texture: TextureHandle::Unmanaged(texture),
        }
    }

    /// Creates a Texture2D from a slice of bytes in an R,G,B,A sequence,
    /// with the given width and height.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// // Create a 2x2 texture from a byte slice with 4 rgba pixels
    /// let bytes: Vec<u8> = vec![255, 0, 0, 192, 0, 255, 0, 192, 0, 0, 255, 192, 255, 255, 255, 192];
    /// let texture = Texture2D::from_rgba8(2, 2, &bytes);
    /// # }
    /// ```
    pub fn from_rgba8(
        quad_context: &mut dyn miniquad::RenderingBackend, 
        textures: &mut TexturesContext, 
        texture_batcher: &mut Batcher,
        width: u16, height: u16, bytes: &[u8]) -> Texture2D {
        let texture = quad_context.new_texture_from_rgba8(width, height, bytes);
        let texture = textures.store_texture(texture);
        let texture = Texture2D { texture };

        texture_batcher.add_unbatched(&texture);

        texture
    }

    /// Uploads [Image] data to this texture.
    pub fn update(
        &self, 
        ctx: &mut dyn miniquad::RenderingBackend, 
        textures: &TexturesContext, 
        gl: &QuadGl,
        image: &Image
        ) 
    {
        let (width, height) = ctx.texture_size(self.raw_miniquad_id(textures, gl));

        assert_eq!(width, image.width as u32);
        assert_eq!(height, image.height as u32);

        ctx.texture_update(self.raw_miniquad_id(textures, gl), &image.bytes);
    }

    // Updates the texture from an array of bytes.
    pub fn update_from_bytes(
        &self, 
        ctx: &mut dyn miniquad::RenderingBackend, 
        textures: &TexturesContext, 
        gl: &QuadGl,
        width: u32, height: u32, bytes: &[u8]) 
    {
        let (texture_width, texture_height) = ctx.texture_size(self.raw_miniquad_id(textures, gl));

        assert_eq!(texture_width, width as u32);
        assert_eq!(texture_height, height as u32);

        ctx.texture_update(self.raw_miniquad_id(textures, gl), bytes);
    }

    /// Uploads [Image] data to part of this texture.
    pub fn update_part(
        &self,
        ctx: &mut dyn miniquad::RenderingBackend,
        textures: &TexturesContext, 
        gl: &QuadGl,
        image: &Image,
        x_offset: i32,
        y_offset: i32,
        width: i32,
        height: i32,
        ) 
    {
        ctx.texture_update_part(
            self.raw_miniquad_id(textures, gl),
            x_offset,
            y_offset,
            width,
            height,
            &image.bytes,
        )
    }

    /// Returns the width of this texture.
    pub fn width(
        &self, 
        ctx: &dyn miniquad::RenderingBackend,
        textures: &TexturesContext, 
        gl: &QuadGl,
        ) -> f32 
    {
        let (width, _) = ctx.texture_size(self.raw_miniquad_id(textures, gl));

        width as f32
    }

    /// Returns the height of this texture.
    pub fn height(
        &self, 
        ctx: &dyn miniquad::RenderingBackend,
        textures: &TexturesContext, 
        gl: &QuadGl,
        ) -> f32 
    {
        let (_, height) = ctx.texture_size(self.raw_miniquad_id(textures, gl));

        height as f32
    }

    pub fn size(
        &self, 
        ctx: &dyn miniquad::RenderingBackend,
        textures: &TexturesContext, 
        gl: &QuadGl,
        ) -> Vec2 
    {
        let (width, height) = ctx.texture_size(self.raw_miniquad_id(textures, gl));

        vec2(width as f32, height as f32)
    }

    /// Sets the [FilterMode] of this texture.
    ///
    /// Use Nearest if you need integer-ratio scaling for pixel art, for example.
    ///
    /// # Example
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// let texture = Texture2D::empty();
    /// texture.set_filter(FilterMode::Linear);
    /// # }
    /// ```
    pub fn set_filter(
        &self, 
        ctx: &mut dyn miniquad::RenderingBackend, 
        textures: &TexturesContext, 
        gl: &QuadGl,
        filter_mode: FilterMode
        ) 
    {
        ctx.texture_set_filter(
            self.raw_miniquad_id(textures, gl),
            filter_mode,
            miniquad::MipmapFilterMode::None,
        );
    }

    /// Returns the handle for this texture.
    pub fn raw_miniquad_id(
        &self,
        textures: &TexturesContext, 
        gl: &QuadGl,
        ) -> miniquad::TextureId 
    {
        raw_miniquad_id(textures, gl, &self.texture)
    }

    /// Updates this texture from the screen.
    pub fn grab_screen(&self, 
        ctx: &dyn miniquad::RenderingBackend,
        textures: &TexturesContext, 
        gl: &QuadGl,
        )
    {
        use miniquad::*;
        let texture = self.raw_miniquad_id(textures, gl);
        let params = ctx.texture_params(texture);
        let raw_id = match unsafe { ctx.texture_raw_id(texture) } {
            miniquad::RawId::OpenGl(id) => id,
            _ => unimplemented!(),
        };
        let internal_format = match params.format {
            TextureFormat::RGB8 => miniquad::gl::GL_RGB,
            TextureFormat::RGBA8 => miniquad::gl::GL_RGBA,
            TextureFormat::RGBA16F => miniquad::gl::GL_RGBA,
            TextureFormat::Depth => miniquad::gl::GL_DEPTH_COMPONENT,
            TextureFormat::Depth32 => miniquad::gl::GL_DEPTH_COMPONENT,
            #[cfg(target_arch = "wasm32")]
            TextureFormat::Alpha => miniquad::gl::GL_ALPHA,
            #[cfg(not(target_arch = "wasm32"))]
            TextureFormat::Alpha => miniquad::gl::GL_R8,
        };
        unsafe {
            gl::glBindTexture(gl::GL_TEXTURE_2D, raw_id);
            gl::glCopyTexImage2D(
                gl::GL_TEXTURE_2D,
                0,
                internal_format,
                0,
                0,
                params.width as _,
                params.height as _,
                0,
            );
        }
    }

    /// Returns an [Image] from the pixel data in this texture.
    ///
    /// This operation can be expensive.
    pub fn get_texture_data(
        &self, 
        ctx: &mut dyn miniquad::RenderingBackend,
        textures: &TexturesContext, 
        gl: &QuadGl,
        ) -> Image 
    {
        let (width, height) = ctx.texture_size(self.raw_miniquad_id(textures, gl));
        let mut image = Image {
            width: width as _,
            height: height as _,
            bytes: vec![0; width as usize * height as usize * 4],
        };
        ctx.texture_read_pixels(self.raw_miniquad_id(textures, gl), &mut image.bytes);
        image
    }
}

pub(crate) struct Batcher {
    unbatched: Vec<Texture2D>,
    atlas: crate::text::atlas::Atlas,
}

impl Batcher {
    pub fn new(ctx: &mut dyn miniquad::RenderingBackend) -> Batcher {
        Batcher {
            unbatched: vec![],
            atlas: crate::text::atlas::Atlas::new(ctx, miniquad::FilterMode::Linear),
        }
    }

    pub fn add_unbatched(&mut self, texture: &Texture2D) {
        self.unbatched.push(texture.weak_clone());
    }

    pub fn get(
        &mut self, 
        quad_context: &mut dyn miniquad::RenderingBackend, 
        textures: &TexturesContext, 
        gl: &QuadGl,
        texture: &Texture2D
        ) -> Option<(Texture2D, Rect)> 
    {
        let id = SpriteKey::Texture(texture.raw_miniquad_id(textures, gl));
        let uv_rect = self.atlas.get_uv_rect(quad_context, id)?;
        Some((Texture2D::unmanaged(self.atlas.texture(quad_context)), uv_rect))
    }
}

/// Build an atlas out of all currently loaded texture
/// Later on all draw_texture calls with texture available in the atlas will use
/// the one from the atlas
/// NOTE: the GPU memory and texture itself in Texture2D will still be allocated
/// and Texture->Image conversions will work with Texture2D content, not the atlas
pub fn build_textures_atlas(
    quad_context: &mut dyn miniquad::RenderingBackend, 
    textures: &TexturesContext, 
    gl: &QuadGl,
    texture_batcher: &mut Batcher
    )
{
    for texture in texture_batcher.unbatched.drain(0..) {
        let sprite: Image = texture.get_texture_data(quad_context, textures, gl);
        let id = SpriteKey::Texture(texture.raw_miniquad_id(textures, gl));

        texture_batcher.atlas.cache_sprite(id, sprite);
    }

    let texture = texture_batcher.atlas.texture(quad_context);
    let (w, h) = quad_context.texture_size(texture);
}

/// Returns the handle for this texture.
pub fn raw_miniquad_id(
    textures: &TexturesContext, 
    gl: &QuadGl,
    handle: &TextureHandle
    ) -> miniquad::TextureId 
{
    match handle {
        TextureHandle::Unmanaged(texture) => *texture,
        TextureHandle::Managed(texture) =>
            textures
            .texture(texture.0)
            .unwrap_or(gl.white_texture),
        TextureHandle::ManagedWeak(texture) =>
            textures
            .texture(*texture)
            .unwrap_or(gl.white_texture),
    }
}

