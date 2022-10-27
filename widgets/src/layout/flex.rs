use super::{Direction, Expanded};
use ribir_core::{impl_query_self_only, prelude::*};

/// How the children should be placed along the main axis in a flex layout.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum JustifyContent {
  /// Place the children as close to the start of the main axis as possible.
  #[default]
  Start,
  ///Place the children as close to the middle of the main axis as possible.
  Center,
  /// Place the children as close to the end of the main axis as possible.
  End,
  /// The children are evenly distributed within the alignment container along
  /// the main axis. The spacing between each pair of adjacent items is the
  /// same. The first item is flush with the main-start edge, and the last
  /// item is flush with the main-end edge.
  SpaceBetween,
  /// The children are evenly distributed within the alignment container
  /// along the main axis. The spacing between each pair of adjacent items is
  /// the same. The empty space before the first and after the last item
  /// equals half of the space between each pair of adjacent items.
  SpaceAround,
  /// The children are evenly distributed within the alignment container along
  /// the main axis. The spacing between each pair of adjacent items, the
  /// main-start edge and the first item, and the main-end edge and the last
  /// item, are all exactly the same.
  SpaceEvenly,
}

#[derive(Default, MultiChild, Declare, Clone, PartialEq)]
pub struct Flex {
  /// Reverse the main axis.
  #[declare(default)]
  pub reverse: bool,
  /// Whether flex items are forced onto one line or can wrap onto multiple
  /// lines
  #[declare(default)]
  pub wrap: bool,
  /// Sets how flex items are placed in the flex container defining the main
  /// axis and the direction
  #[declare(default)]
  pub direction: Direction,
  /// How the children should be placed along the cross axis in a flex layout.
  #[declare(default)]
  pub align_items: Align,
  /// How the children should be placed along the main axis in a flex layout.
  #[declare(default)]
  pub justify_content: JustifyContent,
}

impl Render for Flex {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let direction = self.direction;
    let mut layouter = FlexLayouter {
      max_size: FlexSize::from_size(clamp.max, direction),
      min_size: FlexSize::from_size(clamp.min, direction),
      direction,
      reverse: self.reverse,
      wrap: self.wrap,
      main_max: 0.,
      current_line: <_>::default(),
      lines_info: vec![],
      align_items: self.align_items,
      justify_content: self.justify_content,
    };
    layouter.layout(ctx)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for Flex {
  impl_query_self_only!();
}

#[derive(Debug, Clone, Copy, Default)]
struct FlexSize {
  main: f32,
  cross: f32,
}

impl FlexSize {
  fn to_size(self, dir: Direction) -> Size {
    match dir {
      Direction::Horizontal => Size::new(self.main, self.cross),
      Direction::Vertical => Size::new(self.cross, self.main),
    }
  }

  fn from_size(size: Size, dir: Direction) -> Self {
    match dir {
      Direction::Horizontal => Self { main: size.width, cross: size.height },
      Direction::Vertical => Self { cross: size.width, main: size.height },
    }
  }

  fn to_point(self, dir: Direction) -> Point { self.to_size(dir).to_vector().to_point() }

  fn from_point(pos: Point, dir: Direction) -> Self {
    FlexSize::from_size(Size::new(pos.x, pos.y), dir)
  }

  fn clamp(self, min: FlexSize, max: FlexSize) -> FlexSize {
    FlexSize {
      main: self.main.min(max.main).max(min.main),
      cross: self.cross.min(max.cross).max(min.cross),
    }
  }
}

impl std::ops::Sub for FlexSize {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    FlexSize {
      main: self.main - rhs.main,
      cross: self.cross - rhs.cross,
    }
  }
}

struct FlexLayouter {
  max_size: FlexSize,
  min_size: FlexSize,
  reverse: bool,
  direction: Direction,
  /// the max of child touch in main axis
  main_max: f32,
  wrap: bool,
  current_line: MainLineInfo,
  lines_info: Vec<MainLineInfo>,
  align_items: Align,
  justify_content: JustifyContent,
}

impl FlexLayouter {
  fn layout(&mut self, ctx: &mut LayoutCtx) -> Size {
    macro_rules! inner_layout {
      ($method: ident) => {{
        let (ctx, iter) = ctx.$method();
        self.children_perform(ctx, iter);
        let (ctx, iter) = ctx.$method();
        self.relayout_if_need(ctx, iter);
        let size = self.box_size();
        let (ctx, iter) = ctx.$method();
        self.line_inner_align(ctx, iter, size);
        size.to_size(self.direction)
      }};
    }
    if self.reverse {
      inner_layout!(split_rev_children)
    } else {
      inner_layout!(split_children)
    }
  }

