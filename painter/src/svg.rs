use crate::{
  color::{LinearGradient, RadialGradient},
  Brush, Color, GradientStop, LineCap, LineJoin, Path, PathPaintStyle, StrokeOptions,
};
use ribir_geom::{Point, Size, Transform, Vector};
use serde::{Deserialize, Serialize};
use std::{error::Error, io::Read};
use usvg::{Options, Stop, Tree, TreeParsing};

#[derive(Serialize, Deserialize, Clone)]
pub struct Svg {
  pub size: Size,
  pub view_scale: Vector,
  pub paths: Box<[SvgPath]>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SvgPath {
  pub path: Path,
  pub brush: Brush,
  pub style: PathPaintStyle,
}

/// Fits size into a viewbox. copy from resvg
fn fit_view_box(size: usvg::Size, vb: &usvg::ViewBox) -> usvg::Size {
  let s = vb.rect.size();

  if vb.aspect.align == usvg::Align::None {
    s
  } else if vb.aspect.slice {
    size.expand_to(s)
  } else {
    size.scale_to(s)
  }
}

// todo: we need to support currentColor to change svg color.
impl Svg {
  pub fn parse_from_bytes(svg_data: &[u8]) -> Result<Self, Box<dyn Error>> {
    let opt = Options { ..<_>::default() };
    let tree = Tree::from_data(svg_data, &opt).unwrap();
    let view_rect = tree.view_box.rect;
    let size = tree.size;
    let fit_size = fit_view_box(size, &tree.view_box);

    let view_scale = Vector::new(
      size.width() / fit_size.width(),
      size.height() / fit_size.height(),
    )
    .to_f32();
    let t = Transform::translation(-view_rect.x(), -view_rect.y());

    let mut t_stack = TransformStack::new(t);
    let mut paths = vec![];

    tree.root.traverse().for_each(|edge| match edge {
      rctree::NodeEdge::Start(node) => {
        use usvg::NodeKind;

        match &*node.borrow() {
          NodeKind::Path(p) => {
            t_stack.push(matrix_convert(p.transform));
            let path = usvg_path_to_path(p);
            let path = path.transform(t_stack.current_transform());
            if let Some(ref fill) = p.fill {
              let brush = brush_from_usvg_paint(&fill.paint, fill.opacity, &size);

              paths.push(SvgPath {
                path: path.clone(),
                brush,
                style: PathPaintStyle::Fill,
              });
            }

            if let Some(ref stroke) = p.stroke {
              let cap = match stroke.linecap {
                usvg::LineCap::Butt => LineCap::Butt,
                usvg::LineCap::Square => LineCap::Square,
                usvg::LineCap::Round => LineCap::Round,
              };
              let join = match stroke.linejoin {
                usvg::LineJoin::Miter => LineJoin::Miter,
                usvg::LineJoin::Bevel => LineJoin::Bevel,
                usvg::LineJoin::Round => LineJoin::Round,
              };
              let options = StrokeOptions {
                width: stroke.width.get(),
                line_cap: cap,
                line_join: join,
                miter_limit: stroke.miterlimit.get(),
              };

              let brush = brush_from_usvg_paint(&stroke.paint, stroke.opacity, &size);
              paths.push(SvgPath {
                path,
                brush,
                style: PathPaintStyle::Stroke(options),
              });
            };
          }
          NodeKind::Image(_) => {
            // todo;
            log::warn!("[painter]: not support draw embed image in svg, ignored!");
          }
          NodeKind::Group(ref g) => {
            t_stack.push(matrix_convert(g.transform));
            // todo;
            if g.opacity.get() != 1. {
              log::warn!("[painter]: not support `opacity` in svg, ignored!");
            }
            if g.clip_path.is_some() {
              log::warn!("[painter]: not support `clip path` in svg, ignored!");
            }
            if g.mask.is_some() {
              log::warn!("[painter]: not support `mask` in svg, ignored!");
            }
            if !g.filters.is_empty() {
              log::warn!("[painter]: not support `filters` in svg, ignored!");
            }
          }
          NodeKind::Text(_) => {
            todo!("Not support text in SVG temporarily, we'll add it after refactoring `painter`.")
          }
        }
      }
      rctree::NodeEdge::End(_) => {
        t_stack.pop();
      }
    });

    Ok(Svg {
      size: Size::new(size.width(), size.height()),
      paths: paths.into_boxed_slice(),
      view_scale,
    })
  }

  pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut bytes = vec![];
    file.read_to_end(&mut bytes)?;
    Self::parse_from_bytes(&bytes)
  }

  pub fn serialize(&self) -> Result<String, Box<dyn Error>> {
    // use json replace bincode, because https://github.com/Ogeon/palette/issues/130
    Ok(serde_json::to_string(self)?)
  }

  pub fn deserialize(str: &str) -> Result<Self, Box<dyn Error>> { Ok(serde_json::from_str(str)?) }
}

