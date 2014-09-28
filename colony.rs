use std::*;
use std::collections::*;
use std::rand::*;
use point::*;
use direction::*;
use cell::*;
use move::*;
use input::*;

static GATHERING_FOOD_PATH_SIZE: uint = 30;

static ATTACK_ANTHILLS_PATH_SIZE: uint = 10;

static TERRITORY_PATH_SIZE_CONST: uint = 6;

#[deriving(Clone)]
struct Tag {
  start: uint,
  prev: uint,
  length: uint,
  general: uint
}

pub struct Colony {
  pub width: uint,
  pub height: uint,
  pub turn_time: uint,
  pub turns_count: uint,
  pub view_radius2: uint,
  pub attack_radius2: uint,
  pub spawn_radius2: uint,
  pub cur_turn: uint,
  rng: XorShiftRng,
  territory_path_size: uint,
  enemies_count: uint, // Известное количество врагов.
  world: Vec<Cell>, // Текущее состояние мира. При ходе нашего муравья он передвигается на новую клетку.
  last_world: Vec<Cell>, // Предыдущее состояние мира со сделавшими ход нашими муравьями.
  visible_area: Vec<uint>, // Равняется 0 для видимых клеток и известной воды, для остальных увеличивается на 1 перед каждым ходом.
  standing_ants: Vec<uint>, // Каждый ход увеличивается на 1 для вражеских муравьев и сбрасывается в 0 для всех остальных клеток. То есть показывает, сколько ходов на месте стоит вражеский муравей.
  moved: Vec<bool>, // Помечаются флагом клетки, откуда сделал ход наш муравей, а также куда он сделал ход.
  gathered_food: Vec<uint>, // Помечается флагом клетки с едой, к которым отправлен наш муравей. Значение - позиция муравья + 1.
  territory: Vec<uint>,
  groups: Vec<bool>,
  tags: Vec<Tag>,
  tagged: DList<uint>, // Список позиций start_tags и path_tags с ненулевыми значениями.
  ours_ants: DList<uint>, // Список наших муравьев. Если муравей сделал ход, позиция помечена в moved.
  enemies_ants: DList<uint>,
  enemies_anthills: DList<uint>,
  food: DList<uint> // Список клеток с едой (как в видимой области, так и за туманом войны, если видели там еду раньше).
}

impl Colony {
  pub fn new(width: uint, height: uint, turn_time: uint, turns_count: uint, view_radius2: uint, attack_radius2: uint, spawn_radius2: uint) -> Colony {
    let len = length(width, height);
    Colony {
      width: width,
      height: height,
      turn_time: turn_time,
      turns_count: turns_count,
      view_radius2: view_radius2,
      attack_radius2: attack_radius2,
      spawn_radius2: spawn_radius2,
      cur_turn: 0,
      rng: SeedableRng::from_seed([1, 2, 3, 4]),
      territory_path_size: ((view_radius2 * 2 * TERRITORY_PATH_SIZE_CONST) as f32).sqrt().ceil() as uint,
      enemies_count: 0,
      world: Vec::from_elem(len, Unknown),
      last_world: Vec::from_elem(len, Unknown),
      visible_area: Vec::from_elem(len, 0u),
      standing_ants: Vec::from_elem(len, 0u),
      moved: Vec::from_elem(len, false),
      gathered_food: Vec::from_elem(len, 0u),
      territory: Vec::from_elem(len, 0u),
      groups: Vec::from_elem(len, false),
      tags: Vec::from_elem(len, Tag { start: 0, prev: 0, length: 0, general: 0 }),
      tagged: DList::new(),
      ours_ants: DList::new(),
      enemies_ants: DList::new(),
      enemies_anthills: DList::new(),
      food: DList::new()
    }
  }
}

fn length(width: uint, height: uint) -> uint {
  width * height
}

fn to_pos(width: uint, point: Point) -> uint {
  point.y * width + point.x
}

