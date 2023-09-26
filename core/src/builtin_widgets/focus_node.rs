use crate::{
  events::focus_mgr::{FocusHandle, FocusType},
  prelude::*,
};

#[derive(Default, Query, Declare2)]
pub struct FocusNode {
  /// Indicates that `widget` can be focused, and where it participates in
  /// sequential keyboard navigation (usually with the Tab key, hence the name.
  ///
  /// It accepts an integer as a value, with different results depending on the
  /// integer's value:
  /// - A negative value (usually -1) means that the widget is not reachable via
  ///   sequential keyboard navigation, but could be focused with API or
  ///   visually by clicking with the mouse.
  /// - Zero means that the element should be focusable in sequential keyboard
  ///   navigation, after any positive tab_index values and its order is defined
  ///   by the tree's source order.
  /// - A positive value means the element should be focusable in sequential
  ///   keyboard navigation, with its order defined by the value of the number.
  ///   That is, tab_index=4 is focused before tab_index=5 and tab_index=0, but
  ///   after tab_index=3. If multiple elements share the same positive
  ///   tab_index value, their order relative to each other follows their
  ///   position in the tree source. The maximum value for tab_index is 32767.
  ///   If not specified, it takes the default value 0.
  #[declare(default, builtin)]
  pub tab_index: i16,
  /// Indicates whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial
  /// focus.
  #[declare(default, builtin)]
  pub auto_focus: bool,
}

impl ComposeChild for FocusNode {
  type Child = Widget;
  fn compose_child(this: State<Self>, mut child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let tree = ctx!().tree.borrow();
      let node = child.id().assert_get(&tree.arena);
      let has_focus_node = node.contain_type::<FocusNode>();
      if !has_focus_node {
        let subject = node.query_most_outside(|l: &LifecycleListener| l.lifecycle_stream());
        drop(tree);
        let subject = if let Some(subject) = subject {
          subject
        } else {
          let listener = LifecycleListener::default();
          let subject = listener.lifecycle_stream();
          child = child.attach_data(listener, ctx!());
          subject
        };

        fn subscribe_fn(this: Reader<FocusNode>) -> impl FnMut(&'_ mut AllLifecycle) + 'static {
          move |e| match e {
            AllLifecycle::Mounted(e) => {
              let auto_focus = this.read().auto_focus;
              e.window().add_focus_node(e.id, auto_focus, FocusType::Node)
            }
            AllLifecycle::PerformedLayout(_) => {}
            AllLifecycle::Disposed(e) => e.window().remove_focus_node(e.id, FocusType::Node),
          }
        }
        let h = subject
          .subscribe(subscribe_fn(this.clone_reader()))
          .unsubscribe_when_dropped();
        child = child.attach_state_data(this, ctx!()).attach_anonymous_data(h, ctx!());
      }
      child
    }
  }
}

#[derive(Declare2, Query)]
pub struct RequestFocus {
  #[declare(default)]
  handle: Option<FocusHandle>,
}

impl ComposeChild for RequestFocus {
  type Child = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @$child {
        on_mounted: move |e| {
          let handle = e.window().focus_mgr.borrow().focus_handle(e.id);
          $this.silent().handle = Some(handle);
        }
      }
      .widget_build(ctx!())
      .attach_state_data(this, ctx!())
    }
  }
}
impl RequestFocus {
  pub fn request_focus(&self) {
    if let Some(h) = self.handle.as_ref() {
      h.request_focus();
    }
  }

  pub fn unfocus(&self) {
    if let Some(h) = self.handle.as_ref() {
      h.unfocus();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn dynamic_focus_node() {
    reset_test_env!();

    let widget = fn_widget! {
      @FocusNode {
        tab_index: 0i16, auto_focus: false,
        @FocusNode {
          tab_index: 0i16, auto_focus: false,
          @FocusNode {
            tab_index: 0i16, auto_focus: false,
            @MockBox {
              size: Size::default(),
            }
          }
        }
      }
    };

    let wnd = TestWindow::new(widget);
    let tree = wnd.widget_tree.borrow();
    let id = tree.root();
    let node = id.get(&tree.arena).unwrap();
    let mut cnt = 0;
    node.query_type_inside_first(|_: &FocusNode| {
      cnt += 1;
      true
    });

    assert!(cnt == 1);
  }
}
