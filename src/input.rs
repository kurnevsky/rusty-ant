use coordinates::*;

#[derive(Clone)]
pub enum Input { //TODO: remove Input prefix.
  InputWater(Point),
  InputFood(Point),
  InputAnthill(Point, uint),
  InputAnt(Point, uint),
  InputDead(Point, uint)
}
