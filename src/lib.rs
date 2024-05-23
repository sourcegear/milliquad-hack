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

use std::fmt::{Display, Formatter};

use {
    crate::image::ImageFileFormat,
    std::io::{BufRead, Seek},
};

use quad_gl::{color::*, math::*, sprite_batcher::SpriteBatcher, Context3};
use std::sync::{Arc, Mutex};

//pub mod log { pub use miniquad::{debug, error, info, trace, warn}; }
pub use ::log as log;

pub mod text
{
    pub use quad_gl::text::Font;
    pub use quad_gl::text::TextDimensions;

    // TODO in my fork, TextParams no longer includes Font or font_size.
    // I've got code here to translate things.

    #[derive(Debug, Clone)]
    pub struct TextParams
    {
        /// The glyphs sizes actually drawn on the screen will be font_size * font_scale
        /// However with font_scale too different from 1.0 letters may be blurry
        pub font_scale: f32,
        /// Font X axis would be scaled by font_scale * font_scale_aspect
        /// and Y axis would be scaled by font_scale
        /// Default is 1.0
        pub font_scale_aspect: f32,
        /// Text rotation in radian
        /// Default is 0.0
        pub rotation: f32,
        pub color: crate::color::Color,
    }

    impl Default for TextParams {
        fn default() -> TextParams {
            TextParams {
                font_scale: 1.0,
                font_scale_aspect: 1.0,
                color: crate::color::Color::BLACK,
                rotation: 0.0,
            }
        }
    }

}

use crate::color::Color;
use crate::dimen::{UVec2, Vec2};
use crate::error::{BacktraceError, ErrorMessage};
use crate::image::{ImageDataType, ImageHandle, ImageSmoothingMode, RawBitmapData};
use crate::shape::{Polygon, Rect, Rectangle, RoundedRectangle};
use crate::window::WindowHandler;
use crate::window::{
    UserEventSender,
    WindowCreationError,
    WindowCreationOptions,
    WindowPosition,
    WindowSize
};
use crate::window_internal_quad::WindowQuad;

pub mod math;

/// Types representing colors.
pub mod color;

/// Types representing shapes.
pub mod shape;

/// Types representing sizes and positions.
pub mod dimen;

/// Utilities and traits for numeric values.
pub mod numeric;

/// Error types.
pub mod error;

/// Types relating to images.
pub mod image;

/// Utilities for accessing the system clock on all platforms.
pub mod time;

/// Allows for the creation and management of windows.
pub mod window;

mod window_internal_quad;

#[cfg(any(doc, doctest))]
mod window_internal_doctest;

#[cfg(target_arch = "wasm32")]
extern "C" {
    pub fn request_animation_frame();
}

/// An error encountered during the creation of a [GLRenderer].
#[derive(Clone, Debug)]
pub struct GLRendererCreationError
{
    description: String
}

impl GLRendererCreationError
{
    fn msg_with_cause<S, Cause>(description: S, cause: Cause) -> BacktraceError<Self>
    where
        S: AsRef<str>,
        Cause: std::error::Error + 'static
    {
        BacktraceError::new_with_cause(
            Self {
                description: description.as_ref().to_string()
            },
            cause
        )
    }

    #[allow(dead_code)]
    fn msg<S>(description: S) -> BacktraceError<Self>
    where
        S: AsRef<str>
    {
        BacktraceError::new(Self {
            description: description.as_ref().to_string()
        })
    }
}

impl Display for GLRendererCreationError
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result
    {
        Display::fmt("GL renderer creation error: ", f)?;
        Display::fmt(&self.description, f)
    }
}

/// A graphics renderer using an OpenGL backend.
///
/// Note: There is no need to use this struct if you are letting Speedy2D create
/// a window for you.
pub struct GLRenderer
{
    ctx: Context3
}

impl GLRenderer
{
    pub fn new_for_quad(
        ) -> Self
    {
        let ctx = miniquad::window::new_rendering_backend();
        let ctx = Arc::new(Mutex::new(ctx));

        let ctx = Context3::new(ctx.clone());

        GLRenderer { ctx }
    }

    pub fn create_font_from_bytes(&mut self, bytes: &[u8]) -> Result<quad_gl::text::Font, i32>
    {
        let f = self.ctx.load_ttf_font_from_bytes(bytes).unwrap();
        Ok(f)
    }

    /// Sets the renderer viewport to the specified pixel size, in response to a
    /// change in the window size.
    pub fn set_viewport_size_pixels(&mut self, viewport_size_pixels: UVec2)
    {
        //panic!();
    }