  fn children_perform<'a>(
    &mut self,
    ctx: &mut LayoutCtx,
    children: impl Iterator<Item = WidgetId>,
  ) {
    let clamp = BoxClamp {
      max: self.max_size.to_size(self.direction),
      min: Size::zero(),
    };

    children.for_each(|child| {
      let size = ctx.perform_child_layout(child, clamp);
      let flex_size = FlexSize::from_size(size, self.direction);
      if self.wrap
        && !self.current_line.is_empty()
        && self.current_line.main_width + flex_size.main > self.max_size.main
      {
        self.place_line();
      }
      ctx.update_position(
        child,
        FlexSize {
          main: self.current_line.main_width,
          cross: self.current_line.cross_pos,
        }
        .to_point(self.direction),
      );
      self.place_widget(flex_size, child, ctx);
    });
    self.place_line();
  }

  fn relayout_if_need<'a>(
    &mut self,
    ctx: &mut LayoutCtx,
    mut children: impl Iterator<Item = WidgetId>,
  ) {
    let Self {
      lines_info,
      direction,
      align_items,
      max_size,
      main_max,
      ..
    } = self;
    lines_info.iter_mut().for_each(|line| {
      (0..line.child_count)
        .map(|_| children.next().unwrap())
        .fold(0.0f32, |main_offset, child| {
          Self::obj_real_rect_with_main_start(
            ctx,
            child,
            line,
            main_offset,
            *direction,
            *align_items,
            *max_size,
          )
        });
      *main_max = main_max.max(line.main_width);
    });
  }

  fn line_inner_align<'a>(
    &mut self,
    ctx: &mut LayoutCtx,
    mut children: impl Iterator<Item = WidgetId>,
    size: FlexSize,
  ) {
    let real_size = self.best_size();
    let Self {
      lines_info,
      justify_content: main_align,
      direction,
      align_items: cross_align,
      ..
    } = self;
    let container_cross_offset = cross_align.align_value(real_size.cross, size.cross);
    lines_info.iter_mut().for_each(|line| {
      let (offset, step) = match main_align {
        JustifyContent::Start => (0., 0.),
        JustifyContent::Center => ((size.main - line.main_width) / 2., 0.),
        JustifyContent::End => (size.main - line.main_width, 0.),
        JustifyContent::SpaceAround => {
          let step = (size.main - line.main_width) / line.child_count as f32;
          (step / 2., step)
        }
        JustifyContent::SpaceBetween => {
          let step = (size.main - line.main_width) / (line.child_count - 1) as f32;
          (0., step)
        }
        JustifyContent::SpaceEvenly => {
          let step = (size.main - line.main_width) / (line.child_count + 1) as f32;
          (step, step)
        }
      };

      (0..line.child_count)
        .map(|_| children.next().unwrap())
        .fold(offset, |main_offset: f32, child| {
          let rect = ctx
            .widget_box_rect(child)
            .expect("relayout a expanded widget which not prepare layout");
          let mut origin = FlexSize::from_point(rect.origin, *direction);
          let child_size = FlexSize::from_size(rect.size, *direction);

          let line_cross_offset = cross_align.align_value(child_size.cross, line.cross_line_height);
          origin.main += main_offset;
          origin.cross += container_cross_offset + line_cross_offset;
          ctx.update_position(child, origin.to_point(*direction));
          main_offset + step
        });
    });
  }

  fn place_widget(&mut self, size: FlexSize, child: WidgetId, ctx: &mut LayoutCtx) {
    let mut line = &mut self.current_line;
    line.main_width += size.main;
    line.cross_line_height = line.cross_line_height.max(size.cross);
    line.child_count += 1;
    if let Some(flex) = Self::child_flex(ctx, child) {
      line.flex_sum += flex;
      line.flex_main_width += size.main;
    }
  }

  fn place_line(&mut self) {
    if !self.current_line.is_empty() {
      self.main_max = self.main_max.max(self.current_line.main_width);
      let new_line = MainLineInfo {
        cross_pos: self.current_line.cross_bottom(),
        ..Default::default()
      };
      self
        .lines_info
        .push(std::mem::replace(&mut self.current_line, new_line));
    }
  }

  // relayout child to get the real size, and return the new offset in main axis
  // for next siblings.
  fn obj_real_rect_with_main_start(
    ctx: &mut LayoutCtx,
    child: WidgetId,
    line: &mut MainLineInfo,
    main_offset: f32,
    dir: Direction,
    cross_align: Align,
    max_size: FlexSize,
  ) -> f32 {
    let pre_layout_rect = ctx
      .widget_box_rect(child)
      .expect("relayout a expanded widget which not prepare layout");

    let pre_size = FlexSize::from_size(pre_layout_rect.size, dir);
    let mut prefer_main = pre_size.main;
    if let Some(flex) = Self::child_flex(ctx, child) {
      let remain_space = max_size.main - line.main_width + line.flex_main_width;
      prefer_main = remain_space * (flex / line.flex_sum);
      line.flex_sum -= flex;
      line.flex_main_width -= pre_size.main;
    }
    prefer_main = prefer_main.max(pre_size.main);

    let clamp_max = FlexSize {
      main: prefer_main,
      cross: line.cross_line_height,
    };
    let mut clamp_min = FlexSize { main: prefer_main, cross: 0. };
    if Align::Stretch == cross_align {
      clamp_min.cross = line.cross_line_height;
    }

    let real_size = if prefer_main > pre_size.main || clamp_min.cross > pre_size.cross {
      // Relayout only if the child object size may change.
      let new_size = ctx.perform_child_layout(
        child,
        BoxClamp {
          max: clamp_max.to_size(dir),
          min: clamp_min.to_size(dir),
        },
      );
      FlexSize::from_size(new_size, dir)
    } else {
      pre_size
    };

    let main_diff = real_size.main - pre_size.main;
    line.main_width += main_diff;

    let mut new_pos = FlexSize::from_point(pre_layout_rect.origin, dir);
    new_pos.main += main_offset;
    let new_pos = new_pos.to_point(dir);

    if pre_layout_rect.origin != new_pos {
      ctx.update_position(child, new_pos);
    }

    main_offset + main_diff
  }

  fn best_size(&self) -> FlexSize {
    let cross = self
      .lines_info
      .last()
      .map(|line| line.cross_bottom())
      .unwrap_or(0.);
    FlexSize { cross, main: self.main_max }
  }

  fn box_size(&self) -> FlexSize { self.best_size().clamp(self.min_size, self.max_size) }

  fn child_flex(ctx: &mut LayoutCtx, child: WidgetId) -> Option<f32> {
    let mut flex = None;
    ctx.query_widget_type(child, |expanded: &Expanded| flex = Some(expanded.flex));
    flex
  }
}