fn usvg_path_to_path(path: &usvg::Path) -> Path {
  let mut builder = lyon_algorithms::path::Path::svg_builder();
  path.data.segments().for_each(|seg| match seg {
    usvg::tiny_skia_path::PathSegment::MoveTo(pt) => {
      builder.move_to(point(pt.x, pt.y));
    }
    usvg::tiny_skia_path::PathSegment::LineTo(pt) => {
      builder.line_to(point(pt.x, pt.y));
    }
    usvg::tiny_skia_path::PathSegment::CubicTo(pt1, pt2, pt3) => {
      builder.cubic_bezier_to(
        point(pt1.x, pt1.y),
        point(pt2.x, pt2.y),
        point(pt3.x, pt3.y),
      );
    }
    usvg::tiny_skia_path::PathSegment::QuadTo(pt1, pt2) => {
      builder.quadratic_bezier_to(point(pt1.x, pt1.y), point(pt2.x, pt2.y));
    }
    usvg::tiny_skia_path::PathSegment::Close => builder.close(),
  });

  builder.build().into()
}

fn point(x: f32, y: f32) -> lyon_algorithms::math::Point { Point::new(x, y).to_untyped() }

fn matrix_convert(t: usvg::Transform) -> Transform {
  let usvg::Transform { sx, kx, ky, sy, tx, ty } = t;
  Transform::new(sx, ky, kx, sy, tx, ty)
}

fn brush_from_usvg_paint(paint: &usvg::Paint, opacity: usvg::Opacity, size: &usvg::Size) -> Brush {
  match paint {
    usvg::Paint::Color(usvg::Color { red, green, blue }) => Color::from_rgb(*red, *green, *blue)
      .with_alpha(opacity.get())
      .into(),
    usvg::Paint::LinearGradient(linear) => {
      let stops = convert_to_gradient_stops(&linear.stops);
      let size_scale = match linear.units {
        usvg::Units::UserSpaceOnUse => (1., 1.),
        usvg::Units::ObjectBoundingBox => (size.width(), size.height()),
      };
      let gradient = LinearGradient {
        start: Point::new(linear.x1 * size_scale.0, linear.y1 * size_scale.1),
        end: Point::new(linear.y2 * size_scale.0, linear.y2 * size_scale.1),
        stops,
        transform: matrix_convert(linear.transform),
        spread_method: linear.spread_method.into(),
      };
      Brush::LinearGradient(gradient)
    }
    usvg::Paint::RadialGradient(radial_gradient) => {
      let stops = convert_to_gradient_stops(&radial_gradient.stops);
      let size_scale = match radial_gradient.units {
        usvg::Units::UserSpaceOnUse => (1., 1.),
        usvg::Units::ObjectBoundingBox => (size.width(), size.height()),
      };
      let gradient = RadialGradient {
        start_center: Point::new(
          radial_gradient.fx * size_scale.0,
          radial_gradient.fy * size_scale.1,
        ),
        start_radius: 0.,
        end_center: Point::new(
          radial_gradient.cx * size_scale.0,
          radial_gradient.cy * size_scale.1,
        ),
        end_radius: radial_gradient.r.get() * size_scale.0,
        stops,
        transform: matrix_convert(radial_gradient.transform),
        spread_method: radial_gradient.spread_method.into(),
      };
      Brush::RadialGradient(gradient)
    }
    paint => {
      log::warn!("[painter]: not support `{paint:?}` in svg, use black instead!");
      Color::BLACK.into()
    }
  }
}

fn convert_to_gradient_stops(stops: &Vec<Stop>) -> Vec<GradientStop> {
  assert!(!stops.is_empty());

  let mut stops: Vec<_> = stops
    .iter()
    .map(|stop| {
      let usvg::Color { red, green, blue } = stop.color;
      GradientStop {
        offset: stop.offset.get(),
        color: Color::from_rgb(red, green, blue),
      }
    })
    .collect();

  stops.sort_by(|s1, s2| s1.offset.partial_cmp(&s2.offset).unwrap());

  if let Some(first) = stops.first() {
    if first.offset != 0. {
      stops.insert(0, GradientStop { offset: 0., color: first.color });
    }
  }
  if let Some(last) = stops.last() {
    if last.offset < 1. {
      stops.push(GradientStop { offset: 1., color: last.color });
    }
  }
  stops
}

struct TransformStack {
  stack: Vec<Transform>,
}

impl TransformStack {
  fn new(t: Transform) -> Self { TransformStack { stack: vec![t] } }

  fn push(&mut self, mut t: Transform) {
    if let Some(p) = self.stack.last() {
      t = p.then(&t);
    }
    self.stack.push(t);
  }

  fn pop(&mut self) -> Option<Transform> { self.stack.pop() }

  fn current_transform(&self) -> &Transform { self.stack.last().unwrap() }
}
