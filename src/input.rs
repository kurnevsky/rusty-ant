use crate::coordinates::*;

#[derive(Clone, Copy)]
pub enum Input {
  Water(Point),
  Food(Point),
  Anthill(Point, u32),
  Ant(Point, u32),
  Dead(Point, u32),
}
