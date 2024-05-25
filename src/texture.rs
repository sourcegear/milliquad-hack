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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TextureHandle {
    // raw miniquad texture, there are no guarantees that this texture is not yet deleted
    Unmanaged(miniquad::TextureId),
}

/// Image, data stored in CPU memory
#[derive(Clone)]
pub(crate) struct Image {
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

pub(crate) fn draw_texture_ex(
    gl: &mut QuadGl,
    quad_context: &mut dyn miniquad::RenderingBackend, 
    texture_batcher: &mut Batcher,
    texture: &Texture2D,
    x: f32,
    y: f32,
    color: Color,
    params: DrawTextureParams,
    ) 
{
    let [mut width, mut height] = texture.size(quad_context).to_array();

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
        .get(quad_context, texture)
        .map(|(batched_texture, uv)| {
            let [batched_width, batched_height] = batched_texture.size(quad_context).to_array();
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

    gl.texture(Some(&texture));
    gl.draw_mode(DrawMode::Triangles);
    gl.geometry(&vertices, &indices);
}

/// Texture, data stored in GPU memory
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Texture2D {
    pub(crate) texture: TextureHandle,
}

impl Texture2D {
    pub(crate) fn unmanaged(texture: miniquad::TextureId) -> Texture2D {
        Texture2D {
            texture: TextureHandle::Unmanaged(texture),
        }
    }

    pub fn size(
        &self, 
        ctx: &dyn miniquad::RenderingBackend,
        ) -> Vec2 
    {
        let (width, height) = ctx.texture_size(self.raw_miniquad_id());

        vec2(width as f32, height as f32)
    }

    /// Returns the handle for this texture.
    pub(crate) fn raw_miniquad_id(
        &self,
        ) -> miniquad::TextureId 
    {
        raw_miniquad_id(&self.texture)
    }

}

pub(crate) struct Batcher {
    atlas: crate::text::atlas::Atlas,
}

impl Batcher {
    pub fn new(ctx: &mut dyn miniquad::RenderingBackend) -> Batcher {
        Batcher {
            atlas: crate::text::atlas::Atlas::new(ctx, miniquad::FilterMode::Linear),
        }
    }

    pub fn get(
        &mut self, 
        quad_context: &mut dyn miniquad::RenderingBackend, 
        texture: &Texture2D
        ) -> Option<(Texture2D, Rect)> 
    {
        let id = SpriteKey::Texture(texture.raw_miniquad_id());
        let uv_rect = self.atlas.get_uv_rect(quad_context, id)?;
        Some((Texture2D::unmanaged(self.atlas.texture(quad_context)), uv_rect))
    }
}

/// Returns the handle for this texture.
pub(crate) fn raw_miniquad_id(
    handle: &TextureHandle
    ) -> miniquad::TextureId 
{
    match handle {
        TextureHandle::Unmanaged(texture) => *texture,
    }
}

