use ribir_core::prelude::*;
use ribir_widgets::avatar::*;

use crate::md;

pub(super) fn init(classes: &mut Classes) {
  named_styles_impl!( base_container => {
    clamp: BoxClamp::fixed_size(md::SIZE_40),
    radius: md::RADIUS_20,
  });

  classes.insert(AVATAR_WIDGET_CONTAINER, base_container);
  classes.insert(AVATAR_WIDGET, empty_cls);
  classes.insert(
    AVATAR_LABEL_CONTAINER,
    class_multi_impl![
      style_class! { background: BuildCtx::color().into_container_color(BuildCtx::get()) },
      base_container
    ],
  );
  classes.insert(
    AVATAR_LABEL,
    style_class! {
      foreground: BuildCtx::color().on_this_container_color(BuildCtx::get()),
      text_style: TypographyTheme::of(BuildCtx::get()).title_medium.text.clone(),
      h_align: HAlign::Center,
      v_align: VAlign::Center,
    },
  );
}
