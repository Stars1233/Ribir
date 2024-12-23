use ribir_core::prelude::*;
use ribir_widgets::prelude::*;

use crate::md;

const THUMB_MIN_SIZE: f32 = 12.;

pub(super) fn init(classes: &mut Classes) {
  // In this theme, the scrollbar is positioned floating on the scroll content
  // widget, so there is no need for any additional padding or adjustments to the
  // content widget.
  //
  // However, we also provide an empty class implementation to prevent it from
  // inheriting the ancestor class implementation of `SCROLL_CLIENT_AREA`.
  classes.insert(SCROLL_CLIENT_AREA, empty_cls);

  classes.insert(H_SCROLL_THUMB, style_class! {
    background: BuildCtx::get().variant_color(),
    border_radius: md::RADIUS_4,
    margin: EdgeInsets::vertical(1.),
    clamp: BoxClamp::min_width(THUMB_MIN_SIZE).with_fixed_height(md::THICKNESS_8)
  });
  classes.insert(V_SCROLL_THUMB, style_class! {
    background: BuildCtx::get().variant_color(),
    border_radius: md::RADIUS_4,
    margin: EdgeInsets::horizontal(1.),
    clamp: BoxClamp::min_height(THUMB_MIN_SIZE).with_fixed_width(md::THICKNESS_8)
  });

  classes
    .insert(H_SCROLL_TRACK, multi_class![base_track, style_class! { v_align: VAlign::Bottom }]);
  classes.insert(V_SCROLL_TRACK, multi_class![base_track, style_class! { h_align: HAlign::Right }]);
}

fn base_track(w: Widget) -> Widget {
  fn_widget! {
    let scroll = &*Provider::of::<Stateful<ScrollableWidget>>(BuildCtx::get()).unwrap();
    let mut w = FatObj::new(w).opacity(0.);

    // Show the scrollbar when scrolling.
    let mut fade: Option<TaskHandle<_>> = None;
    let u = watch!(($scroll).get_scroll_pos())
      .distinct_until_changed()
      .subscribe(move |_| {
        $w.write().opacity = 1.;
        if let Some(f) = fade.take() {
          f.unsubscribe();
        }
        let u = observable::timer((), Duration::from_secs(3), AppCtx::scheduler())
          .filter(move |_| !$w.is_hover())
          .subscribe(move |_| $w.write().opacity = 0.);
        fade = Some(u);
      });

    let trans = EasingTransition {
      easing: md::easing::STANDARD,
      duration: md::easing::duration::MEDIUM2
    };
    // Smoothly fade in and out the scrollbar.
    part_writer!(&mut w.opacity).transition(trans.clone());

    let mut w = @ $w {
      background: {
        let color = BuildCtx::get().variant_container_color();
        pipe!(if $w.is_hover() { color } else { color.with_alpha(0.)})
      },
      on_disposed: move |_| u.unsubscribe(),
    };
    // Smoothly display the background.
    part_writer!(&mut w.background).transition(trans);

    w
  }
  .into_widget()
}
