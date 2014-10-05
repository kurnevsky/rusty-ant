//TODO: учет времени.
//TODO: убегание, если в группе один свой муравей.
//TODO: защита муравейников.
//TODO: аггрессивность, если игра долго идет, а противник известен только один.
//TODO: собирать еду перед сражением.

use std::{int, uint};
use std::collections::*;
use std::rand::*;
use coordinates::*;
use time::*;
use wave::*;
use cell::*;
use move::*;
use input::*;

static TERRITORY_PATH_SIZE_CONST: uint = 5;

static GATHERING_FOOD_PATH_SIZE: uint = 30; // Максимальное манхэттенское расстояние до еды от ближайшего муравья, при котором этот муравей за ней побежит.

static ATTACK_ANTHILLS_PATH_SIZE: uint = 10; // Максимальное манхэттенское расстояние до вражеского муравейника от ближайшего муравья, при котором этот муравей за побежит к нему.

static MINIMAX_CRITICAL_TIME: uint = 100;

static CRITICAL_TIME: uint = 50;

static ENEMIES_DEAD_ESTIMATION_CONST: uint = 4000; // На эту константу умножается количество убитых вражеских муравьев при оценке позиции.

static OURS_DEAD_ESTIMATION_CONST: uint = 5000;

static OUR_FOOD_ESTIMATION_CONST: uint = 2000;

static ENEMY_FOOD_ESTIMATION_CONST: uint = 1000;

static DESTROYED_ENEMY_ANTHILL_ESTIMATION_CONST: uint = 30000;

static DESTROYED_OUR_ANTHILL_ESTIMATION_CONST: uint = 30000;

static DISTANCE_TO_ENEMIES_ESTIMATION_CONST: uint = 1;

static ENEMIES_DEAD_AGGRESSIVE_ESTIMATION_CONST: uint = 5000; // На эту константу умножается количество убитых вражеских муравьев при оценке позиции, если группа агрессивна.

static OURS_DEAD_AGGRESSIVE_ESTIMATION_CONST: uint = 3000;

static OUR_FOOD_AGGRESSIVE_ESTIMATION_CONST: uint = 2000;

static ENEMY_FOOD_AGGRESSIVE_ESTIMATION_CONST: uint = 1000;

static DESTROYED_ENEMY_ANTHILL_AGGRESSIVE_ESTIMATION_CONST: uint = 30000;

static DESTROYED_OUR_ANTHILL_AGGRESSIVE_ESTIMATION_CONST: uint = 30000;

static DISTANCE_TO_ENEMIES_AGGRESSIVE_ESTIMATION_CONST: uint = 1;

static STANDING_ANTS_CONST: uint = 4; // Если муравей находится на одном месте дольше этого числа ходов, считаем, что он и дальше будет стоять.

static NEIGHBORS_FOR_AGGRESSIVE: uint = 6; // Количество соседей (включая диагональных), при котором наш муравей считается агрессивным, а с ним и вся группа.

static OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE: uint = 4; // Максимальное манхэттенское расстояние от нашего муравейника до нашего муравья, при котором он считается агрессивным, а с ним и вся группа.

#[deriving(Clone)]
struct BoardCell {
  ant: uint, // Номер игрока, чей муравей сделал ход в текущую клетку, плюс один.
  attack: uint, // Количество врагов, атакующих муравья.
  cycle: uint, // Клитка, из которой сделал ход муравей в текущую. Нужно для отсечения циклов (хождений муравьев по кругу).
  dead: bool // Помечаются флагом погибшие в битве свои и чужие муравьи.
}

