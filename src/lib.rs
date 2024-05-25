/*
 *  Copyright 2021 QuantumBadger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//pub mod log { pub use miniquad::{debug, error, info, trace, warn}; }
pub use ::log as log;

use crate::color::Color;
use crate::dimen::{UVec2, };
use crate::window::WindowHandler;
use crate::window_internal_quad::WindowQuad;

pub mod math;
pub mod text;
mod shapes;
mod quad_gl;
mod texture;

/// Types representing colors.
pub mod color;

/// Types representing sizes and positions.
pub mod dimen;

/// Utilities for accessing the system clock on all platforms.
pub mod time;

/// Allows for the creation and management of windows.
pub mod window;

mod window_internal_quad;

#[cfg(target_arch = "wasm32")]
extern "C" {
    pub fn request_animation_frame();
}

use quad_gl::QuadGl;

/// A graphics renderer using an OpenGL backend.
///
/// Note: There is no need to use this struct if you are letting Speedy2D create
/// a window for you.
pub(crate) struct GLRenderer
{
    renderer: Graphics2D
}

impl GLRenderer
{
    pub fn new() -> Self
    {
        let mut ctx: Box<dyn miniquad::RenderingBackend> =
            miniquad::window::new_rendering_backend();

        let gl = QuadGl::new(&mut *ctx);
        let texture_batcher = crate::texture::Batcher::new(&mut *ctx);
        let renderer = Graphics2D {
            renderer: ctx,
            gl:  gl,
            texture_batcher: texture_batcher,
        };

        GLRenderer { renderer }
    }

    pub fn create_font_from_bytes(&mut self, bytes: &[u8]) -> Result<crate::text::Font, &'static str>
    {
        let f = text::load_ttf_font_from_bytes(&mut *self.renderer.renderer, bytes)?;
        Ok(f)
    }

    /// Sets the renderer viewport to the specified pixel size, in response to a
    /// change in the window size.
    pub fn set_viewport_size_pixels(&mut self, viewport_size_pixels: UVec2)
    {
        //panic!();
    }

    /// Starts the process of drawing a frame. A `Graphics2D` object will be
    /// provided to the callback. When the callback returns, the internal
    /// render queue will be flushed.
    ///
    /// Note: if calling this method, you are responsible for swapping the
    /// window context buffers if necessary.
    #[inline]
    pub fn draw_frame<F: FnOnce(&mut Graphics2D) -> R, R>(&mut self, callback: F) -> R
    {
        self.renderer.begin_frame();
        let result = callback(&mut self.renderer);
        self.renderer.end_frame();
        result
    }
}

impl Drop for GLRenderer
{
    fn drop(&mut self)
    {
        // TODO ?  self.context.mark_invalid();
    }
}

/// A `Graphics2D` object allows you to draw shapes, images, and text to the
/// screen.
///
/// An instance is provided in the [window::WindowHandler::on_draw] callback.
///
/// If you are managing the GL context yourself, you must invoke
/// [GLRenderer::draw_frame] to obtain an instance.
pub struct Graphics2D
{
    renderer: Box<dyn miniquad::RenderingBackend>,
    gl: QuadGl,
    texture_batcher: crate::texture::Batcher,
}

impl Graphics2D
{
    /// Fills the screen with the specified color.
    pub fn clear_screen(&mut self, color: Color)
    {
        self.renderer.clear(Some((color.r, color.g, color.b, color.a)), None, None);
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font: &crate::text::Font,
        font_size: u16,
        parms: crate::text::TextParams
    )
    {
        crate::text::draw_text_ex(
            &mut self.gl,
            &mut *self.renderer,
            &mut self.texture_batcher,
            text,
            x,
            y,
            font,
            font_size,
            parms
            );
    }

    /// Draws a triangle with the specified color.
    ///
    /// The vertex positions must be provided in clockwise order.
    #[inline]
    pub fn draw_triangle(&mut self, vertex_positions_clockwise: [glam::Vec2; 3], color: Color)
    {
        //self.draw_triangle_three_color(vertex_positions_clockwise, [color, color, color]);
        shapes::draw_triangle(
            &mut self.gl, 
            vertex_positions_clockwise[0],
            vertex_positions_clockwise[1],
            vertex_positions_clockwise[2],
            color);
    }

    /// Draws a single-color rectangle at the specified location. The
    /// coordinates of the rectangle are specified in pixels.
    #[inline]
    pub fn draw_rectangle(&mut self, rect: &crate::math::Rect, color: Color)
    {
        shapes::draw_rectangle(&mut self.gl, rect.left(), rect.top(), rect.width(), rect.height(), color);
    }

    /// Draws a single-color rounded rectangle at the specified location. The
    /// coordinates of the rounded rectangle are specified in pixels.
    #[inline]
    pub fn draw_rounded_rectangle(
        &mut self,
        rect: &crate::math::Rect,
        radius: f32,
        color: Color
    )
    {
        shapes::draw_rectangle_ex2(
            &mut self.gl,
            rect.left(),
            rect.top(),
            rect.width(),
            rect.height(),
            &shapes::DrawRectangleParams2
            {
                color: color,
                border_radius: radius,
                 border_radius_segments: 20,
                ..Default::default()
            }
            );
    }

    fn begin_frame(&mut self) {
        self.gl.reset();
    }

    pub(crate) fn pixel_perfect_projection_matrix(&self) -> glam::Mat4 {
        let (width, height) = miniquad::window::screen_size();
        let dpi = miniquad::window::dpi_scale();

        glam::Mat4::orthographic_rh_gl(0., width / dpi, height / dpi, 0., -1., 1.)
    }

    fn end_frame(&mut self) {
        let screen_mat = self.pixel_perfect_projection_matrix();
        self.gl.draw(&mut *self.renderer, screen_mat);

        self.renderer.commit_frame();
    }

}

/// Struct representing a window.
pub struct Window
{
    window_impl: WindowQuad,
}

impl Window
{
    /// Create a new window with the specified options, with support for user
    /// events. See [window::UserEventSender].
    pub fn new(
        conf: miniquad::conf::Conf,
    ) -> Self
    {
        let window_impl = WindowQuad::new(conf);

        Window {
            window_impl
        }
    }

    /// Run the window event loop, with the specified callback handler.
    ///
    /// Once the event loop finishes running, the entire app will terminate,
    /// even if other threads are still running. See
    /// [window::WindowHelper::terminate_loop()].
    pub fn run_loop<H>(self, handler: H)
    where
        H: WindowHandler + 'static
    {
        self.window_impl.run_loop(handler);
    }
}