#[derive(Default)]
struct MainLineInfo {
  child_count: usize,
  cross_pos: f32,
  main_width: f32,
  flex_sum: f32,
  flex_main_width: f32,
  cross_line_height: f32,
}

impl MainLineInfo {
  fn is_empty(&self) -> bool { self.child_count == 0 || self.main_width == 0. }

  fn cross_bottom(&self) -> f32 { self.cross_pos + self.cross_line_height }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test::*;

  #[test]
  fn horizontal_line() {
    let row = widget! {
      Flex {
        ExprWidget {
          expr: (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
        }
      }
    };
    let (rect, _) = widget_and_its_children_box_rect(row.into_widget(), Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(100., 20.));
  }

  #[test]
  fn vertical_line() {
    let col = widget! {
      Flex {
        direction: Direction::Vertical,
        ExprWidget  {
         expr: (0..10).map(|_| SizedBox { size: Size::new(10., 20.) })
        }
      }
    };
    let (rect, _) = widget_and_its_children_box_rect(col.into_widget(), Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(10., 200.));
  }

  #[test]
  fn row_wrap() {
    let size = Size::new(200., 20.);
    let row = widget! {
      Flex {
        wrap: true,
        ExprWidget {
          expr: (0..3).map(|_| SizedBox { size })
        }
      }
    };

    let (rect, children) =
      widget_and_its_children_box_rect(row.into_widget(), Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(400., 40.));
    assert_eq!(
      children,
      vec![
        Rect::from_size(size),
        Rect { origin: Point::new(200., 0.), size },
        Rect { origin: Point::new(0., 20.), size },
      ]
    );
  }

  #[test]
  fn reverse_row_wrap() {
    let size = Size::new(200., 20.);
    let row = widget! {
      Flex {
        wrap: true,
        reverse: true,
        ExprWidget {
          expr: (0..3).map(|_| SizedBox { size })
        }
      }
    };

    let (rect, children) =
      widget_and_its_children_box_rect(row.into_widget(), Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(400., 40.));
    assert_eq!(
      children,
      vec![
        Rect { origin: Point::new(0., 20.), size },
        Rect { origin: Point::new(200., 0.), size },
        Rect::from_size(size),
      ]
    );
  }

  #[test]
  fn cross_align() {
    fn cross_align_check(align: Align, y_pos: [f32; 3]) {
      let row = widget! {
        Row {
          align_items: align,
          SizedBox { size: Size::new(100., 20.) }
          SizedBox { size: Size::new(100., 30.) }
          SizedBox { size: Size::new(100., 40.) }
        }
      };

      let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
      assert_eq!(rect.size, Size::new(300., 40.));
      assert_eq!(
        children,
        vec![
          Rect {
            origin: Point::new(0., y_pos[0]),
            size: Size::new(100., 20.)
          },
          Rect {
            origin: Point::new(100., y_pos[1]),
            size: Size::new(100., 30.)
          },
          Rect {
            origin: Point::new(200., y_pos[2]),
            size: Size::new(100., 40.)
          },
        ]
      );
    }
    cross_align_check(Align::Start, [0., 0., 0.]);
    cross_align_check(Align::Center, [10., 5., 0.]);
    cross_align_check(Align::End, [20., 10., 0.]);

    let row = widget! {
      Row {
        align_items: Align::Stretch,
        SizedBox { size: Size::new(100., 20.) }
        SizedBox { size: Size::new(100., 30.) }
        SizedBox { size: Size::new(100., 40.) }
      }
    };

    let (rect, children) = widget_and_its_children_box_rect(row, Size::new(500., 500.));
    assert_eq!(rect.size, Size::new(300., 40.));
    assert_eq!(
      children,
      vec![
        Rect {
          origin: Point::new(0., 0.),
          size: Size::new(100., 40.)
        },
        Rect {
          origin: Point::new(100., 0.),
          size: Size::new(100., 40.)
        },
        Rect {
          origin: Point::new(200., 0.),
          size: Size::new(100., 40.)
        },
      ]
    );
  }

  #[test]
  fn main_align() {
    fn main_align_check(justify_content: JustifyContent, pos: [(f32, f32); 3]) {
      let item_size = Size::new(100., 20.);
      let root = widget! {
        SizedBox {
          size: INFINITY_SIZE,
          Row {
            justify_content,
            align_items: Align::Start,
            SizedBox { size: item_size }
            SizedBox { size: item_size }
            SizedBox { size: item_size }
          }
        }
      };

      expect_layout_result(
        root,
        Some(Size::new(500., 500.)),
        &[
          LayoutTestItem {
            path: &[0, 0],
            expect: ExpectRect {
              width: Some(500.),
              height: Some(500.),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0, 0],
            expect: ExpectRect {
              x: Some(pos[0].0),
              y: Some(pos[0].1),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0, 1],
            expect: ExpectRect {
              x: Some(pos[1].0),
              y: Some(pos[1].1),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0, 2],
            expect: ExpectRect {
              x: Some(pos[2].0),
              y: Some(pos[2].1),
              ..<_>::default()
            },
          },
        ],
      );
    }

    main_align_check(JustifyContent::Start, [(0., 0.), (100., 0.), (200., 0.)]);
    main_align_check(JustifyContent::Center, [(100., 0.), (200., 0.), (300., 0.)]);
    main_align_check(JustifyContent::End, [(200., 0.), (300., 0.), (400., 0.)]);
    main_align_check(
      JustifyContent::SpaceBetween,
      [(0., 0.), (200., 0.), (400., 0.)],
    );
    let space = 200.0 / 3.0;
    main_align_check(
      JustifyContent::SpaceAround,
      [
        (0.5 * space, 0.),
        (100. + space * 1.5, 0.),
        (2.5 * space + 200., 0.),
      ],
    );
    main_align_check(
      JustifyContent::SpaceEvenly,
      [(50., 0.), (200., 0.), (350., 0.)],
    );
  }
}