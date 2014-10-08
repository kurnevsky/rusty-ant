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

pub fn is_free(cell: Cell) -> bool {
  match cell {
    Land | Unknown | Anthill(_) => true,
    _ => false
  }
}

pub fn is_water_or_food(cell: Cell) -> bool {
  match cell {
    Water | Food => true,
    _ => false
  }
}

pub fn is_ant(cell: Cell) -> bool {
  match cell {
    AnthillWithAnt(_) | Ant(_) => true,
    _ => false
  }
}

pub fn is_players_ant(cell: Cell, player: uint) -> bool {
  cell == Ant(player) || cell == AnthillWithAnt(player)
}

pub fn is_enemy_ant(cell: Cell) -> bool {
  match cell {
    Ant(player) if player > 0 => true,
    AnthillWithAnt(player) if player > 0 => true,
    _ => false
  }
}

pub fn is_enemy_anthill(cell: Cell) -> bool {
  match cell {
    Anthill(player) if player > 0 => true,
    AnthillWithAnt(player) if player > 0 => true,
    _ => false
  }
}

pub fn is_our_anthill(cell: Cell) -> bool {
  cell == Anthill(0) || cell == AnthillWithAnt(0)
}

pub fn ant_owner(cell: Cell) -> Option<uint> {
  match cell {
    Ant(player) => Some(player),
    AnthillWithAnt(player) => Some(player),
    _ => None
  }
}
