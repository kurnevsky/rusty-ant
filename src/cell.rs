#[derive(Clone, Copy, PartialEq)]
pub enum Cell {
  Anthill(u32),
  AnthillWithAnt(u32),
  Ant(u32),
  Food,
  Land,
  Water,
  Unknown,
}

pub fn is_free(cell: Cell) -> bool {
  match cell {
    Cell::Land | Cell::Unknown | Cell::Anthill(_) => true,
    _ => false,
  }
}

pub fn is_water_or_food(cell: Cell) -> bool {
  match cell {
    Cell::Water | Cell::Food => true,
    _ => false,
  }
}

pub fn is_ant(cell: Cell) -> bool {
  match cell {
    Cell::AnthillWithAnt(_) | Cell::Ant(_) => true,
    _ => false,
  }
}

pub fn is_players_ant(cell: Cell, player: u32) -> bool {
  cell == Cell::Ant(player) || cell == Cell::AnthillWithAnt(player)
}

pub fn is_enemy_ant(cell: Cell) -> bool {
  match cell {
    Cell::Ant(player) | Cell::AnthillWithAnt(player) if player > 0 => true,
    _ => false,
  }
}

pub fn is_enemy_anthill(cell: Cell) -> bool {
  match cell {
    Cell::Anthill(player) | Cell::AnthillWithAnt(player) if player > 0 => true,
    _ => false,
  }
}

pub fn is_our_anthill(cell: Cell) -> bool {
  cell == Cell::Anthill(0) || cell == Cell::AnthillWithAnt(0)
}

pub fn ant_owner(cell: Cell) -> Option<u32> {
  match cell {
    Cell::Ant(player) | Cell::AnthillWithAnt(player) => Some(player),
    _ => None,
  }
}
