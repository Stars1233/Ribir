//! Theme use to share visual config or style compose logic. It can be defined
//! to app-wide or particular part of the application.

use std::collections::HashMap;

pub use ribir_algo::{CowArc, Resource};
use smallvec::SmallVec;

use crate::prelude::*;

mod palette;
pub use palette::*;
mod icon_theme;
pub use icon_theme::*;
mod typography_theme;
pub use typography_theme::*;
mod transition_theme;
pub use transition_theme::*;
mod compose_decorators;
pub use compose_decorators::*;
mod custom_styles;
pub use custom_styles::*;
pub use ribir_painter::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brightness {
  Dark,
  Light,
}

/// A `Theme` widget is used to share design configurations among its
/// descendants.
///
/// This includes palettes, font styles, animation transitions, and icons. An
/// app theme is always present, but you can also use a different
/// theme for parts of your sub-tree. You can customize parts of the theme using
/// `Palette`, `TypographyTheme`, and `IconTheme`.
///
/// # Examples
///
/// Every descendant widget of the theme can query it or its parts.
///
/// ```no_run
/// use ribir::prelude::*;
///
/// let w = fn_widget! {
///   @Text {
///     on_tap: |e| {
///       // Query the `Palette` of the application theme.
///       let mut p = Palette::write_of(e);
///        if p.brightness == Brightness::Light {
///           p.brightness = Brightness::Dark;
///        } else {
///           p.brightness = Brightness::Light;
///        }
///     },
///     text : "Click me!"
///   }
/// };
///
/// App::run(w);
/// ```
///
/// You can provide a theme for a widget:
///
/// ```
/// use ribir::prelude::*;
///
/// let w = Theme::default().with_child(fn_widget! {
///   // Feel free to use a different theme here.
///   Void
/// });
/// ```
///
/// # Todo
///
/// Simplify the theme by eliminating the need for `TransitionTheme`,
// `CustomStyles`, and `ComposeDecorators` if we can find a more elegant way to
// handle widget theme styles.
pub struct Theme {
  pub palette: Palette,
  pub typography_theme: TypographyTheme,
  pub classes: Classes,
  pub icon_theme: IconTheme,
  pub transitions_theme: TransitionTheme,
  pub compose_decorators: ComposeDecorators,
  pub custom_styles: CustomStyles,
  pub font_bytes: Vec<Vec<u8>>,
  pub font_files: Vec<String>,
  /// This font is used for icons to display text as icons through font
  /// ligatures. It is crucial to ensure that this font is included in either
  /// `font_bytes` or `font_files`.
  ///
  /// Theme makers may not know which icons the application will utilize, making
  /// it challenging to provide a default icon font. Additionally, offering a
  /// vast selection of icons in a single font file can result in a large file
  /// size, which is not ideal for web platforms. Therefore, this configuration
  /// allows the application developer to supply the font file. Certainly, the
  /// icon also works with `SVG` and [`named_svgs`](super::named_svgs).
  pub icon_font: FontFace,
}

impl Theme {
  /// Retrieve the nearest `Theme` from the context among its ancestors
  pub fn of(ctx: &impl ProviderCtx) -> QueryRef<Theme> {
    // At least one application theme exists
    Provider::of(ctx).unwrap()
  }

  /// Retrieve the nearest `Theme` from the context among its ancestors and
  /// return a write reference to the theme.
  pub fn write_of(ctx: &impl ProviderCtx) -> WriteRef<Theme> {
    // At least one application theme exists
    Provider::write_of(ctx).unwrap()
  }

  /// Loads the fonts specified in the theme configuration.
  fn load_fonts(&mut self) {
    let mut font_db = AppCtx::font_db().borrow_mut();
    let Theme { font_bytes, font_files, .. } = self;
    font_bytes
      .iter()
      .for_each(|data| font_db.load_from_bytes(data.clone()));

    font_files.iter().for_each(|path| {
      let _ = font_db.load_font_file(path);
    });
  }
}

impl ComposeChild<'static> for Theme {
  /// The child should be a `GenWidget` so that when the theme is modified, we
  /// can regenerate its sub-tree.
  type Child = GenWidget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'static> {
    use crate::prelude::*;

    this.silent().load_fonts();
    let w = this.clone_watcher();
    let theme = ThemeQuerier(this.clone_writer());

    Provider::new(Box::new(theme))
      .with_child(fn_widget! {
        pipe!($w;).map(move |_| child.gen_widget())
      })
      .into_widget()
  }
}

struct ThemeQuerier<T: StateWriter<Value = Theme>>(T);