pub struct Colony {
  pub width: uint, // Ширина поля.
  pub height: uint, // Высота поля.
  pub turn_time: uint,
  pub turns_count: uint,
  pub view_radius2: uint,
  pub attack_radius2: uint,
  pub spawn_radius2: uint,
  pub cur_turn: uint,
  start_time: u64, // Время начала хода.
  rng: XorShiftRng,
  min_view_radius_manhattan: uint,
  max_view_radius_manhattan: uint,
  enemies_count: uint, // Известное количество врагов.
  world: Vec<Cell>, // Текущее состояние мира. При ходе нашего муравья он передвигается на новую клетку.
  last_world: Vec<Cell>, // Предыдущее состояние мира со сделавшими ход нашими муравьями.
  visible_area: Vec<uint>, // Равняется 0 для видимых клеток и известной воды, для остальных увеличивается на 1 перед каждым ходом.
  discovered_area: Vec<uint>,
  standing_ants: Vec<uint>, // Каждый ход увеличивается на 1 для вражеских муравьев и сбрасывается в 0 для всех остальных клеток. То есть показывает, сколько ходов на месте стоит вражеский муравей.
  moved: Vec<bool>, // Помечаются флагом клетки, откуда сделал ход наш муравей, а также куда он сделал ход.
  gathered_food: Vec<uint>, // Помечается флагом клетки с едой, к которым отправлен наш муравей. Значение - позиция муравья + 1.
  territory: Vec<uint>,
  dangerous_place: Vec<bool>, // Помечаются флагом клетки которые либо под атакой врага, либо которые он может атаковать за один ход.
  dangerous_place_for_enemies: Vec<bool>,
  aggressive_place: Vec<bool>,
  groups: Vec<uint>,
  board: Vec<BoardCell>,
  tags: Vec<Tag>, // Тэги для волнового алгоритма.
  tagged: DList<uint>, // Список позиций start_tags и path_tags с ненулевыми значениями.
  ours_ants: DList<uint>, // Список наших муравьев. Если муравей сделал ход, позиция помечена в moved.
  enemies_ants: DList<uint>,
  enemies_anthills: DList<uint>,
  ours_anthills: DList<uint>,
  food: DList<uint> // Список клеток с едой (как в видимой области, так и за туманом войны, если видели там еду раньше).
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
      min_view_radius_manhattan: (view_radius2 as f32).sqrt() as uint,
      max_view_radius_manhattan: ((view_radius2 * 2) as f32).sqrt() as uint,
      enemies_count: 0,
      world: Vec::from_elem(len, Unknown),
      last_world: Vec::from_elem(len, Unknown),
      visible_area: Vec::from_elem(len, 0u),
      discovered_area: Vec::from_elem(len, 0u),
      standing_ants: Vec::from_elem(len, 0u),
      moved: Vec::from_elem(len, false),
      gathered_food: Vec::from_elem(len, 0u),
      territory: Vec::from_elem(len, 0u),
      dangerous_place: Vec::from_elem(len, false),
      dangerous_place_for_enemies: Vec::from_elem(len, false),
      aggressive_place: Vec::from_elem(len, false),
      groups: Vec::from_elem(len, 0u),
      board: Vec::from_elem(len, BoardCell { ant: 0, attack: 0, cycle: 0, dead: false }),
      tags: Vec::from_elem(len, Tag::new()),
      tagged: DList::new(),
      ours_ants: DList::new(),
      enemies_ants: DList::new(),
      enemies_anthills: DList::new(),
      ours_anthills: DList::new(),
      food: DList::new()
    }
  }
}

fn remove_ant(world: &mut Vec<Cell>, pos: uint) {
  *world.get_mut(pos) = match (*world)[pos] {
    AnthillWithAnt(player) => Anthill(player),
    _ => Land
  };
}

fn set_ant(world: &mut Vec<Cell>, pos: uint, player: uint) {
  *world.get_mut(pos) = if (*world)[pos] == Anthill(player) { AnthillWithAnt(player) } else { Ant(player) };
}

fn move<T: MutableSeq<Move>>(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut T, pos: uint, next_pos: uint) {
  remove_ant(world, pos);
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
    set_ant(world, next_pos, 0);
    *world.get_mut(next_pos) = if (*world)[next_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
    *moved.get_mut(next_pos) = true;
    output.push(Move { point: from_pos(width, pos), direction: to_direction(width, height, pos, next_pos).unwrap() });
  }
}

