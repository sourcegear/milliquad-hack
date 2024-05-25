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

use std::rc::Rc;
use std::cell::Cell;
use std::cell::RefCell;
use crate::dimen::{UVec2, };
use crate::{GLRenderer, Graphics2D};

pub use miniquad::conf::Conf;
pub use miniquad::KeyCode;
pub use miniquad::KeyMods;
pub use miniquad::MouseButton;

/// A set of callbacks for an active window. If a callback is not implemented,
/// it will do nothing by default, so it is only necessary to implement the
/// callbacks you actually need.
pub trait WindowHandler
{
    /// Invoked once when the window first starts.
    #[allow(unused_variables)]
    #[inline]
    fn on_start(
        &mut self,
        helper: &mut WindowHelper,
        info: WindowStartupInfo
    )
    {
    }

    #[allow(unused_variables)]
    #[inline]
    fn on_update(
        &mut self,
        helper: &mut WindowHelper,
    )
    {
    }

    /// Invoked when the window is resized.
    #[allow(unused_variables)]
    #[inline]
    fn on_resize(&mut self, helper: &mut WindowHelper, size_pixels: UVec2)
    {
    }

    /// Invoked when the window scale factor changes.
    #[allow(unused_variables)]
    #[inline]
    fn on_scale_factor_changed(
        &mut self,
        helper: &mut WindowHelper,
        scale_factor: f64
    )
    {
    }

    /// Invoked when the contents of the window needs to be redrawn.
    ///
    /// It is possible to request a redraw from any callback using
    /// [WindowHelper::request_redraw].
    #[allow(unused_variables)]
    #[inline]
    fn on_draw(
        &mut self,
        helper: &mut WindowHelper,
        graphics: &mut Graphics2D
    )
    {
    }

    /// Invoked when the mouse changes position.
    ///
    /// Normally, this provides the absolute  position of the mouse in the
    /// window/canvas. However, if the mouse cursor is grabbed, this will
    /// instead provide the amount of relative movement since the last move
    /// event.
    ///
    /// See [WindowHandler::on_mouse_grab_status_changed].
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_move(&mut self, helper: &mut WindowHelper, x: f32, y: f32)
    {
    }

    /// Invoked when a mouse button is pressed.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper,
        button: MouseButton,
        x: f32,
        y: f32,
    )
    {
    }

    /// Invoked when a mouse button is released.
    #[allow(unused_variables)]
    #[inline]
    fn on_mouse_button_up(
        &mut self,
        helper: &mut WindowHelper,
        button: MouseButton,
        x: f32,
        y: f32,
    )
    {
    }

    /// Invoked when a keyboard key is pressed.
    ///
    /// To detect when a character is typed, see the
    /// [WindowHandler::on_keyboard_char] callback.
    #[allow(unused_variables)]
    #[inline]
    fn on_key_down(
        &mut self,
        helper: &mut WindowHelper,
        key_code: miniquad::KeyCode,
        modifiers: miniquad::KeyMods, 
        repeat: bool
    )
    {
    }

    /// Invoked when a keyboard key is released.
    #[allow(unused_variables)]
    #[inline]
    fn on_key_up(
        &mut self,
        helper: &mut WindowHelper,
        key_code: miniquad::KeyCode,
        modifiers: miniquad::KeyMods, 
    )
    {
    }

    /// Invoked when a character is typed on the keyboard.
    ///
    /// This is invoked in addition to the [WindowHandler::on_key_up] and
    /// [WindowHandler::on_key_down] callbacks.
    #[allow(unused_variables)]
    #[inline]
    fn on_keyboard_char(
        &mut self,
        helper: &mut WindowHelper,
        unicode_codepoint: char,
        modifiers: miniquad::KeyMods, 
        repeat: bool
    )
    {
    }

}

pub(crate) struct DrawingWindowHandler<H>
where
    H: WindowHandler
{
    window_handler: H,
    renderer: Rc<RefCell<GLRenderer>>,
}

