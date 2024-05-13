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

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use crate::dimen::{IVec2, UVec2, Vec2, Vector2};
use crate::error::{BacktraceError, ErrorMessage};
use crate::window::{
    DrawingWindowHandler,
    EventLoopSendError,
    ModifiersState,
    MouseButton,
    MouseScrollDistance,
    UserEventSender,
    WindowCreationError,
    WindowCreationMode,
    WindowCreationOptions,
    WindowEventLoopAction,
    WindowFullscreenMode,
    WindowHandler,
    WindowHelper,
    WindowPosition,
    WindowSize,
    WindowStartupInfo
};
use crate::GLRenderer;
use crate::Color;

pub(crate) struct WindowHelperQuad<UserEventType: 'static>
{
    renderer: Rc<RefCell<GLRenderer>>,
    event_proxy: Sender<UserEventType>,
    redraw_requested: Cell<bool>,
    terminate_requested: bool,
    physical_size: UVec2,
    is_mouse_grabbed: Cell<bool>,
    tmp: std::marker::PhantomData<UserEventType>
}

impl<UserEventType> WindowHelperQuad<UserEventType>
{
    #[inline]
    pub fn new(
        initial_physical_size: UVec2,
        ep: Sender<UserEventType>,
        renderer: Rc<RefCell<GLRenderer>>,
    ) -> Self
    {
        WindowHelperQuad {
            renderer: renderer,
            event_proxy: ep,
            redraw_requested: Cell::new(false),
            terminate_requested: false,
            physical_size: initial_physical_size,
            is_mouse_grabbed: Cell::new(false),
            tmp: std::marker::PhantomData {},
        }
    }

    #[must_use]
    pub fn create_font_from_bytes(&self, bytes: &[u8]) -> Result<crate::text::Font,i32>
    {
        self.renderer.borrow_mut().create_font_from_bytes(bytes)
    }

    #[inline]
    #[must_use]
    pub fn is_redraw_requested(&self) -> bool
    {
        self.redraw_requested.get()
    }

    #[inline]
    pub fn set_redraw_requested(&mut self, redraw_requested: bool)
    {
        self.redraw_requested.set(redraw_requested);
    }

    #[inline]
    pub fn get_event_loop_action(&self) -> WindowEventLoopAction
    {
        match self.terminate_requested {
            true => WindowEventLoopAction::Exit,
            false => WindowEventLoopAction::Continue
        }
    }

    pub fn terminate_loop(&mut self)
    {
        self.terminate_requested = true;
    }

    pub fn set_icon_from_rgba_pixels(
        &self,
        data: Vec<u8>,
        size: UVec2
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {

        Ok(())
    }

    pub fn set_cursor_visible(&self, visible: bool)
    {
    }

    pub fn set_cursor_grab(
        &self,
        grabbed: bool
    ) -> Result<(), BacktraceError<ErrorMessage>>
    {
        panic!();
    }

    pub fn set_resizable(&self, resizable: bool)
    {
    }

    #[inline]
    pub fn request_redraw(&self)
    {
        #[cfg(target_arch = "wasm32")]
        unsafe {crate::request_animation_frame(); }

        self.redraw_requested.set(true);
    }

    pub fn set_title(&self, title: &str)
    {
    }

    pub fn set_fullscreen_mode(&self, mode: WindowFullscreenMode)
    {
    }

    pub fn set_size_pixels<S: Into<UVec2>>(&self, size: S)
    {
    }

    pub fn get_size_pixels(&self) -> UVec2
    {
        let (w, h) = miniquad::window::screen_size();
        let dpi = miniquad::window::dpi_scale();
        return UVec2::new((w / dpi) as u32, (h / dpi) as u32);
    }

    pub fn set_size_scaled_pixels<S: Into<Vec2>>(&self, size: S)
    {
    }

    pub fn set_position_pixels<P: Into<IVec2>>(&self, position: P)
    {
    }

    pub fn set_position_scaled_pixels<P: Into<Vec2>>(&self, position: P)
    {
    }

    #[inline]
    #[must_use]
    pub fn get_scale_factor(&self) -> f64
    {
        miniquad::window::dpi_scale().into()
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        UserEventSender::new(UserEventSenderQuad::new(self.event_proxy.clone()))
    }
}

pub(crate) struct WindowQuad<UserEventType: 'static>
{
    title: String,
    options: WindowCreationOptions,
    tmp: std::marker::PhantomData<UserEventType>,
}

impl<UserEventType: 'static> WindowQuad<UserEventType>
{
    pub fn new(
        title: &str,
        options: WindowCreationOptions
    ) -> Result<WindowQuad<UserEventType>, BacktraceError<WindowCreationError>>
    {
        return Ok(WindowQuad
        {
            title: title.to_string(),
            options: options,
            tmp: std::marker::PhantomData {},
        });
    }

    pub fn create_user_event_sender(&self) -> UserEventSender<UserEventType>
    {
        todo!();
    }

    pub fn get_inner_size_pixels(&self) -> UVec2
    {
        let (w, h) = miniquad::window::screen_size();
        return UVec2::new(w as u32, h as u32);
    }

