use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

const THICKNESS: f32 = 1.;

named_style_class!(horizontal_base => {
  clamp: BoxClamp::fixed_size(Size::new(f32::INFINITY, THICKNESS)),
  background: Palette::of(BuildCtx::get()).outline_variant(),
});

named_style_class!(vertical_base => {
  clamp: BoxClamp::fixed_size(Size::new(THICKNESS, f32::INFINITY)),
  background: Palette::of(BuildCtx::get()).outline_variant(),
});
pub(super) fn init(classes: &mut Classes) {
  classes.insert(HORIZONTAL_DIVIDER, horizontal_base);

  classes.insert(
    HORIZONTAL_DIVIDER_INDENT_START,
    multi_class! {
      horizontal_base,
      style_class!{ margin: md::EDGES_LEFT_16 }
    },
  );

  classes.insert(
    HORIZONTAL_DIVIDER_INDENT_END,
    multi_class! {
      horizontal_base,
      style_class!{ margin: md::EDGES_RIGHT_16 }
    },
  );

  classes.insert(
    HORIZONTAL_DIVIDER_INDENT_BOTH,
    multi_class! {
      horizontal_base,
      style_class!{ margin: md::EDGES_HOR_16 }
    },
  );

  classes.insert(VERTICAL_DIVIDER, vertical_base);

  classes.insert(
    VERTICAL_DIVIDER_INDENT_START,
    multi_class! {
      vertical_base,
      style_class! { margin: md::EDGES_TOP_8 }
    },
  );

  classes.insert(
    VERTICAL_DIVIDER_INDENT_END,
    multi_class! {
      vertical_base,
      style_class! { margin: md::EDGES_BOTTOM_8}
    },
  );

  classes.insert(
    VERTICAL_DIVIDER_INDENT_BOTH,
    multi_class! {
      vertical_base,
      style_class! { margin: md::EDGES_VER_8 }
    },
  );
}
