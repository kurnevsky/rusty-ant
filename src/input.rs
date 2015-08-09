use coordinates::*;

#[derive(Clone, Copy)]
pub enum Input {
  Water(Point),
  Food(Point),
  Anthill(Point, usize),
  Ant(Point, usize),
  Dead(Point, usize)
}