impl<T: StateWriter<Value = Theme>> Query for ThemeQuerier<T> {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    // The value of the writer and the writer itself cannot be queried
    // at the same time.
    if let Some(h) = self.query(query_id) {
      out.push(h)
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    if let Some(h) = self.query_write(query_id) {
      out.push(h)
    }
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    ReadRef::filter_map(self.0.read(), |v: &Theme| {
      let w: Option<&dyn QueryAny> = if &QueryId::of::<Theme>() == query_id {
        Some(v)
      } else if &QueryId::of::<Palette>() == query_id {
        Some(&v.palette)
      } else if &QueryId::of::<TypographyTheme>() == query_id {
        Some(&v.typography_theme)
      } else if &QueryId::of::<TextStyle>() == query_id {
        Some(&v.typography_theme.body_medium.text)
      } else if &QueryId::of::<Classes>() == query_id {
        Some(&v.classes)
      } else if &QueryId::of::<IconTheme>() == query_id {
        Some(&v.icon_theme)
      } else if &QueryId::of::<TransitionTheme>() == query_id {
        Some(&v.transitions_theme)
      } else if &QueryId::of::<ComposeDecorators>() == query_id {
        Some(&v.compose_decorators)
      } else if &QueryId::of::<CustomStyles>() == query_id {
        Some(&v.custom_styles)
      } else {
        None
      };
      w.map(PartData::from_ref)
    })
    .ok()
    .map(QueryHandle::from_read_ref)
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    WriteRef::filter_map(self.0.write(), |v: &mut Theme| {
      let w: Option<&mut dyn QueryAny> = if &QueryId::of::<Theme>() == query_id {
        Some(v)
      } else if &QueryId::of::<Palette>() == query_id {
        Some(&mut v.palette)
      } else if &QueryId::of::<TypographyTheme>() == query_id {
        Some(&mut v.typography_theme)
      } else if &QueryId::of::<IconTheme>() == query_id {
        Some(&mut v.icon_theme)
      } else if &QueryId::of::<TransitionTheme>() == query_id {
        Some(&mut v.transitions_theme)
      } else if &QueryId::of::<ComposeDecorators>() == query_id {
        Some(&mut v.compose_decorators)
      } else if &QueryId::of::<CustomStyles>() == query_id {
        Some(&mut v.custom_styles)
      } else {
        None
      };
      w.map(PartData::from_ref_mut)
    })
    .ok()
    .map(QueryHandle::from_write_ref)
  }

  fn query_match(
    &self, ids: &[QueryId], filter: &dyn Fn(&QueryId, &QueryHandle) -> bool,
  ) -> Option<(QueryId, QueryHandle)> {
    ids.iter().find_map(|id| {
      self
        .query(id)
        .filter(|h| filter(id, h))
        .map(|h| (*id, h))
    })
  }
  fn queryable(&self) -> bool { true }
}

impl Default for Theme {
  fn default() -> Self {
    let icon_size = IconSize {
      tiny: Size::new(18., 18.),
      small: Size::new(24., 24.),
      medium: Size::new(36., 36.),
      large: Size::new(48., 48.),
      huge: Size::new(64., 64.),
    };

    let icon_theme = IconTheme::new(icon_size);

    Theme {
      palette: Palette::default(),
      typography_theme: typography_theme(),
      icon_theme,
      classes: <_>::default(),
      transitions_theme: Default::default(),
      compose_decorators: Default::default(),
      custom_styles: Default::default(),
      font_bytes: vec![],
      font_files: vec![],
      icon_font: Default::default(),
    }
  }
}

fn typography_theme() -> TypographyTheme {
  fn text_theme(line_height: f32, font_size: f32, letter_space: f32) -> TextTheme {
    let font_face = FontFace {
      families: Box::new([FontFamily::Name(std::borrow::Cow::Borrowed("Lato")), FontFamily::Serif]),
      weight: FontWeight::NORMAL,
      ..<_>::default()
    };
    let overflow = TextOverflow::Clip;
    TextTheme {
      text: TextStyle { line_height, font_size, letter_space, font_face, overflow },
      decoration: TextDecorationStyle {
        decoration: TextDecoration::NONE,
        decoration_color: Color::BLACK.with_alpha(0.87).into(),
      },
    }
  }

  TypographyTheme {
    display_large: text_theme(64., 57., 0.),
    display_medium: text_theme(52., 45., 0.),
    display_small: text_theme(44., 36., 0.),
    headline_large: text_theme(40., 32., 0.),
    headline_medium: text_theme(36., 28., 0.),
    headline_small: text_theme(32., 24., 0.),
    title_large: text_theme(28., 22., 0.),
    title_medium: text_theme(24., 16., 0.15),
    title_small: text_theme(20., 14., 0.1),
    label_large: text_theme(20., 14., 0.1),
    label_medium: text_theme(16., 12., 0.5),
    label_small: text_theme(16., 11., 0.5),
    body_large: text_theme(24., 16., 0.5),
    body_medium: text_theme(20., 14., 0.25),
    body_small: text_theme(16., 12., 0.4),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn themes() {
    reset_test_env!();

    let (watcher, writer) = split_value(vec![]);

    let w = fn_widget! {
      let writer = writer.clone_writer();
      let mut theme = Theme::default();
      theme.palette.brightness = Brightness::Light;
      theme.with_child(fn_widget! {
        $writer.write().push(Palette::of(BuildCtx::get()).brightness);
        let writer = writer.clone_writer();
        Palette { brightness: Brightness::Dark, ..Default::default() }
          .with_child(fn_widget!{
            $writer.write().push(Palette::of(BuildCtx::get()).brightness);
            let writer = writer.clone_writer();
            Palette { brightness: Brightness::Light, ..Default::default() }
              .with_child(fn_widget!{
                $writer.write().push(Palette::of(BuildCtx::get()).brightness);
                Void
            })
        })
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*watcher.read(), [Brightness::Light, Brightness::Dark, Brightness::Light]);
  }
}
