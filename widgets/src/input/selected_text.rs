use ribir_core::prelude::*;

use crate::layout::Stack;

#[derive(Declare)]
pub(crate) struct SelectedHighLight {
  pub(crate) rects: Vec<Rect>,
}

#[derive(Clone, PartialEq)]
pub struct SelectedHighLightStyle {
  pub brush: Brush,
}
impl CustomStyle for SelectedHighLightStyle {
  fn default_style(_: &impl ProviderCtx) -> Self {
    SelectedHighLightStyle { brush: Color::from_rgb(181, 215, 254).into() }
  }
}

impl Compose for SelectedHighLight {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      let color = SelectedHighLightStyle::of(BuildCtx::get()).brush;
      @Stack {
        @ { pipe!{
          let color = color.clone();
          $this.rects.clone().into_iter().map(move |rc| {
            @Container {
              background: color.clone(),
              anchor: Anchor::from_point(rc.origin),
              size: rc.size,
            }
          })
        }}
      }
    }
    .into_widget()
  }
}