    pub fn run_loop<Handler>(self, handler: Handler)
    where
        Handler: WindowHandler<UserEventType> + 'static
    {
        // TODO get initial width and height
        let config = 
            miniquad::conf::Conf {
                window_width: 1200,
                window_height: 1200,
                window_title: self.title.to_string(),
                high_dpi: true,
                ..Default::default()
            };

        miniquad::start(miniquad::conf::Conf { ..config }, move || {
            let (tx, rx): (Sender<UserEventType>, Receiver<UserEventType>) = mpsc::channel();
            let (w, h) = miniquad::window::screen_size();
            let initial_viewport_size_pixels = UVec2::new(w as u32, h as u32);
            let dpi = miniquad::window::dpi_scale();

            let renderer = GLRenderer::new_for_quad();
            let renderer = RefCell::new(renderer);
            let renderer = Rc::new(renderer);

            let scaled_size = UVec2::new((w / dpi) as u32, (h / dpi) as u32);
            let mut helper = WindowHelper::new(WindowHelperQuad::new(
                initial_viewport_size_pixels, // TODO is this right?
                tx.clone(),
                renderer.clone(),
            ));

            let mut handler = DrawingWindowHandler::new(handler, renderer);
            handler.on_start(
                &mut helper,
                WindowStartupInfo::new(
                    scaled_size,
                    miniquad::window::dpi_scale().into(),
                )
            );

            Box::new(Stage::new(handler, helper, rx))
        });

        //panic!("reached end of the event loop?"); // TODO should not get here
    }

}

impl From<miniquad::KeyMods> for ModifiersState
{
    fn from(state: miniquad::KeyMods) -> Self
    {
        ModifiersState {
            ctrl: state.ctrl,
            alt: state.alt,
            shift: state.shift,
            logo: state.logo
        }
    }
}

/*
impl From<PhysicalSize<u32>> for UVec2
{
    fn from(value: PhysicalSize<u32>) -> Self
    {
        Self::new(value.width, value.height)
    }
}
*/

pub(crate) enum UserEventQuad<UserEventType: 'static>
{
    MouseGrabStatusChanged(bool),
    FullscreenStatusChanged(bool),
    UserEvent(UserEventType)
}

pub struct UserEventSenderQuad<UserEventType: 'static>
{
    event_proxy: Sender<UserEventType>,
}

impl<UserEventType> Clone for UserEventSenderQuad<UserEventType>
{
    fn clone(&self) -> Self
    {
        UserEventSenderQuad {
            event_proxy: self.event_proxy.clone()
        }
    }
}

impl<UserEventType> UserEventSenderQuad<UserEventType>
{
    fn new(ep: Sender<UserEventType>) -> Self
    {
        Self { event_proxy: ep }
    }

    pub fn send_event(&self, event: UserEventType) -> Result<(), EventLoopSendError>
    {
        self.event_proxy.send(event);
        Ok(())
    }
}

struct Stage<UserEventType, HandlerType>
    where UserEventType: 'static,
        HandlerType: WindowHandler<UserEventType> + 'static
{
    handler: DrawingWindowHandler<UserEventType, HandlerType>,
    helper: WindowHelper<UserEventType>,
    user_events: Receiver<UserEventType>,
}

impl<UserEventType, HandlerType: WindowHandler<UserEventType>> Stage<UserEventType, HandlerType>
{
    const DEFAULT_BG_COLOR: Color = Color::BLACK;

    fn new(
        handler: DrawingWindowHandler<UserEventType, HandlerType>,
        helper: WindowHelper<UserEventType>,
        user_events: Receiver<UserEventType>,
        ) -> Self 
    {
        Stage 
        {
            handler: handler,
            helper: helper,
            user_events: user_events,
        }
    }

}

impl<UserEventType, HandlerType: WindowHandler<UserEventType>> miniquad::EventHandler for Stage<UserEventType, HandlerType> {
    fn resize_event(&mut self, width: f32, height: f32) {
        let dpi = miniquad::window::dpi_scale();
        self.handler.on_resize(&mut self.helper, UVec2::new((width / dpi) as u32, (height / dpi) as u32));
    }

    fn raw_mouse_motion(&mut self, x: f32, y: f32) {
        //self.handler.on_mouse_move(&mut self.helper, Vec2::new(x, y));
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let dpi = miniquad::window::dpi_scale();
        self.handler.on_mouse_move(&mut self.helper, Vec2::new(x / dpi, y / dpi));
    }

    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
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
    fn touch_event(&mut self, phase: miniquad::TouchPhase, id: u64, x: f32, y: f32) {
    }
*/

    fn char_event(&mut self, character: char, modifiers: miniquad::KeyMods, repeat: bool) {
        self.handler.on_keyboard_char(&mut self.helper, character);
    }

    fn key_down_event(&mut self, keycode: miniquad::KeyCode, modifiers: miniquad::KeyMods, repeat: bool) {
        // TODO why is the keycode in the window handler an option?
        self.handler.on_key_down(&mut self.helper, Some(keycode), 0); // TODO
    }

    fn key_up_event(&mut self, keycode: miniquad::KeyCode, modifiers: miniquad::KeyMods) {
    }

    fn update(&mut self) {
        self.handler.on_update(&mut self.helper);
        match self.user_events.try_recv()
        {
            Ok(x) => 
            {
                self.handler.on_user_event(&mut self.helper, x);
            },
            Err(_) =>
            {
            },
        }
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

