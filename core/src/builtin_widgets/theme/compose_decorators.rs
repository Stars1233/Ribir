use std::any::type_name;

use crate::prelude::*;

type ComposeDecoratorFn = dyn for<'r> Fn(Box<dyn Any>, Widget<'r>, &BuildCtx) -> Widget<'r>;
/// Compose style is a compose child widget to decoration its child.
#[derive(Default)]
pub struct ComposeDecorators {
  pub(crate) styles: ahash::HashMap<TypeId, Box<ComposeDecoratorFn>>,
}

pub trait ComposeDecorator: Sized {
  fn compose_decorator(this: State<Self>, host: Widget) -> Widget;
}

impl ComposeDecorators {
  #[inline]
  pub fn override_compose_decorator<W: ComposeDecorator + 'static>(
    &mut self,
    compose_decorator: impl for<'r> Fn(State<W>, Widget<'r>, &BuildCtx) -> Widget<'r> + 'static,
  ) {
    self.styles.insert(
      TypeId::of::<W>(),
      Box::new(move |this: Box<dyn Any>, host: Widget, ctx: &BuildCtx| {
        let this = this.downcast().unwrap_or_else(|_| {
          panic!("Caller should guarantee the boxed type is Stateful<{}>.", type_name::<W>())
        });

        compose_decorator(*this, host, ctx)
      }),
    );
  }
}

#[cfg(test)]
mod tests {

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[test]
  fn compose_decorator_smoke() {
    reset_test_env!();

    let mut theme = Theme::default();

    #[derive(Declare)]
    struct Size100Style;

    impl ComposeDecorator for Size100Style {
      fn compose_decorator(_: State<Self>, host: Widget) -> Widget { host }
    }
    theme
      .compose_decorators
      .override_compose_decorator::<Size100Style>(|_, host, _| {
        fn_widget! {
          @MockBox {
            size: Size::new(100., 100.),
            @ { host }
          }
        }
        .into_widget()
      });

    let w = fn_widget! {
      @Size100Style { @MockBox {
        size: Size::zero(),
      }}
    };

    AppCtx::set_app_theme(theme);
    let mut wnd = TestWindow::new_with_size(w, Size::new(500., 500.));
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(100., 100.));
  }
}
