use holiday::{prelude::*, widget::column, widget::row};

#[derive(Debug)]
struct Todos {}

impl CombinationWidget for Todos {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    column(vec![
      row(
        (0..15)
          .map(|i| {
            let (stateful, mut state_modify) = Text(format!("FirstRow {} ", i)).into_stateful(ctx);
            stateful.on_pointer_down(move |_| state_modify.0 = state_modify.0.clone() + "1")
          })
          .collect(),
      )
      .box_it(),
      row(
        (0..1)
          .map(|i| Text(format!("SecondRow {} ", i)).box_it())
          .collect(),
      )
      .box_it(),
      row(
        (0..3)
          .map(|i| Text(format!("ThirdRow {} ", i)).box_it())
          .collect(),
      )
      .box_it(),
    ])
    .box_it()
  }
}
fn main() {
  env_logger::init();
  let todo = Todos {};
  Application::new().run(todo.box_it());
}