    /*
    /// Creates a new [ImageHandle] from the specified raw pixel data.
    ///
    /// The data provided in the `data` parameter must be in the format
    /// specified by `data_type`.
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    pub fn create_image_from_raw_pixels(
        &mut self,
        data_type: ImageDataType,
        smoothing_mode: ImageSmoothingMode,
        size: UVec2,
        data: &[u8]
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.canvas
            .create_image_from_raw_pixels(data_type, smoothing_mode, size, data)
    }
*/

    /// Loads an image from the provided encoded image file data.
    ///
    /// If no `data_type` is provided, an attempt will be made to guess the file
    /// format.
    ///
    /// The data source must implement `std::io::BufRead` and `std::io::Seek`.
    /// For example, if you have a `&[u8]`, you may wrap it in a
    /// `std::io::Cursor` as follows:
    ///
    /// ```rust,no_run
    /// # use speedy2d::GLRenderer;
    /// # use speedy2d::color::Color;
    /// # use speedy2d::image::ImageSmoothingMode;
    /// use std::io::Cursor;
    /// # let mut renderer = unsafe {
    /// #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
    /// #         std::ptr::null() as *const _
    /// #     })
    /// # }.unwrap();
    ///
    /// let image_bytes : &[u8] = include_bytes!("../assets/screenshots/hello_world.png");
    ///
    /// let image_result = renderer.create_image_from_file_bytes(
    ///     None,
    ///     ImageSmoothingMode::Linear,
    ///     Cursor::new(image_bytes));
    /// ```
    ///
    /// For a list of supported image types, see [image::ImageFileFormat].
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    /*
    pub fn create_image_from_file_bytes<R: Seek + BufRead>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        file_bytes: R
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        self.renderer
            .create_image_from_file_bytes(data_type, smoothing_mode, file_bytes)
    }
*/

    /// Starts the process of drawing a frame. A `Graphics2D` object will be
    /// provided to the callback. When the callback returns, the internal
    /// render queue will be flushed.
    ///
    /// Note: if calling this method, you are responsible for swapping the
    /// window context buffers if necessary.
    #[inline]
    pub fn draw_frame<F: FnOnce(&mut Graphics2D) -> R, R>(&mut self, callback: F) -> R
    {
        //self.renderer.set_clip(None);
        let mut g = Graphics2D { ctx: self.ctx.clone(), canvas: self.ctx.new_canvas() };
        g.begin_frame();
        let result = callback(&mut g);
        g.end_frame();
        result
    }
}

impl Drop for GLRenderer
{
    fn drop(&mut self)
    {
        //self.context.mark_invalid();
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
    ctx: Context3,
    canvas: SpriteBatcher,
}

impl Graphics2D
{
    /// Creates a new [ImageHandle] from the specified raw pixel data.
    ///
    /// The data provided in the `data` parameter must be in the format
    /// specified by `data_type`.
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    pub fn create_image_from_raw_pixels<S: Into<UVec2>>(
        &mut self,
        data_type: ImageDataType,
        smoothing_mode: ImageSmoothingMode,
        size: S,
        data: &[u8]
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        panic!();
    }

    /// Loads an image from the provided encoded image file data.
    ///
    /// If no `data_type` is provided, an attempt will be made to guess the file
    /// format.
    ///
    /// The data source must implement `std::io::BufRead` and `std::io::Seek`.
    /// For example, if you have a `&[u8]`, you may wrap it in a
    /// `std::io::Cursor` as follows:
    ///
    /// ```rust,no_run
    /// # use speedy2d::GLRenderer;
    /// # use speedy2d::color::Color;
    /// # use speedy2d::image::ImageSmoothingMode;
    /// use std::io::Cursor;
    /// # let mut renderer = unsafe {
    /// #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
    /// #         std::ptr::null() as *const _
    /// #     })
    /// # }.unwrap();
    /// # renderer.draw_frame(|graphics| {
    ///
    /// let image_bytes : &[u8] = include_bytes!("../assets/screenshots/hello_world.png");
    ///
    /// let image_result = graphics.create_image_from_file_bytes(
    ///     None,
    ///     ImageSmoothingMode::Linear,
    ///     Cursor::new(image_bytes));
    /// # });
    /// ```
    ///
    /// For a list of supported image types, see [image::ImageFileFormat].
    ///
    /// The returned [ImageHandle] is valid only for the current graphics
    /// context.
    pub fn create_image_from_file_bytes<R: Seek + BufRead>(
        &mut self,
        data_type: Option<ImageFileFormat>,
        smoothing_mode: ImageSmoothingMode,
        file_bytes: R
    ) -> Result<ImageHandle, BacktraceError<ErrorMessage>>
    {
        panic!();
    }