impl<H> DrawingWindowHandler<H>
where
    H: WindowHandler
{
    pub fn new(window_handler: H, renderer: Rc<RefCell<GLRenderer>>) -> Self
    {
        DrawingWindowHandler {
            window_handler,
            renderer,
        }
    }

    #[inline]
    pub fn on_start(
        &mut self,
        helper: &mut WindowHelper,
        info: WindowStartupInfo
    )
    {
        self.window_handler.on_start(helper, info);
    }

    #[inline]
    pub fn on_update(
        &mut self,
        helper: &mut WindowHelper,
    )
    {
        self.window_handler.on_update(helper)
    }

    #[inline]
    pub fn on_resize(
        &mut self,
        helper: &mut WindowHelper,
        size_pixels: UVec2
    )
    {
        self.renderer.borrow_mut().set_viewport_size_pixels(size_pixels);
        self.window_handler.on_resize(helper, size_pixels)
    }

    #[inline]
    pub fn on_scale_factor_changed(
        &mut self,
        helper: &mut WindowHelper,
        scale_factor: f64
    )
    {
        self.window_handler
            .on_scale_factor_changed(helper, scale_factor)
    }

    #[inline]
    pub fn on_draw(&mut self, helper: &mut WindowHelper)
    {
        let renderer = &mut self.renderer;
        let window_handler = &mut self.window_handler;

        renderer.borrow_mut().draw_frame(|graphics| window_handler.on_draw(helper, graphics))
    }

    #[inline]
    pub fn on_mouse_move(
        &mut self,
        helper: &mut WindowHelper,
        x: f32,
        y: f32,
    )
    {
        self.window_handler.on_mouse_move(helper, x, y)
    }

    #[inline]
    pub fn on_mouse_button_down(
        &mut self,
        helper: &mut WindowHelper,
        button: MouseButton,
        x: f32,
        y: f32,
    )
    {
        self.window_handler.on_mouse_button_down(helper, button, x, y)
    }

    #[inline]
    pub fn on_mouse_button_up(
        &mut self,
        helper: &mut WindowHelper,
        button: MouseButton,
        x: f32,
        y: f32,
    )
    {
        self.window_handler.on_mouse_button_up(helper, button, x, y)
    }

    #[inline]
    pub fn on_key_down(
        &mut self,
        helper: &mut WindowHelper,
        key_code: miniquad::KeyCode,
        modifiers: miniquad::KeyMods, 
        repeat: bool
    )
    {
        self.window_handler
            .on_key_down(helper, key_code, modifiers, repeat)
    }

    #[inline]
    pub fn on_key_up(
        &mut self,
        helper: &mut WindowHelper,
        key_code: miniquad::KeyCode,
        modifiers: miniquad::KeyMods, 
    )
    {
        self.window_handler
            .on_key_up(helper, key_code, modifiers)
    }

    #[inline]
    pub fn on_keyboard_char(
        &mut self,
        helper: &mut WindowHelper,
        unicode_codepoint: char,
        modifiers: miniquad::KeyMods, 
        repeat: bool
    )
    {
        self.window_handler
            .on_keyboard_char(helper, unicode_codepoint, modifiers, repeat)
    }

}

pub struct WindowHelper
{
    renderer: Rc<RefCell<GLRenderer>>,
    redraw_requested: Cell<bool>,
}

impl WindowHelper
{
    #[inline]
    pub(crate) fn new(
        renderer: Rc<RefCell<GLRenderer>>,
    ) -> Self
    {
        WindowHelper {
            renderer: renderer,
            redraw_requested: Cell::new(false),
        }
    }

    #[must_use]
    pub fn create_font_from_bytes(&self, bytes: &[u8]) -> Result<crate::text::Font,&'static str>
    {
        self.renderer.borrow_mut().create_font_from_bytes(bytes)
    }

    #[inline]
    pub(crate) fn set_redraw_requested(&mut self, redraw_requested: bool)
    {
        self.redraw_requested.set(redraw_requested);
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        if !self.redraw_requested.get()
        {
            #[cfg(target_arch = "wasm32")]
            unsafe {crate::request_animation_frame(); }

            self.redraw_requested.set(true);
        }
    }

    pub fn get_size_pixels(&self) -> UVec2
    {
        let (w, h) = miniquad::window::screen_size();
        let dpi = miniquad::window::dpi_scale();
        return UVec2::new((w / dpi) as u32, (h / dpi) as u32);
    }

    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        miniquad::window::dpi_scale().into()
    }

}

/// Information about the starting state of the window.
#[derive(Debug, PartialEq, Clone)]
pub struct WindowStartupInfo
{
    viewport_size_pixels: UVec2,
    scale_factor: f64
}

impl WindowStartupInfo
{
    pub(crate) fn new(viewport_size_pixels: UVec2, scale_factor: f64) -> Self
    {
        WindowStartupInfo {
            viewport_size_pixels,
            scale_factor
        }
    }

    /// The scale factor of the window. When a high-dpi display is in use,
    /// this will be greater than `1.0`.
    pub fn scale_factor(&self) -> f64
    {
        self.scale_factor
    }

    /// The size of the viewport in pixels.
    pub fn viewport_size_pixels(&self) -> &UVec2
    {
        &self.viewport_size_pixels
    }
}