fn from_pos(width: uint, pos: uint) -> Point {
  Point {
    x: pos % width,
    y: pos / width
  }
}

fn to_direction(width: uint, height: uint, pos1: uint, pos2: uint) -> Direction {
  if n(width, height, pos1) == pos2 {
    North
  } else if s(width, height, pos1) == pos2 {
    South
  } else if w(width, pos1) == pos2 {
    West
  } else {
    East
  }
}

fn point_n(height: uint, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y - 1 + height) % height
  }
}

fn point_s(height: uint, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y + 1) % height
  }
}

fn point_w(width: uint, point: Point) -> Point {
  Point {
    x: (point.x - 1 + width) % width,
    y: point.y
  }
}

fn point_e(width: uint, point: Point) -> Point {
  Point {
    x: (point.x + 1) % width,
    y: point.y
  }
}

fn n(width: uint, height: uint, pos: uint) -> uint {
  let len = length(width, height);
  (pos - width + len) % len
}

fn s(width: uint, height: uint, pos: uint) -> uint {
  (pos + width) % length(width, height)
}

fn w(width: uint, pos: uint) -> uint {
  if pos % width == 0 {
    pos + width - 1
  } else {
    pos - 1
  }
}

fn e(width: uint, pos: uint) -> uint {
  if pos % width == width - 1 {
    pos - width + 1
  } else {
    pos + 1
  }
}

fn nw(width: uint, height: uint, pos: uint) -> uint {
  n(width, height, w(width, pos))
}

fn ne(width: uint, height: uint, pos: uint) -> uint {
  n(width, height, e(width, pos))
}

fn sw(width: uint, height: uint, pos: uint) -> uint {
  s(width, height, w(width, pos))
}

fn se(width: uint, height: uint, pos: uint) -> uint {
  s(width, height, e(width, pos))
}

fn point_manhattan(width: uint, height: uint, point1: Point, point2: Point) -> uint {
  let diff_x = num::abs(point1.x as int - point2.x as int) as uint;
  let diff_y = num::abs(point1.y as int - point2.y as int) as uint;
  cmp::min(diff_x, width - diff_x) + cmp::min(diff_y, height - diff_y)
}

fn point_euclidean(width: uint, height: uint, point1: Point, point2: Point) -> uint {
  let diff_x = num::abs(point1.x as int - point2.x as int) as uint;
  let diff_y = num::abs(point1.y as int - point2.y as int) as uint;
  num::pow(cmp::min(diff_x, width - diff_x), 2) + num::pow(cmp::min(diff_y, height - diff_y), 2)
}

fn manhattan(width: uint, height: uint, pos1: uint, pos2: uint) -> uint {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_manhattan(width, height, point1, point2)
}

fn euclidean(width: uint, height: uint, pos1: uint, pos2: uint) -> uint {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_euclidean(width, height, point1, point2)
}

