use crate::{impl_query_self_only, prelude::*};

#[derive(Declare, Declare2)]
pub struct Visibility {
  #[declare(builtin)]
  pub visible: bool,
}

impl ComposeChild for Visibility {
  type Child = Widget;
  fn compose_child(mut this: State<Self>, child: Self::Child) -> Widget {
    fn_widget! {
      @FocusScope {
        skip_descendants: pipe!(!$this.get_visible()),
        can_focus: pipe!($this.get_visible()),
        @VisibilityRender {
          display: pipe!($this.get_visible()),
          @ { child }
        }
      }
    }
    .into()
  }
}

#[derive(SingleChild, Declare, Declare2, Clone)]
struct VisibilityRender {
  display: bool,
}

impl Render for VisibilityRender {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if self.display {
      ctx.assert_perform_single_child_layout(clamp)
    } else {
      ZERO_SIZE
    }
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    if !self.display {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest {
      hit: false,
      can_hit_child: self.display,
    }
  }
}

impl_query_self_only!(VisibilityRender);

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }

  #[inline]
  fn get_visible(&self) -> bool { self.visible }
}