    /// Fills the screen with the specified color.
    pub fn clear_screen(&mut self, color: Color)
    {
        self.ctx.quad_ctx.lock().unwrap().clear(Some((color.r(), color.g(), color.b(), color.a())), None, None);
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font: &quad_gl::text::Font,
        font_size: u16,
        parms: crate::text::TextParams
    )
    {
        // translate TextParams to what quad_gl wants
        let p = quad_gl::text::TextParams
        {
            font: Some(font),
            font_size: font_size,
            font_scale: parms.font_scale,
            font_scale_aspect: parms.font_scale_aspect,
            rotation: parms.rotation,
            color: parms.color.to_quad(),
        };

        quad_gl::text::draw_text_ex(
            &mut self.canvas,
            text,
            x,
            y,
            p
            );
    }

    /// Draws a polygon with a single color, with the specified offset in
    /// pixels.
    pub fn draw_polygon<V: Into<Vec2>>(
        &mut self,
        polygon: &Polygon,
        offset: V,
        color: Color
    )
    {
        panic!();
    }

    /// Draws a triangle with the specified colors (one color for each corner).
    ///
    /// The vertex positions (and associated colors) must be provided in
    /// clockwise order.
    pub fn draw_triangle_three_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 3],
        vertex_colors_clockwise: [Color; 3]
    )
    {
        //panic!();
    }

    /// Draws part of an image, tinted with the provided colors, at the
    /// specified location. The sub-image will be scaled to fill the
    /// triangle described by the vertices in `vertex_positions_clockwise`.
    ///
    /// The coordinates in `image_coords_normalized` should be in the range
    /// `0.0` to `1.0`, and define the portion of the source image which
    /// should be drawn.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    ///
    /// The vertex positions (and associated colors and image coordinates) must
    /// be provided in clockwise order.
    pub fn draw_triangle_image_tinted_three_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 3],
        vertex_colors: [Color; 3],
        image_coords_normalized: [Vec2; 3],
        image: &ImageHandle
    )
    {
        panic!();
    }

    /// Draws a triangle with the specified color.
    ///
    /// The vertex positions must be provided in clockwise order.
    #[inline]
    pub fn draw_triangle(&mut self, vertex_positions_clockwise: [Vec2; 3], color: Color)
    {
        //self.draw_triangle_three_color(vertex_positions_clockwise, [color, color, color]);
        self.canvas.draw_triangle(
            quad_gl::math::Vec2::new(vertex_positions_clockwise[0].x, vertex_positions_clockwise[0].y),
            quad_gl::math::Vec2::new(vertex_positions_clockwise[1].x, vertex_positions_clockwise[1].y),
            quad_gl::math::Vec2::new(vertex_positions_clockwise[2].x, vertex_positions_clockwise[2].y),
            color.to_quad());
    }

    /// Draws a quadrilateral with the specified colors (one color for each
    /// corner).
    ///
    /// The vertex positions (and associated colors) must be provided in
    /// clockwise order.
    #[inline]
    pub fn draw_quad_four_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 4],
        vertex_colors: [Color; 4]
    )
    {
        let vp = vertex_positions_clockwise;
        let vc = vertex_colors;

        self.draw_triangle_three_color([vp[0], vp[1], vp[2]], [vc[0], vc[1], vc[2]]);

        self.draw_triangle_three_color([vp[2], vp[3], vp[0]], [vc[2], vc[3], vc[0]]);
    }

    /// Draws a quadrilateral with the specified color.
    ///
    /// The vertex positions must be provided in clockwise order.
    #[inline]
    pub fn draw_quad(&mut self, vertex_positions_clockwise: [Vec2; 4], color: Color)
    {
        self.draw_quad_four_color(
            vertex_positions_clockwise,
            [color, color, color, color]
        );
    }

    /// Draws part of an image, tinted with the provided colors, at the
    /// specified location. The sub-image will be scaled to fill the
    /// quadrilateral described by the vertices in
    /// `vertex_positions_clockwise`.
    ///
    /// The coordinates in `image_coords_normalized` should be in the range
    /// `0.0` to `1.0`, and define the portion of the source image which
    /// should be drawn.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    ///
    /// The vertex positions (and associated colors and image coordinates) must
    /// be provided in clockwise order.
    #[inline]
    pub fn draw_quad_image_tinted_four_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 4],
        vertex_colors: [Color; 4],
        image_coords_normalized: [Vec2; 4],
        image: &ImageHandle
    )
    {
        let vp = vertex_positions_clockwise;
        let vc = vertex_colors;
        let ic = image_coords_normalized;

        self.draw_triangle_image_tinted_three_color(
            [vp[0], vp[1], vp[2]],
            [vc[0], vc[1], vc[2]],
            [ic[0], ic[1], ic[2]],
            image
        );

        self.draw_triangle_image_tinted_three_color(
            [vp[2], vp[3], vp[0]],
            [vc[2], vc[3], vc[0]],
            [ic[2], ic[3], ic[0]],
            image
        );
    }

    /// Draws part of an image, tinted with the provided color, at the specified
    /// location. The sub-image will be scaled to fill the pixel coordinates
    /// in the provided rectangle.
    ///
    /// The coordinates in `image_coords_normalized` should be in the range
    /// `0.0` to `1.0`, and define the portion of the source image which
    /// should be drawn.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    #[inline]
    pub fn draw_rectangle_image_subset_tinted(
        &mut self,
        rect: impl AsRef<Rectangle>,
        color: Color,
        image_coords_normalized: impl AsRef<Rectangle>,
        image: &ImageHandle
    )
    {
        let rect = rect.as_ref();
        let image_coords_normalized = image_coords_normalized.as_ref();

        self.draw_quad_image_tinted_four_color(
            [
                *rect.top_left(),
                rect.top_right(),
                *rect.bottom_right(),
                rect.bottom_left()
            ],
            [color, color, color, color],
            [
                *image_coords_normalized.top_left(),
                image_coords_normalized.top_right(),
                *image_coords_normalized.bottom_right(),
                image_coords_normalized.bottom_left()
            ],
            image
        );
    }

    /// Draws an image, tinted with the provided color, at the specified
    /// location. The image will be scaled to fill the pixel coordinates in
    /// the provided rectangle.
    ///
    /// The tinting is performed by for each pixel by multiplying each color
    /// component in the image pixel by the corresponding color component in
    /// the `color` parameter.
    #[inline]
    pub fn draw_rectangle_image_tinted(
        &mut self,
        rect: impl AsRef<Rectangle>,
        color: Color,
        image: &ImageHandle
    )
    {
        self.draw_rectangle_image_subset_tinted(
            rect,
            color,
            Rectangle::new(Vec2::ZERO, Vec2::new(1.0, 1.0)),
            image
        );
    }

    /// Draws an image at the specified location. The image will be
    /// scaled to fill the pixel coordinates in the provided rectangle.
    #[inline]
    pub fn draw_rectangle_image(
        &mut self,
        rect: impl AsRef<Rectangle>,
        image: &ImageHandle
    )
    {
        self.draw_rectangle_image_tinted(rect, Color::WHITE, image);
    }

    /// Draws an image at the specified pixel location. The image will be
    /// drawn at its original size with no scaling.
    #[inline]
    pub fn draw_image<P: Into<Vec2>>(&mut self, position: P, image: &ImageHandle)
    {
        let position = position.into();

        self.draw_rectangle_image(
            Rectangle::new(position, position + image.size().into_f32()),
            image
        );
    }

    /// Draws a single-color rectangle at the specified location. The
    /// coordinates of the rectangle are specified in pixels.
    #[inline]
    pub fn draw_rectangle(&mut self, rect: impl AsRef<Rectangle>, color: Color)
    {
        let rect = rect.as_ref();

        /*
        self.draw_quad(
            [
                *rect.top_left(),
                rect.top_right(),
                *rect.bottom_right(),
                rect.bottom_left()
            ],
            color
        );
        */
        //log::info!("rect: {:?} {:?}", rect, color);
        self.canvas.draw_rectangle(rect.left(), rect.top(), rect.width(), rect.height(), color.to_quad());
    }

    /// Draws a single-color rounded rectangle at the specified location. The
    /// coordinates of the rounded rectangle are specified in pixels.
    #[inline]
    pub fn draw_rounded_rectangle(
        &mut self,
        round_rect: impl AsRef<RoundedRectangle>,
        color: Color
    )
    {
        let round_rect = round_rect.as_ref();
        self.canvas.draw_rectangle_ex2(
            round_rect.left(),
            round_rect.top(),
            round_rect.width(),
            round_rect.height(),
            &quad_gl::rounded_rect::DrawRectangleParams2
            {
                color: color.to_quad(),
                border_radius: round_rect.radius(),
                 border_radius_segments: 20,
                ..Default::default()
            }
            );
    }

    /// Draws a single-color line between the given points, specified in pixels.
    ///
    /// # Pixel alignment
    ///
    /// On a display with square pixels, an integer-valued coordinate is located
    /// at the boundary between two pixels, rather than the center of the
    /// pixel. For example:
    ///
    ///  * `(0.0, 0.0)` = Top left of pixel
    ///  * `(0.5, 0.5)` = Center of pixel
    ///  * `(1.0, 1.0)` = Bottom right of pixel
    ///
    /// If drawing a line of odd-numbered thickness, it is advisable to locate
    /// the start and end of the line at the centers of pixels, rather than
    /// the edges.
    ///
    /// For example, a one-pixel-thick line between `(0.0, 10.0)` and `(100.0,
    /// 10.0)` will be drawn as a rectangle with corners `(0.0, 9.5)` and
    /// `(100.0, 10.5)`, meaning that the line's thickness will actually
    /// span two half-pixels. Drawing the same line between `(0.0, 10.5)`
    /// and `(100.0, 10.5)` will result in a pixel-aligned rectangle between
    /// `(0.0, 10.0)` and `(100.0, 11.0)`.
    pub fn draw_line<VStart: Into<Vec2>, VEnd: Into<Vec2>>(
        &mut self,
        start_position: VStart,
        end_position: VEnd,
        thickness: f32,
        color: Color
    )
    {
        let start_position = start_position.into();
        let end_position = end_position.into();

        let gradient_normalized = match (end_position - start_position).normalize() {
            None => return,
            Some(gradient) => gradient
        };

        let gradient_thickness = gradient_normalized * (thickness / 2.0);

        let offset_anticlockwise = gradient_thickness.rotate_90_degrees_anticlockwise();
        let offset_clockwise = gradient_thickness.rotate_90_degrees_clockwise();

        let start_anticlockwise = start_position + offset_anticlockwise;
        let start_clockwise = start_position + offset_clockwise;

        let end_anticlockwise = end_position + offset_anticlockwise;
        let end_clockwise = end_position + offset_clockwise;

        self.draw_quad(
            [
                start_anticlockwise,
                end_anticlockwise,
                end_clockwise,
                start_clockwise
            ],
            color
        );
    }

    /// Draws a circle, filled with a single color, at the specified pixel
    /// location.
    pub fn draw_circle<V: Into<Vec2>>(
        &mut self,
        center_position: V,
        radius: f32,
        color: Color
    )
    {
    }

    /// Draws a triangular subset of a circle.
    ///
    /// Put simply, this function will draw a triangle on the screen, textured
    /// with a region of a circle.
    ///
    /// The circle region is specified using `vertex_circle_coords_normalized`,
    /// which denotes UV coordinates relative to an infinitely-detailed
    /// circle of radius `1.0`, and center `(0.0, 0.0)`.
    ///
    /// For example, to draw the top-right half of a circle with radius 100px:
    ///
    /// ```rust,no_run
    /// # use speedy2d::GLRenderer;
    /// # use speedy2d::dimen::Vec2;
    /// # use speedy2d::color::Color;
    /// # let mut renderer = unsafe {
    /// #     GLRenderer::new_for_gl_context((640, 480), |fn_name| {
    /// #         std::ptr::null() as *const _
    /// #     })
    /// # }.unwrap();
    /// # renderer.draw_frame(|graphics| {
    /// graphics.draw_circle_section_triangular_three_color(
    ///         [
    ///                 Vec2::new(200.0, 200.0),
    ///                 Vec2::new(300.0, 200.0),
    ///                 Vec2::new(300.0, 300.0)],
    ///         [Color::MAGENTA; 3],
    ///         [
    ///                 Vec2::new(-1.0, -1.0),
    ///                 Vec2::new(1.0, -1.0),
    ///                 Vec2::new(1.0, 1.0)]);
    /// # });
    /// ```
    #[inline]
    pub fn draw_circle_section_triangular_three_color(
        &mut self,
        vertex_positions_clockwise: [Vec2; 3],
        vertex_colors: [Color; 3],
        vertex_circle_coords_normalized: [Vec2; 3]
        )
    {
        /*
        shapes::draw_triangle(
            &mut self.gl, 
            glam::Vec2::new(vertex_positions_clockwise[0].x, vertex_positions_clockwise[0].y),
            glam::Vec2::new(vertex_positions_clockwise[1].x, vertex_positions_clockwise[1].y),
            glam::Vec2::new(vertex_positions_clockwise[2].x, vertex_positions_clockwise[2].y),
            vertex_colors[0]);
            */
        //self.renderer.draw_circle_section(
            //vertex_positions_clockwise,
            //vertex_colors,
            //vertex_circle_coords_normalized
        //);
    }

    /// Sets the current clip to the rectangle specified by the given
    /// coordinates. Rendering operations have no effect outside of the
    /// clipping area.
    pub fn set_clip(&mut self, rect: Option<Rectangle<i32>>)
    {
        // TODO
    }

    /// Captures a screenshot of the render window. The returned data contains
    /// the color of each pixel. Pixels are represented using a `u8` for each
    /// component (red, green, blue, and alpha). Use the `format` parameter to
    /// specify the byte layout (and size) of each pixel.
    pub fn capture(&mut self, format: ImageDataType) -> RawBitmapData
    {
        panic!();
    }

    fn begin_frame(&mut self) {
        self.canvas.reset();
    }

    pub(crate) fn pixel_perfect_projection_matrix(&self) -> quad_gl::math::Mat4 {
        let (width, height) = miniquad::window::screen_size();
        let dpi = miniquad::window::dpi_scale();

        quad_gl::math::Mat4::orthographic_rh_gl(0., width / dpi, height / dpi, 0., -1., 1.)
    }

    fn end_frame(&mut self) {
        let screen_mat = self.pixel_perfect_projection_matrix();
        //self.gl.draw(&mut *self.renderer, screen_mat);
        self.canvas.draw3(screen_mat);

        // TODO do we need this?
        self.ctx.quad_ctx.lock().unwrap().commit_frame();
    }

}