fn wave<'r, T: Iterator<&'r uint>>(width: uint, height: uint, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, start: &mut T, cond: |uint, uint, uint, uint, uint, &mut uint| -> bool, stop_cond: |uint, uint, uint, uint| -> bool) -> Option<uint> {
  let mut q = DList::new();
  for &pos in *start {
    if cond(pos, pos, 0, pos, 0, &mut tags.get_mut(pos).general) {
      q.push(pos);
      let tag = tags.get_mut(pos);
      tag.start = pos;
      tag.prev = pos;
      tag.length = 1;
    }
  }
  while !q.is_empty() {
    let pos = q.pop_front().unwrap();
    tagged.push(pos);
    let tag = (*tags)[pos];
    if stop_cond(pos, tag.start, tag.length - 1, tag.prev) {
      return Some(pos);
    }
    let start_pos = tag.start;
    let n_pos = n(width, height, pos);
    if (*tags)[n_pos].length == 0 && cond(n_pos, start_pos, tag.length, pos, (*tags)[pos].general, &mut tags.get_mut(n_pos).general) {
      let n_tag = tags.get_mut(n_pos);
      n_tag.start = start_pos;
      n_tag.prev = pos;
      n_tag.length = tag.length + 1;
      tagged.push(n_pos);
      q.push(n_pos);
    }
    let w_pos = w(width, pos);
    if (*tags)[w_pos].length == 0 && cond(w_pos, start_pos, tag.length, pos, (*tags)[pos].general, &mut tags.get_mut(w_pos).general) {
      let w_tag = tags.get_mut(w_pos);
      w_tag.start = start_pos;
      w_tag.prev = pos;
      w_tag.length = tag.length + 1;
      tagged.push(w_pos);
      q.push(w_pos);
    }
    let s_pos = s(width, height, pos);
    if (*tags)[s_pos].length == 0 && cond(s_pos, start_pos, tag.length, pos, (*tags)[pos].general, &mut tags.get_mut(s_pos).general) {
      let s_tag = tags.get_mut(s_pos);
      s_tag.start = start_pos;
      s_tag.prev = pos;
      s_tag.length = tag.length + 1;
      tagged.push(s_pos);
      q.push(s_pos);
    }
    let e_pos = e(width, pos);
    if (*tags)[e_pos].length == 0 && cond(e_pos, start_pos, tag.length, pos, (*tags)[pos].general, &mut tags.get_mut(e_pos).general) {
      let e_tag = tags.get_mut(e_pos);
      e_tag.start = start_pos;
      e_tag.prev = pos;
      e_tag.length = tag.length + 1;
      tagged.push(e_pos);
      q.push(e_pos);
    }
  }
  return None;
}

fn simple_wave(width: uint, height: uint, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, start: uint, cond: |uint, uint, uint, uint, &mut uint| -> bool, stop_cond: |uint, uint, uint| -> bool) -> Option<uint> {
  wave(width, height, tags, tagged, &mut Some(start).iter(), |pos, _, path_size, prev, prev_general_tag, general_tag| { cond(pos, path_size, prev, prev_general_tag, general_tag) }, |pos, _, path_size, prev| { stop_cond(pos, path_size, prev) })
}

fn clear_tags(tags: &mut Vec<Tag>, tagged: &mut DList<uint>) {
  for &pos in tagged.iter() {
    let tag = tags.get_mut(pos);
    tag.start = 0;
    tag.prev = 0;
    tag.length = 0;
    tag.general = 0;
  }
  tagged.clear();
}

fn find_path<T: Deque<uint>>(tags: &Vec<Tag>, from: uint, to: uint, path: &mut T) {
  path.clear();
  if tags[to].start != from {
    return;
  }
  let mut pos = to;
  while pos != from {
    path.push_front(pos);
    pos = tags[pos].prev;
  }
}

fn is_free(cell: Cell) -> bool {
  match cell {
    Land | Unknown | Anthill(_) => true,
    _ => false
  }
}

/*fn is_free_or_food(cell: Cell) -> bool {
  match cell {
    Land | Unknown | Anthill(_) | Food => true,
    _ => false
  }
}*/

