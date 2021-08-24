pub trait Declare {
  type Builder: DeclareBuilder<Target = Self>;
}

pub trait DeclareBuilder {
  type Target;
  fn build(self) -> Self::Target;
}