#[deriving(Clone, PartialEq)]
pub enum Cell {
  Anthill(uint),
  AnthillWithAnt(uint),
  Ant(uint),
  Food,
  Land,
  Water,
  Unknown
}
