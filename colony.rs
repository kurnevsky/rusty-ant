//TODO: учет еды, муравейников, расстояния до вражеских муравьев в функции оценки.
//TODO: убегание, если в группе один свой муравей.

extern crate time;

use std::{int, num, cmp, io};
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

static MINIMAX_TIME: f32 = 0.5;

static ENEMIES_DEAD_ESTIMATION_CONST: uint = 2;

static OURS_DEAD_ESTIMATION_CONST: uint = 3;

static STANDING_ANTS_CONST: uint = 4;

#[deriving(Clone)]
struct Tag {
  start: uint,
  prev: uint,
  length: uint,
  general: uint
}

#[deriving(Clone)]
struct BoardCell {
  ant: uint,
  attack: uint,
  cycle: uint
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
  start_time: u64,
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
  dangerous_place: Vec<bool>,
  dangerous_place_for_enemies: Vec<bool>,
  groups: Vec<uint>,
  board: Vec<BoardCell>,
  tags: Vec<Tag>,
  tagged: DList<uint>, // Список позиций start_tags и path_tags с ненулевыми значениями.
  ours_ants: DList<uint>, // Список наших муравьев. Если муравей сделал ход, позиция помечена в moved.
  enemies_ants: DList<uint>,
  enemies_anthills: DList<uint>,
  food: DList<uint> // Список клеток с едой (как в видимой области, так и за туманом войны, если видели там еду раньше).
  //aggressive_place
}

impl Colony {
  pub fn new(width: uint, height: uint, turn_time: uint, turns_count: uint, view_radius2: uint, attack_radius2: uint, spawn_radius2: uint, seed: u64) -> Colony {
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
      start_time: get_time(),
      rng: SeedableRng::from_seed([1, ((seed >> 32) & 0xFFFFFFFF) as u32, 3, (seed & 0xFFFFFFFF) as u32]),
      territory_path_size: ((view_radius2 * 2 * TERRITORY_PATH_SIZE_CONST) as f32).sqrt().ceil() as uint,
      enemies_count: 0,
      world: Vec::from_elem(len, Unknown),
      last_world: Vec::from_elem(len, Unknown),
      visible_area: Vec::from_elem(len, 0u),
      standing_ants: Vec::from_elem(len, 0u),
      moved: Vec::from_elem(len, false),
      gathered_food: Vec::from_elem(len, 0u),
      territory: Vec::from_elem(len, 0u),
      dangerous_place: Vec::from_elem(len, false),
      dangerous_place_for_enemies: Vec::from_elem(len, false),
      groups: Vec::from_elem(len, 0u),
      board: Vec::from_elem(len, BoardCell { ant: 0, attack: 0, cycle: 0 }),
      tags: Vec::from_elem(len, Tag { start: 0, prev: 0, length: 0, general: 0 }),
      tagged: DList::new(),
      ours_ants: DList::new(),
      enemies_ants: DList::new(),
      enemies_anthills: DList::new(),
      food: DList::new()
    }
  }
}

fn get_time() -> u64 {
  time::precise_time_ns() / 1000000
}