fn discover_direction(width: uint, height: uint, min_view_radius_manhattan: uint, world: &Vec<Cell>, discovered_area: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, ant_pos: uint) -> Option<uint> {
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
      if pos == s(width, height, prev) || path_size > manhattan(width, height, n_pos, pos) || manhattan(width, height, n_pos, pos) > min_view_radius_manhattan || world[pos] == Water {
        false
      } else {
        if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
          n_score += discovered_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  if is_free(world[s_pos]) {
    simple_wave(width, height, tags, tagged, s_pos, |pos, path_size, prev, _, _| {
      if pos == n(width, height, prev) || path_size > manhattan(width, height, s_pos, pos) || manhattan(width, height, s_pos, pos) > min_view_radius_manhattan || world[pos] == Water {
        false
      } else {
        if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
          s_score += discovered_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  if is_free(world[w_pos]) {
    simple_wave(width, height, tags, tagged, w_pos, |pos, path_size, prev, _, _| {
      if pos == e(width, prev) || path_size > manhattan(width, height, w_pos, pos) || manhattan(width, height, w_pos, pos) > min_view_radius_manhattan || world[pos] == Water {
        false
      } else {
        if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
          w_score += discovered_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  if is_free(world[e_pos]) {
    simple_wave(width, height, tags, tagged, e_pos, |pos, path_size, prev, _, _| {
      if pos == w(width, prev) || path_size > manhattan(width, height, e_pos, pos) || manhattan(width, height, e_pos, pos) > min_view_radius_manhattan || world[pos] == Water {
        false
      } else {
        if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
          e_score += discovered_area[pos];
        }
        true
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
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

fn discover<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let width = colony.width;
  let height = colony.height;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let discovered_area = &mut colony.discovered_area;
  for &pos in colony.ours_ants.iter() {
    if colony.moved[pos] {
      continue;
    }
    match discover_direction(width, height, min_view_radius_manhattan, &colony.world, discovered_area, &mut colony.tags, &mut colony.tagged, pos) {
      Some(next_pos) => {
        simple_wave(width, height, &mut colony.tags, &mut colony.tagged, next_pos, |pos, _, _, _, _| {
          if manhattan(width, height, pos, next_pos) <= min_view_radius_manhattan {
            *discovered_area.get_mut(pos) = 0;
            true
          } else {
            false
          }
        }, |_, _, _| { false });
        clear_tags(&mut colony.tags, &mut colony.tagged);
        move(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, pos, next_pos);
      },
      None => { }
    }
  }
}

fn travel<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let territory = &mut colony.territory;
  let territory_path_size = colony.max_view_radius_manhattan + TERRITORY_PATH_SIZE_CONST;
  let moved = &mut colony.moved;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_ants.iter().chain(colony.enemies_ants.iter()).chain(colony.enemies_anthills.iter()), |pos, start_pos, path_size, _, _, _| {
    if path_size <= territory_path_size && (*world)[pos] != Water {
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
      if cell == Water || prev_general_tag_or_start && ((*moved)[pos] && is_players_ant(cell, 0) || cell == Food) {
        false
      } else {
        *general_tag = if prev_general_tag_or_start && is_players_ant(cell, 0) { 1 } else { 0 };
        true
      }
    }, |pos, _, _| { (*territory)[pos] != 1 });
    if goal.is_none() {
      clear_tags(&mut colony.tags, &mut colony.tagged);
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
  let dangerous_place = &colony.dangerous_place;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.enemies_anthills.iter(), |pos, start_pos, path_size, prev, _, _| {
    if dangerous_place[pos] {
      return false;
    }
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
  let dangerous_place = &colony.dangerous_place;
  for &pos in colony.ours_ants.iter() {
    if (*moved)[pos] || dangerous_place[pos] {
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
    if dangerous_place[pos] {
      return false;
    }
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
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn in_one_group(width: uint, height: uint, pos1: uint, pos2: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>) -> bool {
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
  let n_pos1_cell = world[n_pos1];
  let s_pos1_cell = world[s_pos1];
  let w_pos1_cell = world[w_pos1];
  let e_pos1_cell = world[e_pos1];
  let n_pos2_cell = world[n_pos2];
  let s_pos2_cell = world[s_pos2];
  let w_pos2_cell = world[w_pos2];
  let e_pos2_cell = world[e_pos2];
  let n_pos1_busy = is_water_or_food(n_pos1_cell) || is_players_ant(n_pos1_cell, 0) && moved[n_pos1];
  let s_pos1_busy = is_water_or_food(s_pos1_cell) || is_players_ant(s_pos1_cell, 0) && moved[s_pos1];
  let w_pos1_busy = is_water_or_food(w_pos1_cell) || is_players_ant(w_pos1_cell, 0) && moved[w_pos1];
  let e_pos1_busy = is_water_or_food(e_pos1_cell) || is_players_ant(e_pos1_cell, 0) && moved[e_pos1];
  let n_pos2_busy = is_water_or_food(n_pos2_cell) || is_players_ant(n_pos2_cell, 0) && moved[n_pos2];
  let s_pos2_busy = is_water_or_food(s_pos2_cell) || is_players_ant(s_pos2_cell, 0) && moved[s_pos2];
  let w_pos2_busy = is_water_or_food(w_pos2_cell) || is_players_ant(w_pos2_cell, 0) && moved[w_pos2];
  let e_pos2_busy = is_water_or_food(e_pos2_cell) || is_players_ant(e_pos2_cell, 0) && moved[e_pos2];
  if !n_pos1_busy {
    let n_distance = euclidean(width, height, n_pos1, pos2);
    if n_distance <= attack_radius2 {
      return true;
    }
    if n_distance < distance {
      if !s_pos2_busy && euclidean(width, height, n_pos1, s_pos2) <= attack_radius2 {
        return true;
      }
      if !w_pos2_busy && euclidean(width, height, n_pos1, w_pos2) <= attack_radius2 {
        return true;
      }
      if !e_pos2_busy && euclidean(width, height, n_pos1, e_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  if !s_pos1_busy {
    let s_distance = euclidean(width, height, s_pos1, pos2);
    if s_distance <= attack_radius2 {
      return true;
    }
    if s_distance < distance {
      if !n_pos2_busy && euclidean(width, height, s_pos1, n_pos2) <= attack_radius2 {
        return true;
      }
      if !w_pos2_busy && euclidean(width, height, s_pos1, w_pos2) <= attack_radius2 {
        return true;
      }
      if !e_pos2_busy && euclidean(width, height, s_pos1, e_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  if !w_pos1_busy {
    let w_distance = euclidean(width, height, w_pos1, pos2);
    if w_distance <= attack_radius2 {
      return true;
    }
    if w_distance < distance {
      if !e_pos2_busy && euclidean(width, height, w_pos1, e_pos2) <= attack_radius2 {
        return true;
      }
      if !n_pos2_busy && euclidean(width, height, w_pos1, n_pos2) <= attack_radius2 {
        return true;
      }
      if !s_pos2_busy && euclidean(width, height, w_pos1, s_pos2) <= attack_radius2 {
        return true;
      }
    }
  }
  if !e_pos1_busy {
    let e_distance = euclidean(width, height, e_pos1, pos2);
    if e_distance <= attack_radius2 {
      return true;
    }
    if e_distance < distance {
      if !w_pos2_busy && euclidean(width, height, e_pos1, w_pos2) <= attack_radius2 {
        return true;
      }
      if !n_pos2_busy && euclidean(width, height, e_pos1, n_pos2) <= attack_radius2 {
        return true;
      }
      if !s_pos2_busy && euclidean(width, height, e_pos1, s_pos2) <= attack_radius2 {
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
          if ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world, moved) {
            group.push(pos);
            *groups.get_mut(pos) = group_index;
          }
        },
        Ant(_) | AnthillWithAnt(_) => {
          if !ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world, moved) {
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

fn group_enough(ours_moves_count: uint, enemies_count: uint) -> bool {
  ours_moves_count > 21 ||
  ours_moves_count > 15 && enemies_count > 6 ||
  ours_moves_count > 11 && enemies_count > 7
}

fn get_group(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>, dangerous_place: &Vec<bool>, standing_ants: &Vec<uint>, groups: &mut Vec<uint>, group_index: uint, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, ours: &mut Vec<uint>, enemies: &mut Vec<uint>) {
  ours.clear();
  enemies.clear();
  let mut ours_moves_count = 0u;
  let mut enemies_count = 0u;
  let mut ours_q = DList::new();
  let mut enemies_q = DList::new();
  ours_q.push(ant_pos);
  *groups.get_mut(ant_pos) = group_index;
  while !ours_q.is_empty() && !group_enough(ours_moves_count, enemies_count) {
    let pos = ours_q.pop_front().unwrap();
    ours.push(pos);
    ours_moves_count += get_moves_count(width, height, pos, world, dangerous_place);
    find_near_ants(width, height, pos, attack_radius2, world, moved, groups, group_index, tags, tagged, &mut enemies_q, false);
    while !enemies_q.is_empty() {
      let pos = enemies_q.pop_front().unwrap();
      enemies.push(pos);
      if standing_ants[pos] <= STANDING_ANTS_CONST {
        enemies_count += 1;
      }
      find_near_ants(width, height, pos, attack_radius2, world, moved, groups, group_index, tags, tagged, &mut ours_q, true);
    }
  }
  for &pos in ours_q.iter().chain(enemies.iter()) {
    *groups.get_mut(pos) = 0;
  }
}

fn is_near_food(width: uint, height: uint, world: &Vec<Cell>, pos: uint) -> bool {
  if world[n(width, height, pos)] == Food ||
     world[s(width, height, pos)] == Food ||
     world[w(width, pos)] == Food ||
     world[e(width, pos)] == Food {
    true
  } else {
    false
  }
}

fn is_dead(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, board: &Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>) -> bool {
  let mut result = false;
  let attack_value = board[ant_pos].attack;
  let ant_number = board[ant_pos].ant;
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| { euclidean(width, height, ant_pos, pos) <= attack_radius2 }, |pos, _, _| {
    let board_cell = board[pos];
    if board_cell.ant != 0 && board_cell.ant != ant_number && board_cell.attack <= attack_value {
      result = true;
      true
    } else {
      false
    }
  });
  clear_tags(tags, tagged);
  result
}

fn estimate(width: uint, height: uint, world: &Vec<Cell>, attack_radius2: uint, ants: &DList<uint>, board: &mut Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, aggressive: bool) -> int { //TODO: для оптимизации юзать dangerous_place.
  let mut other_ours = DList::new();
  let mut ours_dead_count = 0u;
  let mut enemies_dead_count = 0u;
  let mut our_food = 0u;
  let mut enemy_food = 0u;
  let mut destroyed_enemy_anthills = 0u;
  let mut destroyed_our_anthills = 0u;
  let mut distance_to_enemies = 0u;
  for &ant_pos in ants.iter() {
    let ant_board_cell = (*board)[ant_pos];
    if ant_board_cell.ant == 0 {
      continue;
    }
    let ant_number = ant_board_cell.ant;
    simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
        let board_cell = board.get_mut(pos);
        if board_cell.ant != 0 {
          if board_cell.ant != ant_number {
            board_cell.attack += 1;
          }
        } else if ant_number != 1 && is_players_ant(world[pos], 0) {
          board_cell.ant = 1;
          other_ours.push(pos);
          board_cell.attack += 1;
        }
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  for &ant_pos in other_ours.iter() {
    simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
        let board_cell = board.get_mut(pos);
        if board_cell.ant > 1 {
          board_cell.attack += 1;
        }
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    if (*board)[ant_pos].ant == 0 {
      continue;
    }
    if is_dead(width, height, ant_pos, attack_radius2, board, tags, tagged) {
      board.get_mut(ant_pos).dead = true;
    }
  }
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    let ant_board_cell = (*board)[ant_pos];
    if ant_board_cell.ant == 0 {
      continue;
    }
    if ant_board_cell.dead {
      if ant_board_cell.ant == 1 {
        ours_dead_count += 1;
      } else {
        enemies_dead_count += 1;
      }
    } else {
      if ant_board_cell.ant == 1 {
        if is_near_food(width, height, world, ant_pos) {
          our_food += 1;
        }
        if is_enemy_anthill(world[ant_pos]) {
          destroyed_enemy_anthills += 1;
        }
        let mut min_distance_to_enemy = uint::MAX;
        for &enemy_pos in ants.iter() {
          let enemy_board_cell = (*board)[enemy_pos];
          if enemy_board_cell.ant < 2 || enemy_board_cell.dead {
            continue;
          }
          let cur_distance = euclidean(width, height, ant_pos, enemy_pos);
          if cur_distance < min_distance_to_enemy {
            min_distance_to_enemy = cur_distance;
          }
        }
        if min_distance_to_enemy != uint::MAX {
          distance_to_enemies += min_distance_to_enemy;
        }
      } else {
        if is_near_food(width, height, world, ant_pos) {
          enemy_food += 1;
        }
        if is_our_anthill(world[ant_pos]) {
          destroyed_our_anthills += 1;
        }
      }
    }
  }
  for &ant_pos in ants.iter() {
    board.get_mut(ant_pos).attack = 0;
    board.get_mut(ant_pos).dead = false;
  }
  for &ant_pos in other_ours.iter() {
    board.get_mut(ant_pos).ant = 0;
    board.get_mut(ant_pos).attack = 0;
    board.get_mut(ant_pos).dead = false;
  }
  let enemies_dead_estimation_conts = if aggressive { ENEMIES_DEAD_AGGRESSIVE_ESTIMATION_CONST } else { ENEMIES_DEAD_ESTIMATION_CONST };
  let ours_dead_estimation_conts = if aggressive { OURS_DEAD_AGGRESSIVE_ESTIMATION_CONST } else { OURS_DEAD_ESTIMATION_CONST };
  let our_food_estimation_const = if aggressive { OUR_FOOD_AGGRESSIVE_ESTIMATION_CONST } else { OUR_FOOD_ESTIMATION_CONST };
  let enemy_food_estimation_const = if aggressive { ENEMY_FOOD_AGGRESSIVE_ESTIMATION_CONST } else { ENEMY_FOOD_ESTIMATION_CONST };
  let destroyed_enemy_anthill_estimation_const = if aggressive { DESTROYED_ENEMY_ANTHILL_AGGRESSIVE_ESTIMATION_CONST } else { DESTROYED_ENEMY_ANTHILL_ESTIMATION_CONST };
  let destroyed_our_anthill_estimation_const = if aggressive { DESTROYED_OUR_ANTHILL_AGGRESSIVE_ESTIMATION_CONST } else { DESTROYED_OUR_ANTHILL_ESTIMATION_CONST };
  let distance_to_enemies_estimation_const = if aggressive { DISTANCE_TO_ENEMIES_AGGRESSIVE_ESTIMATION_CONST } else { DISTANCE_TO_ENEMIES_ESTIMATION_CONST };
  (enemies_dead_count * enemies_dead_estimation_conts) as int - (ours_dead_count * ours_dead_estimation_conts) as int +
  (our_food * our_food_estimation_const) as int - (enemy_food * enemy_food_estimation_const) as int +
  (destroyed_enemy_anthills * destroyed_enemy_anthill_estimation_const) as int - (destroyed_our_anthills * destroyed_our_anthill_estimation_const) as int -
  (distance_to_enemies * distance_to_enemies_estimation_const) as int
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

fn get_moves_count(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<bool>) -> uint {
  let mut result = 1u;
  let mut escape = false;
  if !dangerous_place[pos] {
    escape = true;
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  if !is_water_or_food(world[n_pos]) {
    if !dangerous_place[n_pos] {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[w_pos]) {
    if !dangerous_place[w_pos] {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[s_pos]) {
    if !dangerous_place[s_pos] {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[e_pos]) {
    if !dangerous_place[e_pos] {
      if !escape {
        result += 1;
      }
    } else {
      result += 1;
    }
  }
  result
}

fn get_escape_moves_count(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<bool>) -> uint {
  let mut result = 0u;
  if !dangerous_place[pos] {
    result += 1;
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  if !is_water_or_food(world[n_pos]) && !dangerous_place[n_pos] {
    result += 1;
  }
  if !is_water_or_food(world[w_pos]) && !dangerous_place[w_pos] {
    result += 1;
  }
  if !is_water_or_food(world[s_pos]) && !dangerous_place[s_pos] {
    result += 1;
  }
  if !is_water_or_food(world[e_pos]) && !dangerous_place[e_pos] {
    result += 1;
  }
  result
}

fn get_our_moves<T: MutableSeq<uint>>(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<bool>, board: &Vec<BoardCell>, moves: &mut T) {
  let mut escape = false;
  if board[pos].ant == 0 {
    moves.push(pos);
    if !dangerous_place[pos] {
      escape = true;
    }
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  let n_cell = world[n_pos];
  let chain_begin = get_chain_begin(pos, board);
  if !is_water_or_food(n_cell) && !is_ant(n_cell) && board[n_pos].ant == 0 && n_pos != chain_begin {
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
  if !is_water_or_food(w_cell) && !is_ant(w_cell) && board[w_pos].ant == 0 && w_pos != chain_begin {
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
  if !is_water_or_food(s_cell) && !is_ant(s_cell) && board[s_pos].ant == 0 && s_pos != chain_begin {
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
  if !is_water_or_food(e_cell) && !is_ant(e_cell) && board[e_pos].ant == 0 && e_pos != chain_begin {
    if !dangerous_place[e_pos] {
      if !escape {
        moves.push(e_pos);
      }
    } else {
      moves.push(e_pos);
    }
  }
}

fn get_enemy_moves<T: MutableSeq<uint>>(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<bool>, board: &Vec<BoardCell>, standing_ants: &Vec<uint>, moves: &mut T) {
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
  let n_cell = world[n_pos];
  let chain_begin = get_chain_begin(pos, board);
  if !is_water_or_food(n_cell) && !is_players_ant(n_cell, 0) && board[n_pos].ant == 0 && n_pos != chain_begin {
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
  if !is_water_or_food(w_cell) && !is_players_ant(w_cell, 0) && board[w_pos].ant == 0 && w_pos != chain_begin {
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
  if !is_water_or_food(s_cell) && !is_players_ant(s_cell, 0) && board[s_pos].ant == 0 && s_pos != chain_begin {
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
  if !is_water_or_food(e_cell) && !is_players_ant(e_cell, 0) && board[e_pos].ant == 0 && e_pos != chain_begin {
    if !dangerous_place[e_pos] {
      if !escape {
        moves.push(e_pos);
      }
    } else {
      moves.push(e_pos);
    }
  }
}

fn minimax_min(width: uint, height: uint, idx: uint, minimax_moved: &mut DList<uint>, enemies: &Vec<uint>, world: &Vec<Cell>, dangerous_place_for_enemies: &Vec<bool>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, alpha: int, start_time: u64, turn_time: uint, aggressive: bool) -> int {
  if idx < enemies.len() {
    let pos = enemies[idx];
    let mut moves = DList::new();
    get_enemy_moves(width, height, pos, world, dangerous_place_for_enemies, board, standing_ants, &mut moves);
    let mut min_estimate = int::MAX;
    for &next_pos in moves.iter() {
      if elapsed_time(start_time) + MINIMAX_CRITICAL_TIME > turn_time { return int::MIN; }
      minimax_moved.push(next_pos);
      board.get_mut(next_pos).ant = ant_owner(world[pos]).unwrap() + 1;
      board.get_mut(next_pos).cycle = pos + 1;
      let cur_estimate = minimax_min(width, height, idx + 1, minimax_moved, enemies, world, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, start_time, turn_time, aggressive);
      board.get_mut(next_pos).ant = 0;
      board.get_mut(next_pos).cycle = 0;
      minimax_moved.pop();
      if cur_estimate < min_estimate {
        min_estimate = cur_estimate;
        if cur_estimate <= alpha {
          break;
        }
      }
    }
    min_estimate
  } else {
    estimate(width, height, world, attack_radius2, minimax_moved, board, tags, tagged, aggressive)
  }
}

fn minimax_max(width: uint, height: uint, idx: uint, minimax_moved: &mut DList<uint>, ours: &Vec<uint>, enemies: &mut Vec<uint>, world: &Vec<Cell>, dangerous_place: &Vec<bool>, dangerous_place_for_enemies: &mut Vec<bool>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut DList<uint>, alpha: &mut int, aggressive: bool, start_time: u64, turn_time: uint, best_moves: &mut Vec<uint>) {
  if idx < ours.len() {
    let pos = ours[idx];
    let mut moves = DList::new();
    get_our_moves(width, height, pos, world, dangerous_place, board, &mut moves);
    for &next_pos in moves.iter() {
      if elapsed_time(start_time) + MINIMAX_CRITICAL_TIME > turn_time { return; }
      minimax_moved.push(next_pos);
      board.get_mut(next_pos).ant = 1;
      board.get_mut(next_pos).cycle = pos + 1;
      minimax_max(width, height, idx + 1, minimax_moved, ours, enemies, world, dangerous_place, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, aggressive, start_time, turn_time, best_moves);
      board.get_mut(next_pos).ant = 0;
      board.get_mut(next_pos).cycle = 0;
      minimax_moved.pop();
    }
  } else {
    for &ant_pos in minimax_moved.iter() {
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
    enemies.sort_by(|&pos1, &pos2|
      if (*dangerous_place_for_enemies)[pos1] && !(*dangerous_place_for_enemies)[pos2] {
        Less
      } else {
        get_escape_moves_count(width, height, pos1, world, dangerous_place_for_enemies).cmp(&get_escape_moves_count(width, height, pos2, world, dangerous_place_for_enemies))
      }
    );
    let cur_estimate = minimax_min(width, height, 0, minimax_moved, enemies, world, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, *alpha, start_time, turn_time, aggressive);
    for &ant_pos in minimax_moved.iter() {
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
      for &move in minimax_moved.iter() {
        best_moves.push(move);
      }
    }
  }
}

fn attack<T: MutableSeq<Move>>(colony: &mut Colony, output: &mut T) {
  let mut ours = Vec::new();
  let mut enemies = Vec::new();
  let mut minimax_moved = DList::new();
  let mut best_moves = Vec::new();
  let mut group_index = 1;
  for &pos in colony.ours_ants.iter() {
    if elapsed_time(colony.start_time) + MINIMAX_CRITICAL_TIME > colony.turn_time { return; }
    if colony.groups[pos] != 0 {
      continue;
    }
    get_group(colony.width, colony.height, pos, colony.attack_radius2, &colony.world, &colony.moved, &colony.dangerous_place, &colony.standing_ants, &mut colony.groups, group_index, &mut colony.tags, &mut colony.tagged, &mut ours, &mut enemies);
    group_index += 1;
    if !enemies.is_empty() {
      let mut alpha = int::MIN;
      let mut aggressive = false;
      for &pos in ours.iter() {
        if colony.aggressive_place[pos] {
          aggressive = true;
          break;
        }
      }
      ours.sort_by(|&pos1, &pos2|
        if colony.dangerous_place[pos1] && !colony.dangerous_place[pos2] {
          Less
        } else {
          get_escape_moves_count(colony.width, colony.height, pos1, &colony.world, &colony.dangerous_place).cmp(&get_escape_moves_count(colony.width, colony.height, pos2, &colony.world, &colony.dangerous_place))
        }
      );
      for &pos in ours.iter() {
        remove_ant(&mut colony.world, pos);
      }
      minimax_max(colony.width, colony.height, 0, &mut minimax_moved, &ours, &mut enemies, &colony.world, &colony.dangerous_place, &mut colony.dangerous_place_for_enemies, colony.attack_radius2, &mut colony.board, &colony.standing_ants, &mut colony.tags, &mut colony.tagged, &mut alpha, aggressive, colony.start_time, colony.turn_time, &mut best_moves);
      for &pos in ours.iter() {
        set_ant(&mut colony.world, pos, 0);
      }
      if alpha != int::MIN {
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
}

fn calculate_aggressive_place(colony: &mut Colony) {
  let aggressive_place = &mut colony.aggressive_place;
  for &pos in colony.ours_ants.iter() {
    let mut neighbors = 0;
    if colony.world[n(colony.width, colony.height, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[w(colony.width, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[s(colony.width, colony.height, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[e(colony.width, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[nw(colony.width, colony.height, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[ne(colony.width, colony.height, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[sw(colony.width, colony.height, pos)] == Ant(0) {
      neighbors += 1;
    }
    if colony.world[se(colony.width, colony.height, pos)] == Ant(0) {
      neighbors += 1;
    }
    if neighbors >= NEIGHBORS_FOR_AGGRESSIVE {
      *aggressive_place.get_mut(pos) = true;
    }
  }
  wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_anthills.iter(), |pos, _, path_size, _, _, _| {
    if path_size <= OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE {
      *aggressive_place.get_mut(pos) = true;
      true
    } else {
      false
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
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

fn update_world<'r, T: Iterator<&'r Input>>(colony: &mut Colony, input: &mut T) {
  let view_radius2 = colony.view_radius2;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let width = colony.width;
  let height = colony.height;
  let visible_area = &mut colony.visible_area;
  let discovered_area = &mut colony.discovered_area;
  let len = length(width, height);
  for pos in range(0u, len) {
    *colony.last_world.get_mut(pos) = colony.world[pos];
    *colony.world.get_mut(pos) = Unknown;
    *colony.moved.get_mut(pos) = false;
    *colony.gathered_food.get_mut(pos) = 0;
    *visible_area.get_mut(pos) += 1;
    *discovered_area.get_mut(pos) += 1;
    *colony.territory.get_mut(pos) = 0;
    *colony.groups.get_mut(pos) = 0;
    *colony.dangerous_place.get_mut(pos) = false;
    *colony.aggressive_place.get_mut(pos) = false;
  }
  colony.ours_ants.clear();
  colony.enemies_ants.clear();
  colony.enemies_anthills.clear();
  colony.ours_anthills.clear();
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
        if player == 0 {
          colony.ours_anthills.push(pos);
        } else {
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
      if euclidean(width, height, pos, ant_pos) <= view_radius2 {
        if manhattan(width, height, pos, ant_pos) <= min_view_radius_manhattan {
          *discovered_area.get_mut(pos) = 0;
        }
        *visible_area.get_mut(pos) = 0;
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
  for pos in range(0u, len) {
    if (*visible_area)[pos] == 0 {
      if colony.world[pos] == Unknown {
        *colony.world.get_mut(pos) = match colony.last_world[pos] {
          Water => Water,
          _ => Land
        }
      }
      match colony.world[pos] {
        Ant(player) if player > 0 => *colony.standing_ants.get_mut(pos) += 1,
        AnthillWithAnt(player) if player > 0 => *colony.standing_ants.get_mut(pos) += 1,
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
          colony.ours_anthills.push(pos);
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

pub fn turn<'r, T1: Iterator<&'r Input>, T2: MutableSeq<Move>>(colony: &mut Colony, input: &mut T1, output: &mut T2) {
  colony.start_time = get_time();
  output.clear();
  colony.cur_turn += 1;
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  update_world(colony, input);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  calculate_dangerous_place(colony);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  calculate_aggressive_place(colony);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  attack_anthills(colony, output);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  gather_food(colony, output);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  attack(colony, output);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  discover(colony, output);
  if elapsed_time(colony.start_time) + CRITICAL_TIME > colony.turn_time { return; }
  travel(colony, output);
}