/// Struct representing a window.
pub struct Window<UserEventType = ()>
where
    UserEventType: 'static
{
    window_impl: WindowQuad<UserEventType>,
}

impl Window<()>
{
    /// Create a new window, centered in the middle of the primary monitor.
    pub fn new_centered<Str, Size>(
        title: Str,
        size: Size
    ) -> Result<Window<()>, BacktraceError<WindowCreationError>>
    where
        Str: AsRef<str>,
        Size: Into<UVec2>
    {
        let size = size.into();

        Self::new_with_options(
            title.as_ref(),
            WindowCreationOptions::new_windowed(
                WindowSize::PhysicalPixels(size),
                Some(WindowPosition::Center)
            )
        )
    }

    /// Create a new window, in fullscreen borderless mode on the primary
    /// monitor.
    pub fn new_fullscreen_borderless<Str>(
        title: Str
    ) -> Result<Window<()>, BacktraceError<WindowCreationError>>
    where
        Str: AsRef<str>
    {
        Self::new_with_options(
            title.as_ref(),
            WindowCreationOptions::new_fullscreen_borderless()
        )
    }

    /// Create a new window with the specified options.
    pub fn new_with_options(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<Window<()>, BacktraceError<WindowCreationError>>
    {
        Self::new_with_user_events(title, options)
    }
}

impl<UserEventType: 'static> Window<UserEventType>
{
    /// Create a new window with the specified options, with support for user
    /// events. See [window::UserEventSender].
    pub fn new_with_user_events(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<Self, BacktraceError<WindowCreationError>>
    {
        let window_impl = WindowQuad::<UserEventType>::new(title, options)?;

        Ok(Window {
            window_impl
        })
    }

    /// Creates a [window::UserEventSender], which can be used to post custom
    /// events to this event loop from another thread.
    ///
    /// If calling this, specify the type of the event data using
    /// `Window::<YourTypeHere>::new_with_user_events()`.
    ///
    /// See [UserEventSender::send_event], [WindowHandler::on_user_event].
    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        self.window_impl.create_user_event_sender()
    }

    /// Run the window event loop, with the specified callback handler.
    ///
    /// Once the event loop finishes running, the entire app will terminate,
    /// even if other threads are still running. See
    /// [window::WindowHelper::terminate_loop()].
    pub fn run_loop<H>(self, handler: H)
    where
        H: WindowHandler<UserEventType> + 'static
    {
        self.window_impl.run_loop(handler);
    }
}