fn discover_direction(width: uint, height: uint, view_radius2: uint, world: &Vec<Cell>, visible_area: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, ant_pos: uint) -> Option<Direction> {
  let mut n_score = 0u;
  let mut s_score = 0u;
  let mut w_score = 0u;
  let mut e_score = 0u;
  let n_pos = n(width, height, ant_pos);
  let s_pos = s(width, height, ant_pos);
  let w_pos = w(width, ant_pos);
  let e_pos = e(width, ant_pos);
  if is_free(world[n_pos]) {
    simple_wave(width, height, tags, tagged, n_pos, |pos, path_size, prev, _, _| {
      if pos == s(width, height, prev) || path_size > manhattan(width, height, n_pos, pos) || euclidean(width, height, n_pos, pos) > view_radius2 || world[pos] == Water {
        false
      } else {
        if euclidean(width, height, ant_pos, pos) > view_radius2 {
          n_score += visible_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  if is_free(world[s_pos]) {
    simple_wave(width, height, tags, tagged, s_pos, |pos, path_size, prev, _, _| {
      if pos == n(width, height, prev) || path_size > manhattan(width, height, s_pos, pos) || euclidean(width, height, s_pos, pos) > view_radius2 || world[pos] == Water {
        false
      } else {
        if euclidean(width, height, ant_pos, pos) > view_radius2 {
          s_score += visible_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  if is_free(world[w_pos]) {
    simple_wave(width, height, tags, tagged, w_pos, |pos, path_size, prev, _, _| {
      if pos == e(width, prev) || path_size > manhattan(width, height, w_pos, pos) || euclidean(width, height, w_pos, pos) > view_radius2 || world[pos] == Water {
        false
      } else {
        if euclidean(width, height, ant_pos, pos) > view_radius2 {
          w_score += visible_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  if is_free(world[e_pos]) {
    simple_wave(width, height, tags, tagged, e_pos, |pos, path_size, prev, _, _| {
      if pos == w(width, prev) || path_size > manhattan(width, height, e_pos, pos) || euclidean(width, height, e_pos, pos) > view_radius2 || world[pos] == Water {
        false
      } else {
        if euclidean(width, height, ant_pos, pos) > view_radius2 {
          e_score += visible_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  /*simple_wave(width, height, tags, tagged, ant_pos, |pos, path_size, prev| {
    let point = from_pos(width, pos);
    if path_size > point_manhattan(width, height, point, ant_point) {
      return false;
    }
    let distance = point_euclidean(width, height, point, ant_point);
    if distance > view_radius2 {
      if point_euclidean(width, height, point, point_n(height, ant_point)) <= view_radius2 {
        n_score += visible_area[pos];
      }
      if point_euclidean(width, height, point, point_w(width, ant_point)) <= view_radius2 {
        w_score += visible_area[pos];
      }
      if point_euclidean(width, height, point, point_s(height, ant_point)) <= view_radius2 {
        s_score += visible_area[pos];
      }
      if point_euclidean(width, height, point, point_e(width, ant_point)) <= view_radius2 {
        e_score += visible_area[pos];
      }
      let prev_point = from_pos(width, prev);
      let prev_distance = point_euclidean(width, height, point, prev_point);
      prev_distance <= view_radius2
    } else {
      world[pos] != Water
    }
  }, |_, _, _| { false });
  clear_tags(tags, tagged);
  if !is_free(world[n(width, height, ant_pos)]) {
    n_score = 0;
  }
  if !is_free(world[s(width, height, ant_pos)]) {
    s_score = 0;
  }
  if !is_free(world[w(width, ant_pos)]) {
    w_score = 0;
  }
  if !is_free(world[e(width, ant_pos)]) {
    e_score = 0;
  }*/
  if n_score == 0 && s_score == 0 && w_score == 0 && e_score == 0 {
    None
  } else if n_score >= s_score && n_score >= w_score && n_score >= e_score {
    Some(North)
  } else if s_score >= n_score && s_score >= w_score && s_score >= e_score {
    Some(South)
  } else if w_score >= e_score && w_score >= n_score && w_score >= s_score {
    Some(West)
  } else {
    Some(East)
  }
}

fn move<T: MutableSeq<Move>>(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut T, pos: uint, direction: Direction) {
  *world.get_mut(pos) = match (*world)[pos] {
    AnthillWithAnt(0) => Anthill(0),
    _ => Land
  };
  *moved.get_mut(pos) = true;
  match direction {
    North => {
      let n_pos = n(width, height, pos);
      *world.get_mut(n_pos) = if (*world)[n_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
      *moved.get_mut(n_pos) = true;
    },
    West => {
      let w_pos = w(width, pos);
      *world.get_mut(w_pos) = if (*world)[w_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
      *moved.get_mut(w_pos) = true;
    },
    South => {
      let s_pos = s(width, height, pos);
      *world.get_mut(s_pos) = if (*world)[s_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
      *moved.get_mut(s_pos) = true;
    },
    East => {
      let e_pos = e(width, pos);
      *world.get_mut(e_pos) = if (*world)[e_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
      *moved.get_mut(e_pos) = true;
    }
  }
  output.push(Move { point: from_pos(width, pos), direction: direction })
}

fn update_world<'r, T: Iterator<&'r Input>>(colony: &mut Colony, input: &mut T) {
  let view_radius2 = colony.view_radius2;
  let width = colony.width;
  let height = colony.height;
  let visible_area = &mut colony.visible_area;
  let len = length(width, height);
  for pos in range(0u, len) {
    *colony.last_world.get_mut(pos) = colony.world[pos];
    *colony.world.get_mut(pos) = Unknown;
    *colony.moved.get_mut(pos) = false;
    *colony.gathered_food.get_mut(pos) = 0;
    *visible_area.get_mut(pos) += 1;
    *colony.territory.get_mut(pos) = 0;
    *colony.groups.get_mut(pos) = false;
  }
  colony.ours_ants.clear();
  colony.enemies_ants.clear();
  colony.enemies_anthills.clear();
  colony.food.clear();
  for &i in *input {
    match i {
      InputWater(point) => {
        let pos = to_pos(width, point);
        *colony.world.get_mut(pos) = Water;
      },
      InputFood(point) => {
        let pos = to_pos(width, point);
        *colony.world.get_mut(pos) = Food;
        colony.food.push(pos);
      },
      InputAnthill(point, player) => {
        let pos = to_pos(width, point);
        *colony.world.get_mut(pos) = if colony.world[pos] == Ant(player) { AnthillWithAnt(player) } else { Anthill(player) };
        if player != 0 {
          colony.enemies_anthills.push(pos);
          if player > colony.enemies_count {
            colony.enemies_count = player;
          }
        }
      },
      InputAnt(point, player) => {
        let pos = to_pos(width, point);
        *colony.world.get_mut(pos) = if colony.world[pos] == Anthill(player) { AnthillWithAnt(player) } else { Ant(player) };
        if player == 0 {
          colony.ours_ants.push(pos);
        } else {
          colony.enemies_ants.push(pos);
          if player > colony.enemies_count {
            colony.enemies_count = player;
          }
        }
      },
      InputDead(_, _) => { }
    }
  }
  for &ant_pos in colony.ours_ants.iter() {
    simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, _, _, _| {
      let distance = euclidean(width, height, pos, ant_pos);
      if distance > view_radius2 {
        false
      } else {
        *visible_area.get_mut(pos) = 0;
        true
      }
    }, |_, _, _| { false });
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
  /*wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_ants.iter(), |pos, start_pos, _, _| {
    let distance = euclidean(width, height, pos, start_pos);
    if distance > view_radius2 {
      false
    } else {
      *visible_area.get_mut(pos) = 0;
      true
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);*/
  for pos in range(0u, len) {
    if (*visible_area)[pos] == 0 {
      if colony.world[pos] == Unknown {
        *colony.world.get_mut(pos) = match colony.last_world[pos] {
          Water => Water,
          _ => Land
        }
      }
      match colony.world[pos] {
        Ant(player) if player > 1 => *colony.standing_ants.get_mut(pos) += 1,
        AnthillWithAnt(player) if player > 1 => *colony.standing_ants.get_mut(pos) += 1,
        _ => *colony.standing_ants.get_mut(pos) = 0
      }
    } else {
      *colony.world.get_mut(pos) = match colony.last_world[pos] {
         Water => {
          *visible_area.get_mut(pos) = 0;
          Water
        },
        Food => {
          colony.food.push(pos);
          Food
        },
        Land => Land,
        Unknown => Unknown,
        Ant(0) | AnthillWithAnt(0) => Land,
        Anthill(0) => {
          Anthill(0)
        },
        Ant(player) => {
          colony.enemies_ants.push(pos);
          Ant(player)
        },
        Anthill(player) => {
          colony.enemies_anthills.push(pos);
          Anthill(player)
        }
        AnthillWithAnt(player) => {
          colony.enemies_anthills.push(pos);
          colony.enemies_ants.push(pos);
          AnthillWithAnt(player)
        }
      };
      *colony.standing_ants.get_mut(pos) = 0;
    }
  }
}

fn discover<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) { //TODO: сделать так, чтобы рядомстоящие муравьи не исследовали одну и ту же область.
  for &pos in colony.ours_ants.iter() {
    if colony.moved[pos] {
      continue;
    }
    match discover_direction(colony.width, colony.height, colony.view_radius2, &colony.world, &colony.visible_area, &mut colony.tags, &mut colony.tagged, pos) {
      Some(d) => move(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, pos, d),
      None => { }
    }
  }
}

fn is_ant(cell: Cell, player: uint) -> bool {
  cell == Ant(player) || cell == AnthillWithAnt(player)
}

fn travel<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let territory = &mut colony.territory;
  let territory_path_size = colony.territory_path_size;
  let moved = &mut colony.moved;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_ants.iter().chain(colony.enemies_ants.iter()).chain(colony.enemies_anthills.iter()), |pos, start_pos, path_size, _, _, _| {
    if path_size < territory_path_size && (*world)[pos] != Water {
      match (*world)[start_pos] {
        AnthillWithAnt(player) => *territory.get_mut(pos) = player + 1,
        Ant(player) => *territory.get_mut(pos) = player + 1,
        Anthill(player) => *territory.get_mut(pos) = player + 1,
        _ => *territory.get_mut(pos) = 1
      }
      true
    } else {
      false
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
  let mut path = DList::new();
  let mut shuffled_ants = Vec::new();
  for &ant_pos in colony.ours_ants.iter() {
    if !(*moved)[ant_pos] {
      shuffled_ants.push(ant_pos);
    }
  }
  colony.rng.shuffle(shuffled_ants.as_mut_slice());
  for &ant_pos in shuffled_ants.iter() {
    if (*moved)[ant_pos] {
      continue;
    }
    let goal = simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, path_size, _, prev_general_tag, general_tag| {
      let cell = (*world)[pos];
      let prev_general_tag_or_start = prev_general_tag == 1 || pos == ant_pos;
      if cell == Water || prev_general_tag_or_start && ((*moved)[pos] && is_ant(cell, 0) || cell == Food) {
        false
      } else {
        *general_tag = if prev_general_tag_or_start && is_ant(cell, 0) { 1 } else { 0 };
        true
      }
    }, |pos, _, _| { (*territory)[pos] != 1 });
    if goal.is_none() {
      continue;
    }
    find_path(&mut colony.tags, ant_pos, goal.unwrap(), &mut path);
    clear_tags(&mut colony.tags, &mut colony.tagged);
    let mut moves = DList::new();
    moves.push(ant_pos);
    for &pos in path.iter() {
      moves.push(pos);
      let cell = (*world)[pos];
      if cell != Ant(0) && cell != AnthillWithAnt(0) {
        break;
      }
    }
    while moves.len() > 1 {
      let next_ant_pos = moves.pop().unwrap();
      let pos = *moves.back().unwrap();
      let direction = to_direction(width, height, pos, next_ant_pos);
      move(width, height, world, moved, output, pos, direction);
    }
  }
}

fn attack_anthills<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.enemies_anthills.iter(), |pos, start_pos, path_size, prev, _, _| {
    match (*world)[pos] {
      Ant(0) | AnthillWithAnt(0) if !(*moved)[pos] => {
        if pos != start_pos && !is_free((*world)[prev]) {
          false
        } else {
          let direction = to_direction(width, height, pos, prev);
          move(width, height, world, moved, output, pos, direction);
          true
        }
      },
      Unknown | Water => false,
      _ => path_size <= ATTACK_ANTHILLS_PATH_SIZE
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn gather_food<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let gathered_food = &mut colony.gathered_food;
  let moved = &mut colony.moved;
  for &pos in colony.ours_ants.iter() {
    if (*moved)[pos] {
      continue;
    }
    let n_pos = n(width, height, pos);
    if (*world)[n_pos] == Food && (*gathered_food)[n_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(n_pos) = pos + 1;
    }
    let s_pos = s(width, height, pos);
    if (*world)[s_pos] == Food && (*gathered_food)[s_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(s_pos) = pos + 1;
    }
    let w_pos = w(width, pos);
    if (*world)[w_pos] == Food && (*gathered_food)[w_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(w_pos) = pos + 1;
    }
    let e_pos = e(width, pos);
    if (*world)[e_pos] == Food && (*gathered_food)[e_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(e_pos) = pos + 1;
    }
  }
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.food.iter(), |pos, start_pos, path_size, prev, _, _| {
    match (*world)[pos] {
      Ant(0) | AnthillWithAnt(0) if (*gathered_food)[start_pos] == 0 && !(*moved)[pos] => {
        if pos != start_pos && !is_free((*world)[prev]) {
          false
        } else {
          let direction = to_direction(width, height, pos, prev);
          move(width, height, world, moved, output, pos, direction);
          *gathered_food.get_mut(start_pos) = pos + 1;
          true
        }
      },
      Unknown | Water => false,
      _ => path_size <= GATHERING_FOOD_PATH_SIZE
    }
  }, |_, _, _, _| { false });
  /*let mut path = DList::new();
  for &food_pos in colony.food.iter() {
    let mut ant_pos = (*gathered_food)[food_pos];
    if ant_pos == 0 {
      continue;
    }
    ant_pos -= 1;
    if (*moved)[ant_pos] {
      continue;
    }
    find_path(&mut colony.tags, food_pos, ant_pos, &mut path);
    path.pop();
    let next_ant_pos = path.pop().unwrap();
    let direction = to_direction(width, height, ant_pos, next_ant_pos);
    move(width, height, world, moved, output, ant_pos, direction);
  }*/
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn in_one_group(width: uint, height: uint, pos1: uint, pos2: uint, attack_radius2: uint, world: &Vec<Cell>) -> bool {
  let distance = euclidean(width, height, pos1, pos2);
  if distance <= attack_radius2 {
    return true;
  }
  let n_pos1 = n(width, height, pos1);
  let s_pos1 = s(width, height, pos1);
  let w_pos1 = w(width, pos1);
  let e_pos1 = e(width, pos1);
  let n_pos2 = n(width, height, pos2);
  let s_pos2 = s(width, height, pos2);
  let w_pos2 = w(width, pos2);
  let e_pos2 = e(width, pos2);
  let n_pos1_water = world[n_pos1] == Water;
  let s_pos1_water = world[s_pos1] == Water;
  let w_pos1_water = world[w_pos1] == Water;
  let e_pos1_water = world[e_pos1] == Water;
  let n_pos2_water = world[n_pos2] == Water;
  let s_pos2_water = world[s_pos2] == Water;
  let w_pos2_water = world[w_pos2] == Water;
  let e_pos2_water = world[e_pos2] == Water;
  if !n_pos1_water {
    let n_distance = euclidean(width, height, n_pos1, pos2);
    if n_distance <= attack_radius2 {
      return true;
    }
    if n_distance < distance {
      if !s_pos2_water && euclidean(width, height, n_pos1, s_pos2) <= attack_radius2 {
        return true;
      }
      if !w_pos2_water && euclidean(width, height, n_pos1, w_pos2) <= attack_radius2 {
        return true;
      }
      if !e_pos2_water && euclidean(width, height, n_pos1, e_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  if !s_pos1_water {
    let s_distance = euclidean(width, height, s_pos1, pos2);
    if s_distance <= attack_radius2 {
      return true;
    }
    if s_distance < distance {
      if !n_pos2_water && euclidean(width, height, s_pos1, n_pos2) <= attack_radius2 {
        return true;
      }
      if !w_pos2_water && euclidean(width, height, s_pos1, w_pos2) <= attack_radius2 {
        return true;
      }
      if !e_pos2_water && euclidean(width, height, s_pos1, e_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  if !w_pos1_water {
    let w_distance = euclidean(width, height, w_pos1, pos2);
    if w_distance <= attack_radius2 {
      return true;
    }
    if w_distance < distance {
      if !e_pos2_water && euclidean(width, height, w_pos1, e_pos2) <= attack_radius2 {
        return true;
      }
      if !n_pos2_water && euclidean(width, height, w_pos1, n_pos2) <= attack_radius2 {
        return true;
      }
      if !s_pos2_water && euclidean(width, height, w_pos1, s_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  if !e_pos1_water {
    let e_distance = euclidean(width, height, e_pos1, pos2);
    if e_distance <= attack_radius2 {
      return true;
    }
    if e_distance < distance {
      if !w_pos2_water && euclidean(width, height, e_pos1, w_pos2) <= attack_radius2 {
        return true;
      }
      if !n_pos2_water && euclidean(width, height, e_pos1, n_pos2) <= attack_radius2 {
        return true;
      }
      if !s_pos2_water && euclidean(width, height, e_pos1, s_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  false
}

fn find_near_ants<T: MutableSeq<uint>>(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, groups: &mut Vec<bool>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, group: &mut T, ours: bool) {
  simple_wave(width, height, tags, tagged, ant_pos, |pos, path_size, prev, _, _| {
    if !(*groups)[pos] {
      match (*world)[pos] {
        Ant(0) | AnthillWithAnt(0) => {
          if ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world) {
            group.push(pos);
            *groups.get_mut(pos) = true;
          }
        },
        Ant(_) | AnthillWithAnt(_) => {
          if !ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world) {
            group.push(pos);
            *groups.get_mut(pos) = true;
          }
        },
        _ => { }
      }
    }
    if euclidean(width, height, ant_pos, prev) <= attack_radius2 {
      true
    } else {
      false
    }
  }, |_, _, _| { false });
  clear_tags(tags, tagged);
}

fn get_group<T: MutableSeq<uint>>(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, groups: &mut Vec<bool>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, ours: &mut T, enemies: &mut T) {
  ours.clear();
  enemies.clear();
  let mut ours_q = DList::new();
  let mut enemies_q = DList::new();
  ours_q.push(ant_pos);
  while !ours_q.is_empty() || !enemies_q.is_empty() {
    if !ours_q.is_empty() {
      let pos = ours_q.pop_front().unwrap();
      ours.push(pos);
      find_near_ants(width, height, pos, attack_radius2, world, groups, tags, tagged, &mut enemies_q, false);
    }
    if !enemies_q.is_empty() {
      let pos = enemies_q.pop_front().unwrap();
      enemies.push(pos);
      find_near_ants(width, height, pos, attack_radius2, world, groups, tags, tagged, &mut ours_q, true);
    }
  }
}

fn attack<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let mut ours = Vec::new();
  let mut enemies = Vec::new();
  for &pos in colony.ours_ants.iter() {
    if colony.groups[pos] {
      continue;
    }
    get_group(colony.width, colony.height, pos, colony.attack_radius2, &colony.world, &mut colony.groups, &mut colony.tags, &mut colony.tagged, &mut ours, &mut enemies);
    //TODO
  }
}

pub fn turn<'r, T1: Iterator<&'r Input>, T2: MutableSeq<Move>>(colony: &mut Colony, input: &mut T1, output: &mut T2) {
  output.clear();
  colony.cur_turn += 1;
  update_world(colony, input);
  attack_anthills(colony, output);
  gather_food(colony, output);
  discover(colony, output);
  travel(colony, output);
}
