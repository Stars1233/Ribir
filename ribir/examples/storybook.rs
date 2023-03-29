use ribir::prelude::{svgs, *};

const WINDOW_SIZE: f32 = 800.;

fn main() {
  let widgets = widget! {
    init ctx => {
      let title_style = TypographyTheme::of(ctx).display_large.text.clone();
      let foreground = Palette::of(ctx).on_surface_variant().into();
      let secondary: Brush = Palette::of(ctx).secondary().into();
      let tertiary: Brush = Palette::of(ctx).tertiary().into();
    }
    ConstrainedBox {
      clamp: BoxClamp { min: Size::new(WINDOW_SIZE, WINDOW_SIZE), max: Size::new(WINDOW_SIZE, WINDOW_SIZE) },
      Tabs {
        Tab {
          TabItem {
            svgs::HOME
            Label::new("Button")
          }
          TabPane {
            Column {
              margin: EdgeInsets::all(20.),
              Text::new("Button", &foreground, title_style.clone())
              Row {
                margin: EdgeInsets::only_top(20.),
                FilledButton { svgs::ADD }
                SizedBox { size: Size::new(20., 0.) }
                FilledButton { Label::new("Filled button") }
                SizedBox { size: Size::new(20., 0.) }
                FilledButton {
                  color: secondary.clone(),
                  svgs::ADD
                  Label::new("Filled button")
                }
                SizedBox { size: Size::new(20., 0.) }
                FilledButton {
                  color: tertiary.clone(),
                  svgs::ADD
                  Label::new("Filled button")
                }
              }
              Row {
                margin: EdgeInsets::only_top(20.),
                OutlinedButton { svgs::ADD }
                SizedBox { size: Size::new(20., 0.) }
                OutlinedButton { Label::new("Outlined button") }
                SizedBox { size: Size::new(20., 0.) }
                OutlinedButton {
                  color: secondary.clone(),
                  svgs::ADD
                  Label::new("Outlined button")
                }
                SizedBox { size: Size::new(20., 0.) }
                OutlinedButton {
                  color: tertiary.clone(),
                  svgs::ADD
                  Label::new("Outlined button")
                }
              }
              Row {
                margin: EdgeInsets::only_top(20.),
                Button { svgs::ADD }
                SizedBox { size: Size::new(20., 0.) }
                Button { Label::new("Raw button") }
                SizedBox { size: Size::new(20., 0.) }
                Button {
                  color: secondary.clone(),
                  Label::new("Raw button")
                }
                SizedBox { size: Size::new(20., 0.) }
                Button {
                  color: tertiary.clone(),
                  Label::new("Raw button")
                }
              }
              Row {
                margin: EdgeInsets::only_top(20.),
                FabButton { svgs::ADD }
                SizedBox { size: Size::new(20., 0.) }
                FabButton { Label::new("Fab button") }
                SizedBox { size: Size::new(20., 0.) }
                FabButton {
                  color: secondary,
                  svgs::ADD
                  Label::new("Fab button")
                }
                SizedBox { size: Size::new(20., 0.) }
                FabButton {
                  color: tertiary,
                  svgs::ADD
                  Label::new("Fab button")
                }
              }
            }
          }
        }
        Tab {
          TabItem {
            svgs::MENU
            Label::new("Lists")
          }
          TabPane {
            Column {
              margin: EdgeInsets::all(20.),
              Text::new("Lists", &foreground, title_style.clone())
            }
          }
        }
      }
    }
  };
  let app = Application::new(material::purple::light());
  let wnd = Window::builder(widgets)
    .with_inner_size(Size::new(WINDOW_SIZE, WINDOW_SIZE))
    .with_title("StoryBook")
    .build(&app);
  app::run_with_window(app, wnd);
}