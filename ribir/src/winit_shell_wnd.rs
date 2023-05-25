use crate::backends::*;

use ribir_core::{
  prelude::*,
  window::{ShellWindow, WindowId},
};
use winit::{dpi::LogicalPosition, event_loop::EventLoopWindowTarget};
pub trait WinitBackend {
  fn new(window: &winit::window::Window) -> Self;

  fn on_resize(&mut self, size: DeviceSize);

  fn begin_frame(&mut self);

  fn draw_commands(
    &mut self,
    viewport: DeviceRect,
    commands: Vec<PaintCommand>,
    surface_color: Color,
  );

  fn end_frame(&mut self);
}

pub struct WinitShellWnd {
  pub(crate) winit_wnd: winit::window::Window,
  backend: Backend,
}

impl ShellWindow for WinitShellWnd {
  fn id(&self) -> WindowId { new_id(self.winit_wnd.id()) }

  fn device_pixel_ratio(&self) -> f32 { self.winit_wnd.scale_factor() as f32 }

  fn inner_size(&self) -> Size {
    let size = self
      .winit_wnd
      .inner_size()
      .to_logical(self.winit_wnd.scale_factor());
    Size::new(size.width, size.height)
  }

  fn outer_size(&self) -> Size {
    let size = self
      .winit_wnd
      .outer_size()
      .to_logical(self.winit_wnd.scale_factor());
    Size::new(size.width, size.height)
  }

  fn set_size(&mut self, size: Size) {
    if self.inner_size() != size {
      self
        .winit_wnd
        .set_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
      self.on_resize(size)
    }
  }

  #[inline]
  fn set_cursor(&mut self, cursor: CursorIcon) { self.winit_wnd.set_cursor_icon(cursor) }

  #[inline]
  fn set_title(&mut self, title: &str) { self.winit_wnd.set_title(title) }

  #[inline]
  fn set_ime_pos(&mut self, pos: Point) {
    let position: LogicalPosition<f32> = LogicalPosition::new(pos.x, pos.y);
    self.winit_wnd.set_ime_position(position);

    // tmp: winit set_ime_position fail when use sogou ime in window
    // platform, issue link: https://github.com/rust-windowing/winit/issues/2780
    #[cfg(windows)]
    unsafe {
      use winapi::um::winuser::SetCaretPos;
      let pos: winit::dpi::PhysicalPosition<i32> =
        position.to_physical(self.winit_wnd.scale_factor());
      SetCaretPos(pos.x, pos.y);
    }
  }

  #[inline]
  fn as_any(&self) -> &dyn std::any::Any { self }

  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }

  #[inline]
  fn begin_frame(&mut self) { self.backend.begin_frame() }

  #[inline]
  fn draw_commands(&mut self, viewport: Rect, mut commands: Vec<PaintCommand>, surface: Color) {
    commands.iter_mut().for_each(|c| match c {
      PaintCommand::ColorPath { path, .. }
      | PaintCommand::ImgPath { path, .. }
      | PaintCommand::Clip(path) => path.scale(self.winit_wnd.scale_factor() as f32),
      PaintCommand::PopClip => {}
    });

    let scale = self.winit_wnd.scale_factor() as f32;
    let viewport: DeviceRect = viewport
      .scale(scale, scale)
      .round_out()
      .to_i32()
      .cast_unit();

    self.backend.draw_commands(viewport, commands, surface);
  }

  #[inline]
  fn end_frame(&mut self) { self.backend.end_frame() }
}

pub(crate) fn new_id(id: winit::window::WindowId) -> WindowId {
  let id: u64 = id.into();
  id.into()
}

impl WinitShellWnd {
  pub(crate) fn new<T>(size: Option<Size>, window_target: &EventLoopWindowTarget<T>) -> Self {
    let mut winit_wnd = winit::window::WindowBuilder::new();
    if let Some(size) = size {
      winit_wnd = winit_wnd.with_inner_size(winit::dpi::LogicalSize::new(size.width, size.height));
    }

    let winit_wnd = winit_wnd.build(window_target).unwrap();
    set_ime_allowed(&winit_wnd);
    WinitShellWnd {
      backend: Backend::new(&winit_wnd),
      winit_wnd,
    }
  }

  pub fn on_resize(&mut self, size: Size) {
    let size: DeviceSize = (size * self.device_pixel_ratio()).to_i32().cast_unit();

    self.backend.on_resize(size);
  }
}

fn set_ime_allowed(wnd: &winit::window::Window) {
  wnd.set_ime_allowed(true);

  // tmp:  winit set_ime_position fail when use sogou ime in window
  // platform, issue link: https://github.com/rust-windowing/winit/issues/2780
  #[cfg(windows)]
  unsafe {
    use std::ptr::null_mut;
    use winapi::{
      shared::minwindef::HINSTANCE,
      shared::windef::HWND,
      um::winuser::{CreateCaret, LoadBitmapW},
    };
    use winit::platform::windows::WindowExtWindows;

    let hwnd = wnd.hwnd();
    let hinst: HINSTANCE = null_mut();
    let resource_id = 120;
    let resource_id_wstr: Vec<u16> = format!("#{}", resource_id).encode_utf16().collect();
    let hcaret = LoadBitmapW(hinst, resource_id_wstr.as_ptr());
    CreateCaret(hwnd as HWND, hcaret, 0, 0);
  }
}