fn elapsed_time(start_time: u64) -> uint {
  (get_time() - start_time) as uint
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

fn to_direction(width: uint, height: uint, pos1: uint, pos2: uint) -> Option<Direction> {
  if n(width, height, pos1) == pos2 {
    Some(North)
  } else if s(width, height, pos1) == pos2 {
    Some(South)
  } else if w(width, pos1) == pos2 {
    Some(West)
  } else if e(width, pos1) == pos2 {
    Some(East)
  } else {
    None
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

fn is_water_or_food(cell: Cell) -> bool {
  match cell {
    Water | Food => true,
    _ => false
  }
}

/*fn is_free_or_food(cell: Cell) -> bool {
  match cell {
    Land | Unknown | Anthill(_) | Food => true,
    _ => false
  }
}*/

fn discover_direction(width: uint, height: uint, view_radius2: uint, world: &Vec<Cell>, visible_area: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, ant_pos: uint) -> Option<uint> {
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
    Some(n_pos)
  } else if s_score >= n_score && s_score >= w_score && s_score >= e_score {
    Some(s_pos)
  } else if w_score >= e_score && w_score >= n_score && w_score >= s_score {
    Some(w_pos)
  } else {
    Some(e_pos)
  }
}

fn move<T: MutableSeq<Move>>(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut T, pos: uint, next_pos: uint) {
  *world.get_mut(pos) = match (*world)[pos] {
    AnthillWithAnt(0) => Anthill(0),
    _ => Land
  };
  *moved.get_mut(pos) = true;
  *world.get_mut(next_pos) = if (*world)[next_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
  *moved.get_mut(next_pos) = true;
  output.push(Move { point: from_pos(width, pos), direction: to_direction(width, height, pos, next_pos).unwrap() })
}

fn move_all<T: MutableSeq<Move>>(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut T, moves: &DList<(uint, uint)>) {
  for &(pos, _) in moves.iter() {
    *world.get_mut(pos) = match (*world)[pos] {
      AnthillWithAnt(0) => Anthill(0),
      _ => Land
    };
    *moved.get_mut(pos) = true;
  }
  for &(pos, next_pos) in moves.iter() {
    *world.get_mut(next_pos) = if (*world)[next_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
    *moved.get_mut(next_pos) = true;
    output.push(Move { point: from_pos(width, pos), direction: to_direction(width, height, pos, next_pos).unwrap() });
  }
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
    *colony.groups.get_mut(pos) = 0;
    *colony.dangerous_place.get_mut(pos) = false;
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
      Some(next_pos) => move(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, pos, next_pos),
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
    let goal = simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, _, prev_general_tag, general_tag| {
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
      move(width, height, world, moved, output, pos, next_ant_pos);
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
          move(width, height, world, moved, output, pos, prev);
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
          move(width, height, world, moved, output, pos, prev);
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
  let n_pos1_water = is_water_or_food(world[n_pos1]);
  let s_pos1_water = is_water_or_food(world[s_pos1]);
  let w_pos1_water = is_water_or_food(world[w_pos1]);
  let e_pos1_water = is_water_or_food(world[e_pos1]);
  let n_pos2_water = is_water_or_food(world[n_pos2]);
  let s_pos2_water = is_water_or_food(world[s_pos2]);
  let w_pos2_water = is_water_or_food(world[w_pos2]);
  let e_pos2_water = is_water_or_food(world[e_pos2]);
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

fn find_near_ants<T: MutableSeq<uint>>(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>, groups: &mut Vec<uint>, group_index: uint, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, group: &mut T, ours: bool) {
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, prev, _, _| {
    if (*groups)[pos] == 0 && !moved[pos] {
      match (*world)[pos] {
        Ant(0) | AnthillWithAnt(0) => {
          if ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world) {
            group.push(pos);
            *groups.get_mut(pos) = group_index;
          }
        },
        Ant(_) | AnthillWithAnt(_) => {
          if !ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world) {
            group.push(pos);
            *groups.get_mut(pos) = group_index;
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

fn get_group<T: MutableSeq<uint>>(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>, groups: &mut Vec<uint>, group_index: uint, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, ours: &mut T, enemies: &mut T) {
  ours.clear();
  enemies.clear();
  let mut ours_q = DList::new();
  let mut enemies_q = DList::new();
  ours_q.push(ant_pos);
  *groups.get_mut(ant_pos) = group_index;
  while !ours_q.is_empty() || !enemies_q.is_empty() {
    if !ours_q.is_empty() {
      let pos = ours_q.pop_front().unwrap();
      ours.push(pos);
      find_near_ants(width, height, pos, attack_radius2, world, moved, groups, group_index, tags, tagged, &mut enemies_q, false);
    }
    if !enemies_q.is_empty() {
      let pos = enemies_q.pop_front().unwrap();
      enemies.push(pos);
      find_near_ants(width, height, pos, attack_radius2, world, moved, groups, group_index, tags, tagged, &mut ours_q, true);
    }
  }
}

fn is_dead(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, board: &Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>) -> bool {
  let mut result = false;
  let attack_value = board[ant_pos].attack;
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| { euclidean(width, height, ant_pos, pos) <= attack_radius2 }, |pos, _, _| {
    if board[pos].attack < attack_value {
      result = true;
      true
    } else {
      false
    }
  });
  clear_tags(tags, tagged);
  result
}

fn estimate(width: uint, height: uint, world: &Vec<Cell>, attack_radius2: uint, ants: &DList<uint>, board: &mut Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>) -> int { //TODO: для оптимизации юзать danger_place.
  let mut ours_dead_count = 0u;
  let mut enemies_dead_count = 0u;
  for &ant_pos in ants.iter() {
    let ant_board_cell = (*board)[ant_pos];
    if ant_board_cell.ant == 0 {
      continue;
    }
    let ant_number = ant_board_cell.ant;
    simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
        let board_cell = board.get_mut(pos);
        if board_cell.ant != ant_number {
          board_cell.attack += 1;
        }
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  for &ant_pos in ants.iter() {
    let ant_board_cell = (*board)[ant_pos];
    if ant_board_cell.ant == 0 {
      continue;
    }
    if is_dead(width, height, ant_pos, attack_radius2, board, tags, tagged) {
      if ant_board_cell.ant == 1 {
        ours_dead_count += 1;
      } else {
        enemies_dead_count += 1;
      }
    }
  }
  for &ant_pos in ants.iter() {
    board.get_mut(ant_pos).attack = 0;
  }
  (enemies_dead_count * ENEMIES_DEAD_ESTIMATION_CONST) as int - (ours_dead_count * OURS_DEAD_ESTIMATION_CONST) as int
}

fn get_chain_begin(mut pos: uint, board: &Vec<BoardCell>) -> uint {
  loop {
    let next_pos = board[pos].cycle;
    if next_pos == 0 {
      break;
    }
    pos = next_pos - 1;
  }
  pos
}

fn get_moves<T: MutableSeq<uint>>(width: uint, height: uint, pos: uint, world: &Vec<Cell>, groups: &Vec<uint>, dangerous_place: &Vec<bool>, board: &Vec<BoardCell>, standing_ants: &Vec<uint>, moves: &mut T) {
  let mut escape = false;
  if board[pos].ant == 0 {
    moves.push(pos);
    if !dangerous_place[pos] {
      escape = true;
    }
  }
  if standing_ants[pos] > STANDING_ANTS_CONST {
    return;
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  let ant_group = groups[pos];
  let n_cell = world[n_pos];
  let chain_begin = get_chain_begin(pos, board);
  if !is_water_or_food(n_cell) && (n_cell != Ant(0) || groups[n_pos] == ant_group) && board[n_pos].ant == 0 && n_pos != chain_begin {
    if !dangerous_place[n_pos] {
      if !escape {
        moves.push(n_pos);
      }
      escape = true;
    } else {
      moves.push(n_pos);
    }
  }
  let w_cell = world[w_pos];
  if !is_water_or_food(w_cell) && (w_cell != Ant(0) || groups[w_pos] == ant_group) && board[w_pos].ant == 0 && w_pos != chain_begin {
    if !dangerous_place[w_pos] {
      if !escape {
        moves.push(w_pos);
      }
      escape = true;
    } else {
      moves.push(w_pos);
    }
  }
  let s_cell = world[s_pos];
  if !is_water_or_food(s_cell) && (s_cell != Ant(0) || groups[s_pos] == ant_group) && board[s_pos].ant == 0 && s_pos != chain_begin {
    if !dangerous_place[s_pos] {
      if !escape {
        moves.push(s_pos);
      }
      escape = true;
    } else {
      moves.push(s_pos);
    }
  }
  let e_cell = world[e_pos];
  if !is_water_or_food(e_cell) && (e_cell != Ant(0) || groups[e_pos] == ant_group) && board[e_pos].ant == 0 && e_pos != chain_begin {
    if !dangerous_place[e_pos] {
      if !escape {
        moves.push(e_pos);
      }
    } else {
      moves.push(e_pos);
    }
  }
}

fn ant_owner(cell: Cell) -> Option<uint> {
  match cell {
    Ant(player) => Some(player),
    AnthillWithAnt(player) => Some(player),
    _ => None
  }
}

fn minimax_min(width: uint, height: uint, idx: uint, moved: &mut DList<uint>, enemies: &Vec<uint>, world: &Vec<Cell>, groups: &Vec<uint>, dangerous_place_for_enemies: &Vec<bool>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, alpha: int) -> int {
  if idx < enemies.len() {
    let pos = enemies[idx];
    let mut moves = DList::new();
    get_moves(width, height, pos, world, groups, dangerous_place_for_enemies, board, standing_ants, &mut moves);
    let mut min_estimate = int::MAX;
    for &next_pos in moves.iter() {
      moved.push(next_pos);
      board.get_mut(next_pos).ant = ant_owner(world[pos]).unwrap() + 1;
      board.get_mut(next_pos).cycle = pos + 1;
      let cur_estimate = minimax_min(width, height, idx + 1, moved, enemies, world, groups, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha);
      board.get_mut(next_pos).ant = 0;
      board.get_mut(next_pos).cycle = 0;
      moved.pop();
      if cur_estimate <= min_estimate {
        min_estimate = cur_estimate;
        if cur_estimate <= alpha {
          break;
        }
      }
    }
    min_estimate
  } else {
    estimate(width, height, world, attack_radius2, moved, board, tags, tagged)
  }
}

fn minimax_max(width: uint, height: uint, idx: uint, moved: &mut DList<uint>, ours: &Vec<uint>, enemies: &Vec<uint>, world: &Vec<Cell>, groups: &Vec<uint>, dangerous_place: &Vec<bool>, dangerous_place_for_enemies: &mut Vec<bool>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, alpha: &mut int, best_moves: &mut Vec<uint>) {
  if idx < ours.len() {
    let pos = ours[idx];
    let mut moves = DList::new();
    get_moves(width, height, pos, world, groups, dangerous_place, board, standing_ants, &mut moves);
    for &next_pos in moves.iter() {
      moved.push(next_pos);
      board.get_mut(next_pos).ant = 1;
      board.get_mut(next_pos).cycle = pos + 1;
      minimax_max(width, height, idx + 1, moved, ours, enemies, world, groups, dangerous_place, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, best_moves);
      board.get_mut(next_pos).ant = 0;
      board.get_mut(next_pos).cycle = 0;
      moved.pop();
    }
  } else {
    for &ant_pos in moved.iter() {
      simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| {
        if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
          *dangerous_place_for_enemies.get_mut(pos) = true;
          true
        } else {
          false
        }
      }, |_, _, _| { false });
      clear_tags(tags, tagged);
    }
    let cur_estimate = minimax_min(width, height, 0, moved, enemies, world, groups, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, *alpha);
    for &ant_pos in moved.iter() {
      simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| {
        if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
          *dangerous_place_for_enemies.get_mut(pos) = false;
          true
        } else {
          false
        }
      }, |_, _, _| { false });
      clear_tags(tags, tagged);
    }
    if cur_estimate > *alpha {
      *alpha = cur_estimate;
      best_moves.clear();
      for &move in moved.iter() {
        best_moves.push(move);
      }
    }
  }
}

fn calculate_dangerous_place(colony: &mut Colony) {
  let width = colony.width;
  let height = colony.height;
  let attack_radius2 = colony.attack_radius2;
  let dangerous_place = &mut colony.dangerous_place;
  for &ant_pos in colony.enemies_ants.iter() {
    simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, prev, _, _| {
      if euclidean(width, height, ant_pos, prev) <= attack_radius2 {
        *dangerous_place.get_mut(pos) = true;
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
}

fn attack<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let mut ours = Vec::new();
  let mut enemies = Vec::new();
  let mut moved = DList::new();
  let mut best_moves = Vec::new();
  let mut group_index = 1;
  for &pos in colony.ours_ants.iter() {
    if colony.groups[pos] != 0 {
      continue;
    }
    get_group(colony.width, colony.height, pos, colony.attack_radius2, &colony.world, &colony.moved, &mut colony.groups, group_index, &mut colony.tags, &mut colony.tagged, &mut ours, &mut enemies);
    group_index += 1;
    if !enemies.is_empty() {
      let mut alpha = int::MIN;
      minimax_max(colony.width, colony.height, 0, &mut moved, &ours, &enemies, &colony.world, &colony.groups, &colony.dangerous_place, &mut colony.dangerous_place_for_enemies, colony.attack_radius2, &mut colony.board, &colony.standing_ants, &mut colony.tags, &mut colony.tagged, &mut alpha, &mut best_moves);
      let mut moves = DList::new();
      for i in range(0u, ours.len()) {
        let pos = ours[i];
        let next_pos = best_moves[i];
        if pos == next_pos {
          *colony.moved.get_mut(pos) = true;
        } else {
          moves.push((pos, next_pos));
        }
      }
      move_all(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, &moves);
    }
  }
}

pub fn turn<'r, T1: Iterator<&'r Input>, T2: MutableSeq<Move>>(colony: &mut Colony, input: &mut T1, output: &mut T2) {
  colony.start_time = get_time();
  output.clear();
  colony.cur_turn += 1;
  update_world(colony, input);
  calculate_dangerous_place(colony);
  attack(colony, output);
  attack_anthills(colony, output);
  gather_food(colony, output);
  discover(colony, output);
  travel(colony, output);
}
