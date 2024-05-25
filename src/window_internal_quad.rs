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

use std::cell::RefCell;
use std::rc::Rc;

use crate::dimen::{UVec2, };
use crate::window::{
    DrawingWindowHandler,
    MouseButton,
    WindowHandler,
    WindowHelper,
    WindowStartupInfo
};
use crate::GLRenderer;

pub(crate) struct WindowQuad
{
    conf: miniquad::conf::Conf,
}

impl WindowQuad
{
    pub fn new(
        conf: miniquad::conf::Conf,
    ) -> Self
    {
        return WindowQuad
        {
            conf: conf,
        };
    }

    pub fn run_loop<Handler>(self, handler: Handler)
    where
        Handler: WindowHandler + 'static
    {
        miniquad::start(miniquad::conf::Conf { ..self.conf }, move || {
            let (w, h) = miniquad::window::screen_size();
            let dpi = miniquad::window::dpi_scale();

            let renderer = GLRenderer::new();
            let renderer = RefCell::new(renderer);
            let renderer = Rc::new(renderer);

            let scaled_size = UVec2::new((w / dpi) as u32, (h / dpi) as u32);
            let mut helper = WindowHelper::new(
                renderer.clone(),
            );

            let mut handler = DrawingWindowHandler::new(handler, renderer);
            handler.on_start(
                &mut helper,
                WindowStartupInfo::new(
                    scaled_size,
                    miniquad::window::dpi_scale().into(),
                )
            );

            Box::new(Stage::new(handler, helper))
        });

    }

}

struct Stage<HandlerType>
    where HandlerType: WindowHandler + 'static
{
    handler: DrawingWindowHandler<HandlerType>,
    helper: WindowHelper,
}

impl<HandlerType: WindowHandler> Stage<HandlerType>
{
    fn new(
        handler: DrawingWindowHandler<HandlerType>,
        helper: WindowHelper,
        ) -> Self 
    {
        Stage 
        {
            handler: handler,
            helper: helper,
        }
    }

}

impl<HandlerType: WindowHandler> miniquad::EventHandler for Stage<HandlerType> {
    fn resize_event(&mut self, width: f32, height: f32) {
        let dpi = miniquad::window::dpi_scale();
        self.handler.on_resize(&mut self.helper, UVec2::new((width / dpi) as u32, (height / dpi) as u32));
    }

    fn raw_mouse_motion(&mut self, _x: f32, _y: f32) {
        // TODO ?
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let dpi = miniquad::window::dpi_scale();
        self.handler.on_mouse_move(&mut self.helper, x / dpi, y / dpi);
    }

    fn mouse_wheel_event(&mut self, _x: f32, _y: f32) {
        // TODO ?
    }

    fn mouse_button_down_event(&mut self, btn: MouseButton, x: f32, y: f32) {
        let dpi = miniquad::window::dpi_scale();
        self.handler.on_mouse_button_down(&mut self.helper, btn, x / dpi, y / dpi);
    }

    fn mouse_button_up_event(&mut self, btn: MouseButton, x: f32, y: f32) {
        let dpi = miniquad::window::dpi_scale();
        self.handler.on_mouse_button_up(&mut self.helper, btn, x / dpi, y / dpi);
    }

/*
    // don't impl this so the default will be used, which emulates mouse
    fn touch_event(&mut self, phase: miniquad::TouchPhase, id: u64, x: f32, y: f32) 
    {
    }
*/

    fn char_event(&mut self, character: char, modifiers: miniquad::KeyMods, repeat: bool) {
        self.handler.on_keyboard_char(&mut self.helper, character, modifiers, repeat);
    }

    fn key_down_event(
        &mut self, 
        keycode: miniquad::KeyCode, 
        modifiers: miniquad::KeyMods, 
        repeat: bool
        ) 
    {
        self.handler.on_key_down(&mut self.helper, keycode, modifiers, repeat);
    }

    fn key_up_event(&mut self, keycode: miniquad::KeyCode, modifiers: miniquad::KeyMods) {
        self.handler.on_key_up(&mut self.helper, keycode, modifiers);
    }

    fn update(&mut self) {
        self.helper.set_redraw_requested(false);
        self.handler.on_update(&mut self.helper);
    }

    fn draw(&mut self) {
        self.handler.on_draw(&mut self.helper);
    }

    fn window_restored_event(&mut self) {
    }

    fn window_minimized_event(&mut self) {
    }

    fn quit_requested_event(&mut self) {
    }
}

