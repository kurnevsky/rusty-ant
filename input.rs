use coordinates::*;

pub enum Input {
  InputWater(Point),
  InputFood(Point),
  InputAnthill(Point, uint),
  InputAnt(Point, uint),
  InputDead(Point, uint)
}
