use crate::prelude::*;
use std::{cell::Cell, rc::Rc};
use winit::window::CursorIcon;

/// `Cursor` is an attribute to assign an `cursor` to a widget.

#[derive(Declare, Debug)]
pub struct Cursor {
  #[declare(custom_convert, builtin, default)]
  pub cursor: Rc<Cell<CursorIcon>>,
}

impl ComposeSingleChild for Cursor {
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this }
      ExprWidget {
        expr: child,
        on_pointer_move: move |e: &mut PointerEvent| {
          let mut ctx = e.context();
          if e.point_type == PointerType::Mouse
            && e.buttons == MouseButtons::empty()
            && ctx.updated_cursor().is_none()
          {
            ctx.set_cursor(this.cursor.get());
          }
        },
      }
    }
  }
}

pub trait IntoCursorIcon {
  fn into_cursor_icon(self) -> Rc<Cell<CursorIcon>>;
}

impl IntoCursorIcon for Rc<Cell<CursorIcon>> {
  #[inline]
  fn into_cursor_icon(self) -> Rc<Cell<CursorIcon>> { self }
}

impl IntoCursorIcon for CursorIcon {
  #[inline]
  fn into_cursor_icon(self) -> Rc<Cell<CursorIcon>> { Rc::new(Cell::new(self)) }
}

impl CursorBuilder {
  #[inline]
  pub fn cursor_convert<C: IntoCursorIcon>(icon: C) -> Rc<Cell<CursorIcon>> {
    icon.into_cursor_icon()
  }
}

impl Cursor {
  #[inline]
  pub fn icon(&self) -> CursorIcon { self.cursor.get() }

  #[inline]
  pub fn set_icon(&self, icon: CursorIcon) { self.cursor.set(icon) }

  #[inline]
  pub fn new_icon(icon: CursorIcon) -> Rc<Cell<CursorIcon>> { Rc::new(Cell::new(icon)) }
}

impl Default for Cursor {
  #[inline]
  fn default() -> Self {
    Cursor {
      cursor: Rc::new(Cell::new(CursorIcon::Default)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use winit::event::{DeviceId, WindowEvent};

  #[test]
  fn tree_down_up() {
    let row_tree = widget! {
      SizedBox {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        cursor: CursorIcon::AllScroll,
        Row{
          v_align: CrossAxisAlign::Start,
          h_align: MainAxisAlign::Start,
          SizedBox {
            size: Size::new(200., 200.),
            cursor: CursorIcon::Hand,
            Row {
              v_align: CrossAxisAlign::Start,
              h_align: MainAxisAlign::Start,
              SizedBox {
                size:  Size::new(100., 100.),
                cursor: CursorIcon::Help,
              }
            }
          }
        }
      }
    };

    let mut wnd = Window::without_render(row_tree, Size::new(400., 400.));

    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    let ctx = &mut wnd.context;
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Help));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Hand));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (201f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::AllScroll));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (101f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Hand));

    let device_id = unsafe { DeviceId::dummy() };
    wnd.dispatcher.dispatch(
      WindowEvent::CursorMoved {
        device_id,
        position: (1f64, 1.).into(),
        modifiers: ModifiersState::default(),
      },
      ctx,
      1.,
    );
    assert_eq!(ctx.cursor.take(), Some(CursorIcon::Help));
  }
}
