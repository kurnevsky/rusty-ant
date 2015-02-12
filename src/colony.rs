//TODO: аггрессивность, если игра долго идет, а противник известен только один.
//TODO: динамический подбор констатнт минимакса путем определения производительности на этапе инициализации. Динамическое уменьшение этих констант при таймаутах.
//TODO: захват муравейников вместо уничтожения.
//TODO: три варианта атаки:
//TODO: 1. для каждого противника вычислять минимаксом его осторожный ход (как это сейчас для себя делаем), а затем вычислить свой лучший ход на эти ходы.
//TODO: 2. попыка вычислить итеративно точку равновесие Нэша - для своих стоящих на месте муравьев вычислить лучший ход противника, затем для этого лучшего хода вычислить наш лучший ход, и так далее. Делать так либо пока не достигнут предел итераций либо лучшие ответы перестанут меняться.
//TODO: 3. в минимаксе вычислять не худший для нас, а лучший для врага ход. При этом придется отказаться от альфа-отсечния.

use std::{int, uint, cmp, iter};
use std::num::*;
use std::collections::*;
use rand::*;
use coordinates::*;
use time::*;
use wave::*;
use cell::*;
use step::*;
use input::*;
use log::*;

static TERRITORY_PATH_SIZE_CONST: uint = 5;

static APPROACH_PATH_SIZE_CONST: uint = 6;

static ESCAPE_PATH_SIZE: uint = 8;

static GATHERING_FOOD_PATH_SIZE: uint = 30; // Максимальное манхэттенское расстояние до еды от ближайшего муравья, при котором этот муравей за ней побежит.

static ATTACK_ANTHILLS_PATH_SIZE: uint = 20; // Максимальное манхэттенское расстояние до вражеского муравейника от ближайшего муравья, при котором этот муравей за побежит к нему.

static DEFEND_ANTHILLS_PATH_SIZE: uint = 15;

static DEFENDER_PATH_SIZE: uint = 20;

static ATTACK_ANTHILLS_ANTS_COUNT: uint = 2; // Максимальное количество наших муравьев, атакующих вражеские муравеники.

static DANGEROUS_ANTHILLS_COUNT: uint = 3; // Если муравейников больше этого количества, не защищаем их вообще.

static DEFEND_ANTHILLS_COUNT: uint = 2; // Максимальное количество атакуемых своих муравейников, которые мы будем защищать.

static MINIMAX_CRITICAL_TIME: uint = 100; // Если на ход осталось времени меньше этого количества миллисекунд, останавливаем минимакс и продолжаем вычислять ходы другими методами.

static CRITICAL_TIME: uint = 50; // Если на ход осталось времени меньше этого количества миллисекунд, больше ничего не делаем, а только отдаем уже сделанные ходы.

static ENEMIES_DEAD_ESTIMATION: &'static [uint] = &[3000, 4000, 6000, 9000, 12000, 18000]; // На эту константу умножается количество убитых вражеских муравьев при оценке позиции.

static OURS_DEAD_ESTIMATION: &'static [uint] = &[6000, 6000, 6000, 6000, 6000, 6000]; // На эту константу умножается количество убитых своих муравьев при оценке позиции.

static OUR_FOOD_ESTIMATION: &'static [uint] = &[2000, 2000, 2000, 2000, 2000, 2000]; // На эту константу умножается количество своих муравьев, которые находятся на расстоянии сбора от еды.

static ENEMY_FOOD_ESTIMATION: &'static [uint] = &[1000, 1500, 2000, 3000, 4000, 6000]; // На эту константу умножается количество вражеских муравьев, которые находятся на расстоянии сбора от еды.

static DESTROYED_ENEMY_ANTHILL_ESTIMATION: &'static [uint] = &[50000, 50000, 50000, 50000, 50000, 50000];

static DESTROYED_OUR_ANTHILL_ESTIMATION: &'static [uint] = &[50000, 50000, 50000, 50000, 50000, 50000];

static DISTANCE_TO_ENEMIES_ESTIMATION: &'static [uint] = &[1, 1, 1, 1, 1, 1];

static STANDING_ANTS_CONST: uint = 4; // Если муравей находится на одном месте дольше этого числа ходов, считаем, что он и дальше будет стоять.

static STANDING_ANTS_WITH_CHANGED_ENVIRONMENT_CONST: uint = 4;

static NEIGHBORS_AGGRESSION: &'static [uint] = &[0, 0, 1, 1, 1, 2, 3, 4, 5]; // Уровни агрессии для муравья от числа его соседей.

static OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE: uint = 6; // Максимальное манхэттенское расстояние от нашего муравейника до нашего муравья, при котором он считается агрессивным, а с ним и вся группа.

static OURS_ANTHILLS_AGGRESSION: uint = 2; // Уровень агрессии для наших муравьев, близких к нашим муравейникам.

#[derive(Clone)]
struct BoardCell {
  ant: uint, // Номер игрока, чей муравей сделал ход в текущую клетку, плюс один.
  attack: uint, // Количество врагов, атакующих муравья.
  cycle: uint, // Клитка, из которой сделал ход муравей в текущую. Нужно для отсечения циклов (хождений муравьев по кругу).
  dead: bool // Помечаются флагом погибшие в битве свои и чужие муравьи.
}

pub struct Colony {
  width: uint, // Ширина поля.
  height: uint, // Высота поля.
  turn_time: uint, // Время на один ход.
  turns_count: uint, // Количество ходов в игре.
  view_radius2: uint,
  attack_radius2: uint,
  spawn_radius2: uint,
  cur_turn: uint, // Номер текущего хода.
  start_time: u64, // Время начала хода.
  rng: XorShiftRng, // Генератор случайных чисел.
  min_view_radius_manhattan: uint,
  max_view_radius_manhattan: uint,
  max_attack_radius_manhattan: uint,
  enemies_count: uint, // Известное количество врагов.
  world: Vec<Cell>, // Текущее состояние мира. При ходе нашего муравья он передвигается на новую клетку.
  last_world: Vec<Cell>, // Предыдущее состояние мира со сделавшими ход нашими муравьями.
  visible_area: Vec<uint>, // Равняется 0 для видимых клеток и известной воды, для остальных увеличивается на 1 перед каждым ходом.
  discovered_area: Vec<uint>,
  standing_ants: Vec<uint>, // Каждый ход увеличивается на 1 для вражеских муравьев (при условии, что у них не изменилось окружение) и сбрасывается в 0 для всех остальных клеток. То есть показывает, сколько ходов на месте стоит вражеский муравей.
  moved: Vec<bool>, // Помечаются флагом клетки, откуда сделал ход наш муравей, а также куда он сделал ход.
  gathered_food: Vec<uint>, // Помечается флагом клетки с едой, к которым отправлен наш муравей. Значение - позиция муравья + 1.
  territory: Vec<uint>,
  dangerous_place: Vec<uint>, // Количество вражеских муравьев, которые могут атаковать клетку на следующем ходу.
  aggressive_place: Vec<uint>,
  groups: Vec<uint>,
  fighting: Vec<bool>,
  board: Vec<BoardCell>,
  tmp: Vec<uint>,
  alone_ants: Vec<uint>,
  tags: Vec<Tag>, // Тэги для волнового алгоритма.
  tagged: Vec<uint>, // Список позиций start_tags и path_tags с ненулевыми значениями.
  tags2: Vec<Tag>, // Вторые тэги для волнового алгоритма.
  tagged2: Vec<uint>, // Список позиций start_tags и path_tags во вторых тэгах с ненулевыми значениями.
  ours_ants: Vec<uint>, // Список наших муравьев. Если муравей сделал ход, позиция помечена в moved.
  enemies_ants: Vec<uint>,
  enemies_anthills: Vec<uint>,
  ours_anthills: Vec<uint>, // Список клеток с нашими муравейниками (как в видимой области, так и за туманом войны).
  food: Vec<uint>, // Список клеток с едой (как в видимой области, так и за туманом войны, если видели там еду раньше).
  log: DList<LogMessage>
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
      max_attack_radius_manhattan: ((attack_radius2 * 2) as f32).sqrt() as uint,
      enemies_count: 0,
      world: iter::repeat(Cell::Unknown).take(len).collect(),
      last_world: iter::repeat(Cell::Unknown).take(len).collect(),
      visible_area: iter::repeat(0u).take(len).collect(),
      discovered_area: iter::repeat(0u).take(len).collect(),
      standing_ants: iter::repeat(0u).take(len).collect(),
      moved: iter::repeat(false).take(len).collect(),
      gathered_food: iter::repeat(0u).take(len).collect(),
      territory: iter::repeat(0u).take(len).collect(),
      dangerous_place: iter::repeat(0u).take(len).collect(),
      aggressive_place: iter::repeat(0u).take(len).collect(),
      groups: iter::repeat(0u).take(len).collect(),
      fighting: iter::repeat(false).take(len).collect(),
      board: iter::repeat(BoardCell { ant: 0, attack: 0, cycle: 0, dead: false }).take(len).collect(),
      tmp: iter::repeat(0u).take(len).collect(),
      alone_ants: Vec::with_capacity(len),
      tags: iter::repeat(Tag::new()).take(len).collect(),
      tagged: Vec::with_capacity(len),
      tags2: iter::repeat(Tag::new()).take(len).collect(),
      tagged2: Vec::with_capacity(len),
      ours_ants: Vec::with_capacity(len),
      enemies_ants: Vec::with_capacity(len),
      enemies_anthills: Vec::with_capacity(len),
      ours_anthills: Vec::with_capacity(len),
      food: Vec::with_capacity(len),
      log: DList::new()
    }
  }
  pub fn log(&self) -> &DList<LogMessage> {
    &self.log
  }
  pub fn width(&self) -> uint {
    self.width
  }
  pub fn height(&self) -> uint {
    self.height
  }
  pub fn cur_turn(&self) -> uint {
    self.cur_turn
  }
}

fn remove_ant(world: &mut Vec<Cell>, pos: uint) {
  *world.get_mut(pos).unwrap() = match world[pos] {
    Cell::AnthillWithAnt(player) => Cell::Anthill(player),
    _ => Cell::Land
  };
}

fn set_ant(world: &mut Vec<Cell>, pos: uint, player: uint) {
  *world.get_mut(pos).unwrap() = if world[pos] == Cell::Anthill(player) { Cell::AnthillWithAnt(player) } else { Cell::Ant(player) };
}

fn move_one(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut DList<Step>, pos: uint, next_pos: uint, log: &mut DList<LogMessage>) {
  let direction = to_direction(width, height, pos, next_pos);
  if direction.is_none() {
    log.push_back(LogMessage::Jump(pos, next_pos));
    return;
  }
  if !is_players_ant(world[pos].clone(), 0) {
    log.push_back(LogMessage::Multitask(pos, next_pos));
    return;
  }
  remove_ant(world, pos);
  *moved.get_mut(pos).unwrap() = true;
  set_ant(world, next_pos, 0);
  *moved.get_mut(next_pos).unwrap() = true;
  output.push_back(Step { point: from_pos(width, pos), direction: direction.unwrap() })
}

fn move_all(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut DList<Step>, moves: &DList<(uint, uint)>, log: &mut DList<LogMessage>) {
  for &(pos, next_pos) in moves.iter() {
    if !is_players_ant(world[pos].clone(), 0) {
      log.push_back(LogMessage::Multitask(pos, next_pos));
      continue;
    }
    remove_ant(world, pos);
    *moved.get_mut(pos).unwrap() = true;
  }
  for &(pos, next_pos) in moves.iter() {
    let direction = to_direction(width, height, pos, next_pos);
    if direction.is_none() {
      log.push_back(LogMessage::Jump(pos, next_pos));
      continue;
    }
    set_ant(world, next_pos, 0);
    *moved.get_mut(next_pos).unwrap() = true;
    output.push_back(Step { point: from_pos(width, pos), direction: direction.unwrap() });
  }
}

fn discover_direction(width: uint, height: uint, min_view_radius_manhattan: uint, world: &Vec<Cell>, discovered_area: &Vec<uint>, dangerous_place: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, ant_pos: uint, rng: &mut XorShiftRng) -> Option<uint> {
  let mut n_score = 0u;
  let mut s_score = 0u;
  let mut w_score = 0u;
  let mut e_score = 0u;
  let n_pos = n(width, height, ant_pos);
  let s_pos = s(width, height, ant_pos);
  let w_pos = w(width, ant_pos);
  let e_pos = e(width, ant_pos);
  if is_free(world[n_pos].clone()) && dangerous_place[n_pos] == 0 {
    simple_wave(width, height, tags, tagged, n_pos, |pos, path_size, prev| {
      if pos == s(width, height, prev) || path_size > manhattan(width, height, n_pos, pos) || manhattan(width, height, n_pos, pos) > min_view_radius_manhattan || world[pos] == Cell::Water {
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
  if is_free(world[s_pos].clone()) && dangerous_place[s_pos] == 0 {
    simple_wave(width, height, tags, tagged, s_pos, |pos, path_size, prev| {
      if pos == n(width, height, prev) || path_size > manhattan(width, height, s_pos, pos) || manhattan(width, height, s_pos, pos) > min_view_radius_manhattan || world[pos] == Cell::Water {
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
  if is_free(world[w_pos].clone()) && dangerous_place[w_pos] == 0 {
    simple_wave(width, height, tags, tagged, w_pos, |pos, path_size, prev| {
      if pos == e(width, prev) || path_size > manhattan(width, height, w_pos, pos) || manhattan(width, height, w_pos, pos) > min_view_radius_manhattan || world[pos] == Cell::Water {
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
  if is_free(world[e_pos].clone())  && dangerous_place[e_pos] == 0 {
    simple_wave(width, height, tags, tagged, e_pos, |pos, path_size, prev| {
      if pos == w(width, prev) || path_size > manhattan(width, height, e_pos, pos) || manhattan(width, height, e_pos, pos) > min_view_radius_manhattan || world[pos] == Cell::Water {
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
  if n_score == 0 && s_score == 0 && w_score == 0 && e_score == 0 { // При равенстве можно было бы учитывать расстояние до своих муравьев, однако на это нужно слишком много вычислений, поэтому выбираем случайно.
    None
  } else {
    let mut next_pos = n_pos;
    let mut score = n_score;
    if s_score > score {
      next_pos = s_pos;
      score = s_score;
    } else if s_score == score && rng.gen() {
      next_pos = s_pos;
    }
    if w_score > score {
      next_pos = w_pos;
      score = w_score;
    } else if w_score == score && rng.gen() {
      next_pos = w_pos;
    }
    if e_score > score || e_score == score && rng.gen() {
      next_pos = e_pos;
    }
    Some(next_pos)
  }
}

fn discover(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::Discover);
  let width = colony.width;
  let height = colony.height;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let discovered_area = &mut colony.discovered_area;
  for &pos in colony.ours_ants.iter() {
    if colony.moved[pos] {
      continue;
    }
    match discover_direction(width, height, min_view_radius_manhattan, &colony.world, discovered_area, &colony.dangerous_place, &mut colony.tags, &mut colony.tagged, pos, &mut colony.rng) {
      Some(next_pos) => {
        simple_wave(width, height, &mut colony.tags, &mut colony.tagged, next_pos, |pos, _, _| {
          if manhattan(width, height, pos, next_pos) <= min_view_radius_manhattan {
            *discovered_area.get_mut(pos).unwrap() = 0;
            true
          } else {
            false
          }
        }, |_, _, _| { false });
        clear_tags(&mut colony.tags, &mut colony.tagged);
        move_one(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, pos, next_pos, &mut colony.log);
        colony.log.push_back(LogMessage::Goal(pos, next_pos));
      },
      None => { }
    }
  }
}

fn travel(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::Travel);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let territory = &colony.territory;
  let moved = &mut colony.moved;
  let tmp = &mut colony.tmp;
  let dangerous_place = &colony.dangerous_place;
  let mut path = DList::new();
  for &ant_pos in colony.ours_ants.iter() {
    if moved[ant_pos] {
      continue;
    }
    *tmp.get_mut(ant_pos).unwrap() = 1;
    let goal = simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, prev| {
      let cell = world[pos].clone();
      let is_column = tmp[prev];
      if cell == Cell::Water || is_column == 1 && (moved[pos] && is_players_ant(cell.clone(), 0) || cell == Cell::Food || dangerous_place[pos] > 0 && pos != ant_pos) {
        false
      } else {
        *tmp.get_mut(pos).unwrap() = if is_players_ant(cell, 0) { is_column } else { 0 };
        true
      }
    }, |pos, _, _| { territory[pos] != 1 });
    if goal.is_none() {
      for &pos in colony.tagged.iter() {
        *tmp.get_mut(pos).unwrap() = 0;
      }
      clear_tags(&mut colony.tags, &mut colony.tagged);
      continue;
    }
    find_path(&mut colony.tags, ant_pos, goal.unwrap(), &mut path);
    for &pos in colony.tagged.iter() {
      *tmp.get_mut(pos).unwrap() = 0;
    }
    clear_tags(&mut colony.tags, &mut colony.tagged);
    let mut path_pos = ant_pos;
    let mut moves = DList::new();
    for &pos in path.iter() {
      moves.push_back((path_pos, pos));
      colony.log.push_back(LogMessage::Goal(path_pos, goal.unwrap()));
      if !is_players_ant(world[pos].clone(), 0) {
        break;
      }
      path_pos = pos;
    }
    move_all(width, height, world, moved, output, &moves, &mut colony.log);
  }
}

fn calculate_territory(colony: &mut Colony) {
  let territory_path_size = colony.max_view_radius_manhattan + TERRITORY_PATH_SIZE_CONST;
  let world = &colony.world;
  let territory = &mut colony.territory;
  wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_ants.iter().chain(colony.enemies_ants.iter()).chain(colony.enemies_anthills.iter()), |pos, start_pos, path_size, _| {
    if path_size <= territory_path_size && world[pos] != Cell::Water {
      match world[start_pos] {
        Cell::AnthillWithAnt(player) => *territory.get_mut(pos).unwrap() = player + 1,
        Cell::Ant(player) => *territory.get_mut(pos).unwrap() = player + 1,
        Cell::Anthill(player) => *territory.get_mut(pos).unwrap() = player + 1,
        _ => *territory.get_mut(pos).unwrap() = 1
      }
      true
    } else {
      false
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn attack_anthills(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::AttackAnthills);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  let dangerous_place = &colony.dangerous_place;
  let tmp = &mut colony.tmp;
  let log = &mut colony.log;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.enemies_anthills.iter(), |pos, start_pos, path_size, prev| {
    if pos != start_pos && dangerous_place[pos] > 0 || path_size > ATTACK_ANTHILLS_PATH_SIZE || tmp[start_pos] > ATTACK_ANTHILLS_ANTS_COUNT {
      return false;
    }
    match world[pos] {
      Cell::Ant(0) | Cell::AnthillWithAnt(0) if !moved[pos] => {
        if !is_free(world[prev].clone()) {
          false
        } else {
          *tmp.get_mut(start_pos).unwrap() += 1;
          move_one(width, height, world, moved, output, pos, prev, log);
          log.push_back(LogMessage::Goal(pos, start_pos));
          true
        }
      },
      Cell::Unknown | Cell::Water => false,
      _ => true
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
  for &pos in colony.enemies_anthills.iter() {
    *tmp.get_mut(pos).unwrap() = 0;
  }
}

fn gather_food(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::GatherFood);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let gathered_food = &mut colony.gathered_food;
  let moved = &mut colony.moved;
  let dangerous_place = &colony.dangerous_place;
  let log = &mut colony.log;
  for &pos in colony.ours_ants.iter() {
    if moved[pos] || dangerous_place[pos] > 0 {
      continue;
    }
    let n_pos = n(width, height, pos);
    if world[n_pos] == Cell::Food && gathered_food[n_pos] == 0 {
      *moved.get_mut(pos).unwrap() = true;
      *gathered_food.get_mut(n_pos).unwrap() = pos + 1;
      log.push_back(LogMessage::Goal(pos, n_pos));
    }
    let s_pos = s(width, height, pos);
    if world[s_pos] == Cell::Food && gathered_food[s_pos] == 0 {
      *moved.get_mut(pos).unwrap() = true;
      *gathered_food.get_mut(s_pos).unwrap() = pos + 1;
      log.push_back(LogMessage::Goal(pos, s_pos));
    }
    let w_pos = w(width, pos);
    if world[w_pos] == Cell::Food && gathered_food[w_pos] == 0 {
      *moved.get_mut(pos).unwrap() = true;
      *gathered_food.get_mut(w_pos).unwrap() = pos + 1;
      log.push_back(LogMessage::Goal(pos, w_pos));
    }
    let e_pos = e(width, pos);
    if world[e_pos] == Cell::Food && gathered_food[e_pos] == 0 {
      *moved.get_mut(pos).unwrap() = true;
      *gathered_food.get_mut(e_pos).unwrap() = pos + 1;
      log.push_back(LogMessage::Goal(pos, e_pos));
    }
  }
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.food.iter(), |pos, start_pos, path_size, prev| {
    if pos != start_pos && dangerous_place[pos] > 0 || path_size > GATHERING_FOOD_PATH_SIZE {
      return false;
    }
    match world[pos] {
      Cell::Ant(0) | Cell::AnthillWithAnt(0) if gathered_food[start_pos] == 0 && !moved[pos] => {
        if pos != start_pos && !is_free(world[prev].clone()) {
          false
        } else {
          move_one(width, height, world, moved, output, pos, prev, log);
          *gathered_food.get_mut(start_pos).unwrap() = pos + 1;
          log.push_back(LogMessage::Goal(pos, start_pos));
          true
        }
      },
      Cell::Unknown | Cell::Water => false,
      _ => true
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
  let n_pos1_cell = world[n_pos1].clone();
  let s_pos1_cell = world[s_pos1].clone();
  let w_pos1_cell = world[w_pos1].clone();
  let e_pos1_cell = world[e_pos1].clone();
  let n_pos2_cell = world[n_pos2].clone();
  let s_pos2_cell = world[s_pos2].clone();
  let w_pos2_cell = world[w_pos2].clone();
  let e_pos2_cell = world[e_pos2].clone();
  let n_pos1_busy = is_water_or_food(n_pos1_cell.clone()) || is_players_ant(n_pos1_cell.clone(), 0) && moved[n_pos1];
  let s_pos1_busy = is_water_or_food(s_pos1_cell.clone()) || is_players_ant(s_pos1_cell.clone(), 0) && moved[s_pos1];
  let w_pos1_busy = is_water_or_food(w_pos1_cell.clone()) || is_players_ant(w_pos1_cell.clone(), 0) && moved[w_pos1];
  let e_pos1_busy = is_water_or_food(e_pos1_cell.clone()) || is_players_ant(e_pos1_cell.clone(), 0) && moved[e_pos1];
  let n_pos2_busy = is_water_or_food(n_pos2_cell.clone()) || is_players_ant(n_pos2_cell.clone(), 0) && moved[n_pos2];
  let s_pos2_busy = is_water_or_food(s_pos2_cell.clone()) || is_players_ant(s_pos2_cell.clone(), 0) && moved[s_pos2];
  let w_pos2_busy = is_water_or_food(w_pos2_cell.clone()) || is_players_ant(w_pos2_cell.clone(), 0) && moved[w_pos2];
  let e_pos2_busy = is_water_or_food(e_pos2_cell.clone()) || is_players_ant(e_pos2_cell.clone(), 0) && moved[e_pos2];
  if !n_pos2_busy && euclidean(width, height, pos1, n_pos2) <= attack_radius2 {
    return true;
  }
  if !s_pos2_busy && euclidean(width, height, pos1, s_pos2) <= attack_radius2 {
    return true;
  }
  if !w_pos2_busy && euclidean(width, height, pos1, w_pos2) <= attack_radius2 {
    return true;
  }
  if !e_pos2_busy && euclidean(width, height, pos1, e_pos2) <= attack_radius2 {
    return true;
  }
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

fn find_near_ants(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>, groups: &mut Vec<uint>, group_index: uint, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, group: &mut DList<uint>, ours: bool) {
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, prev| {
    if groups[pos] == 0 && !moved[pos] {
      match world[pos] {
        Cell::Ant(0) | Cell::AnthillWithAnt(0) => {
          if ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world, moved) {
            group.push_back(pos);
            *groups.get_mut(pos).unwrap() = group_index;
          }
        },
        Cell::Ant(_) | Cell::AnthillWithAnt(_) => {
          if !ours && in_one_group(width, height, ant_pos, pos, attack_radius2, world, moved) {
            group.push_back(pos);
            *groups.get_mut(pos).unwrap() = group_index;
          }
        },
        _ => { }
      }
    }
    euclidean(width, height, ant_pos, prev) <= attack_radius2
  }, |_, _, _| { false });
  clear_tags(tags, tagged);
}

fn group_enough(ours_moves_count: uint, enemies_count: uint) -> bool {
  ours_moves_count > 21 ||
  ours_moves_count > 15 && enemies_count > 4 ||
  ours_moves_count > 11 && enemies_count > 7
}

fn get_group(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>, dangerous_place: &Vec<uint>, standing_ants: &Vec<uint>, groups: &mut Vec<uint>, group_index: uint, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, ours: &mut Vec<uint>, enemies: &mut Vec<uint>) -> uint {
  ours.clear();
  enemies.clear();
  let mut ours_moves_count = 0u;
  let mut enemies_count = 0u;
  let mut ours_q = DList::new();
  let mut enemies_q = DList::new();
  ours_q.push_back(ant_pos);
  *groups.get_mut(ant_pos).unwrap() = group_index;
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
    *groups.get_mut(pos).unwrap() = 0;
  }
  ours_moves_count
}

fn is_near_food(width: uint, height: uint, world: &Vec<Cell>, pos: uint) -> bool { //TODO: spawn_radius2
  if world[n(width, height, pos)] == Cell::Food ||
     world[s(width, height, pos)] == Cell::Food ||
     world[w(width, pos)] == Cell::Food ||
     world[e(width, pos)] == Cell::Food {
    true
  } else {
    false
  }
}

fn is_dead(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, board: &Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) -> bool {
  let mut result = false;
  let attack_value = board[ant_pos].attack;
  let ant_number = board[ant_pos].ant;
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| { euclidean(width, height, ant_pos, pos) <= attack_radius2 }, |pos, _, _| {
    let board_cell = board[pos].clone();
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

fn estimate(width: uint, height: uint, world: &Vec<Cell>, attack_radius2: uint, ants: &DList<uint>, other_ours: &Vec<uint>, board: &mut Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, aggression: uint) -> int {
  let mut ours_dead_count = 0u;
  let mut enemies_dead_count = 0u;
  let mut our_food = 0u;
  let mut enemy_food = 0u;
  let mut destroyed_enemy_anthills = 0u;
  let mut destroyed_our_anthills = 0u;
  let mut distance_to_enemies = 0u;
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    if board[ant_pos].ant == 0 {
      continue;
    }
    if is_dead(width, height, ant_pos, attack_radius2, board, tags, tagged) {
      board.get_mut(ant_pos).unwrap().dead = true;
    }
  }
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    let ant_board_cell = board[ant_pos].clone();
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
        if is_enemy_anthill(world[ant_pos].clone()) {
          destroyed_enemy_anthills += 1;
        }
        let mut min_distance_to_enemy = uint::MAX;
        for &enemy_pos in ants.iter() {
          let enemy_board_cell = board[enemy_pos].clone();
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
        if is_our_anthill(world[ant_pos].clone()) {
          destroyed_our_anthills += 1;
        }
      }
    }
  }
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    board.get_mut(ant_pos).unwrap().dead = false;
  }
  (enemies_dead_count * ENEMIES_DEAD_ESTIMATION[aggression]) as int - (ours_dead_count * OURS_DEAD_ESTIMATION[aggression]) as int +
  (our_food * OUR_FOOD_ESTIMATION[aggression]) as int - (enemy_food * ENEMY_FOOD_ESTIMATION[aggression]) as int +
  (destroyed_enemy_anthills * DESTROYED_ENEMY_ANTHILL_ESTIMATION[aggression]) as int - (destroyed_our_anthills * DESTROYED_OUR_ANTHILL_ESTIMATION[aggression]) as int -
  (distance_to_enemies * DISTANCE_TO_ENEMIES_ESTIMATION[aggression]) as int //TODO: штраф своему муравью за стояние на муравейнике. штраф своему муравью за стояние на одном месте. близость врага к муравейнику. точное вычисление того, кому достанется еда.
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

fn get_moves_count(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>) -> uint {
  let mut result = 1u;
  let mut escape = false;
  if dangerous_place[pos] == 0 {
    escape = true;
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  if !is_water_or_food(world[n_pos].clone()) {
    if dangerous_place[n_pos] == 0 {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[w_pos].clone()) {
    if dangerous_place[w_pos] == 0 {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[s_pos].clone()) {
    if dangerous_place[s_pos] == 0 {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[e_pos].clone()) {
    if dangerous_place[e_pos] == 0 {
      if !escape {
        result += 1;
      }
    } else {
      result += 1;
    }
  }
  result
}

fn get_escape_moves_count(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>) -> uint {
  let mut result = 0u;
  if dangerous_place[pos] == 0 {
    result += 1;
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  if !is_water_or_food(world[n_pos].clone()) && dangerous_place[n_pos] == 0 {
    result += 1;
  }
  if !is_water_or_food(world[w_pos].clone()) && dangerous_place[w_pos] == 0 {
    result += 1;
  }
  if !is_water_or_food(world[s_pos].clone()) && dangerous_place[s_pos] == 0 {
    result += 1;
  }
  if !is_water_or_food(world[e_pos].clone()) && dangerous_place[e_pos] == 0 {
    result += 1;
  }
  result
}

fn get_our_moves(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, board: &Vec<BoardCell>, moves: &mut DList<uint>) {
  let mut escape = false;
  if board[pos].ant == 0 {
    moves.push_back(pos);
    if dangerous_place[pos] == 0 {
      escape = true;
    }
  }
  let n_pos = n(width, height, pos);
  let s_pos = s(width, height, pos);
  let w_pos = w(width, pos);
  let e_pos = e(width, pos);
  let n_cell = world[n_pos].clone();
  let chain_begin = get_chain_begin(pos, board);
  if !is_water_or_food(n_cell.clone()) && !is_ant(n_cell.clone()) && board[n_pos].ant == 0 && n_pos != chain_begin {
    if dangerous_place[n_pos] == 0 {
      if !escape {
        moves.push_back(n_pos);
      }
      escape = true;
    } else {
      moves.push_back(n_pos);
    }
  }
  let w_cell = world[w_pos].clone();
  if !is_water_or_food(w_cell.clone()) && !is_ant(w_cell.clone()) && board[w_pos].ant == 0 && w_pos != chain_begin {
    if dangerous_place[w_pos] == 0 {
      if !escape {
        moves.push_back(w_pos);
      }
      escape = true;
    } else {
      moves.push_back(w_pos);
    }
  }
  let s_cell = world[s_pos].clone();
  if !is_water_or_food(s_cell.clone()) && !is_ant(s_cell.clone()) && board[s_pos].ant == 0 && s_pos != chain_begin {
    if dangerous_place[s_pos] == 0 {
      if !escape {
        moves.push_back(s_pos);
      }
      escape = true;
    } else {
      moves.push_back(s_pos);
    }
  }
  let e_cell = world[e_pos].clone();
  if !is_water_or_food(e_cell.clone()) && !is_ant(e_cell.clone()) && board[e_pos].ant == 0 && e_pos != chain_begin {
    if dangerous_place[e_pos] == 0 {
      if !escape {
        moves.push_back(e_pos);
      }
    } else {
      moves.push_back(e_pos);
    }
  }
}

// Рассматриваем также дополнительно сбегающие ходы на наши муравейники. Для своих муравьев такое делать не нужно, так как атака муравейников идет до сражения.
fn get_enemy_moves(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, board: &Vec<BoardCell>, standing_ants: &Vec<uint>, moves: &mut DList<uint>) {
  let mut escape = false;
  if board[pos].ant == 0 {
    moves.push_back(pos);
    if dangerous_place[pos] == 0 {
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
  let n_cell = world[n_pos].clone();
  let chain_begin = get_chain_begin(pos, board);
  if !is_water_or_food(n_cell.clone()) && !is_players_ant(n_cell.clone(), 0) && board[n_pos].ant == 0 && n_pos != chain_begin {
    if dangerous_place[n_pos] == 0 {
      if !escape || n_cell == Cell::Anthill(0) {
        moves.push_back(n_pos);
      }
      escape = true;
    } else {
      moves.push_back(n_pos);
    }
  }
  let w_cell = world[w_pos].clone();
  if !is_water_or_food(w_cell.clone()) && !is_players_ant(w_cell.clone(), 0) && board[w_pos].ant == 0 && w_pos != chain_begin {
    if dangerous_place[w_pos] == 0 {
      if !escape || w_cell == Cell::Anthill(0) {
        moves.push_back(w_pos);
      }
      escape = true;
    } else {
      moves.push_back(w_pos);
    }
  }
  let s_cell = world[s_pos].clone();
  if !is_water_or_food(s_cell.clone()) && !is_players_ant(s_cell.clone(), 0) && board[s_pos].ant == 0 && s_pos != chain_begin {
    if dangerous_place[s_pos] == 0 {
      if !escape || s_cell == Cell::Anthill(0) {
        moves.push_back(s_pos);
      }
      escape = true;
    } else {
      moves.push_back(s_pos);
    }
  }
  let e_cell = world[e_pos].clone();
  if !is_water_or_food(e_cell.clone()) && !is_players_ant(e_cell.clone(), 0) && board[e_pos].ant == 0 && e_pos != chain_begin {
    if dangerous_place[e_pos] == 0 {
      if !escape || e_cell == Cell::Anthill(0) {
        moves.push_back(e_pos);
      }
    } else {
      moves.push_back(e_pos);
    }
  }
}

fn is_minimax_timeout(start_time: u64, turn_time: uint, log: &mut DList<LogMessage>) -> bool {
  if elapsed_time(start_time) + MINIMAX_CRITICAL_TIME > turn_time {
    log.push_back(LogMessage::MinimaxTimeout);
    true
  } else {
    false
  }
}

fn minimax_min(width: uint, height: uint, idx: uint, minimax_moved: &mut DList<uint>, enemies: &Vec<uint>, other_ours: &Vec<uint>, world: &Vec<Cell>, dangerous_place_for_enemies: &Vec<uint>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, alpha: int, start_time: u64, turn_time: uint, aggression: uint, log: &mut DList<LogMessage>) -> int {
  if idx < enemies.len() {
    let pos = enemies[idx];
    let mut moves = DList::new();
    get_enemy_moves(width, height, pos, world, dangerous_place_for_enemies, board, standing_ants, &mut moves);
    let mut min_estimate = int::MAX;
    for &next_pos in moves.iter() {
      if is_minimax_timeout(start_time, turn_time, log) { return int::MIN; }
      minimax_moved.push_back(next_pos);
      let ant_number = ant_owner(world[pos].clone()).unwrap() + 1;
      board.get_mut(next_pos).unwrap().ant = ant_number;
      board.get_mut(next_pos).unwrap().cycle = pos + 1;
      board.get_mut(next_pos).unwrap().attack = dangerous_place_for_enemies[next_pos];
      simple_wave(width, height, tags, tagged, next_pos, |pos, _, _| {
        if euclidean(width, height, next_pos, pos) <= attack_radius2 {
          if board[pos].ant != 0 && board[pos].ant != ant_number {
            board.get_mut(pos).unwrap().attack += 1;
            if board[pos].ant != 1 {
              board.get_mut(next_pos).unwrap().attack += 1;
            }
          }
          true
        } else {
          false
        }
      }, |_, _, _| { false });
      clear_tags(tags, tagged);
      let cur_estimate = minimax_min(width, height, idx + 1, minimax_moved, enemies, other_ours, world, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, start_time, turn_time, aggression, log);
      simple_wave(width, height, tags, tagged, next_pos, |pos, _, _| {
        if euclidean(width, height, next_pos, pos) <= attack_radius2 {
          let board_cell = board.get_mut(pos).unwrap();
          if board_cell.ant != 0 && board_cell.ant != ant_number {
            board_cell.attack -= 1;
          }
          true
        } else {
          false
        }
      }, |_, _, _| { false });
      clear_tags(tags, tagged);
      board.get_mut(next_pos).unwrap().attack = 0;
      board.get_mut(next_pos).unwrap().ant = 0;
      board.get_mut(next_pos).unwrap().cycle = 0;
      minimax_moved.pop_back();
      if cur_estimate < min_estimate {
        min_estimate = cur_estimate;
        if cur_estimate <= alpha {
          break;
        }
      }
    }
    min_estimate
  } else {
    estimate(width, height, world, attack_radius2, minimax_moved, other_ours, board, tags, tagged, aggression)
  }
}

fn minimax_max(width: uint, height: uint, idx: uint, minimax_moved: &mut DList<uint>, ours: &Vec<uint>, enemies: &mut Vec<uint>, other_ours: &Vec<uint>, world: &Vec<Cell>, dangerous_place: &Vec<uint>, dangerous_place_for_enemies: &mut Vec<uint>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, alpha: &mut int, aggression: uint, start_time: u64, turn_time: uint, best_moves: &mut Vec<uint>, log: &mut DList<LogMessage>) {
  if idx < ours.len() {
    let pos = ours[idx];
    let mut moves = DList::new();
    get_our_moves(width, height, pos, world, dangerous_place, board, &mut moves);
    for &next_pos in moves.iter() {
      if is_minimax_timeout(start_time, turn_time, log) { return; }
      minimax_moved.push_back(next_pos);
      board.get_mut(next_pos).unwrap().ant = 1;
      board.get_mut(next_pos).unwrap().cycle = pos + 1;
      add_attack(width, height, attack_radius2, next_pos, dangerous_place_for_enemies, tags, tagged);
      minimax_max(width, height, idx + 1, minimax_moved, ours, enemies, other_ours, world, dangerous_place, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, aggression, start_time, turn_time, best_moves, log);
      remove_attack(width, height, attack_radius2, next_pos, dangerous_place_for_enemies, tags, tagged);
      board.get_mut(next_pos).unwrap().ant = 0;
      board.get_mut(next_pos).unwrap().cycle = 0;
      minimax_moved.pop_back();
    }
  } else {
    enemies.sort_by(|&pos1, &pos2| {
      let pos1_dangerous = dangerous_place_for_enemies[pos1] > 0;
      let pos2_dangerous = dangerous_place_for_enemies[pos2] > 0;
      if pos1_dangerous && !pos2_dangerous {
        cmp::Ordering::Less
      } else if !pos1_dangerous && pos2_dangerous {
        cmp::Ordering::Greater
      } else if pos1_dangerous && pos2_dangerous {
        get_escape_moves_count(width, height, pos1, world, dangerous_place_for_enemies).cmp(&get_escape_moves_count(width, height, pos2, world, dangerous_place_for_enemies))
      } else {
        cmp::Ordering::Equal
      }
    });
    let cur_estimate = minimax_min(width, height, 0, minimax_moved, enemies, other_ours, world, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, *alpha, start_time, turn_time, aggression, log);
    if cur_estimate > *alpha { //TODO: среди всех одинаковых выбирать ту оценку, которая больше при условии, что враг останется на месте.
      *alpha = cur_estimate;
      best_moves.clear();
      for &pos in minimax_moved.iter() {
        best_moves.push(pos);
      }
    }
  }
}

fn is_alone(width: uint, height: uint, attack_radius2: uint, world: &Vec<Cell>, ant_pos: uint, enemies: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) -> bool {
  for &enemy_pos in enemies.iter() {
    let result = simple_wave(width, height, tags, tagged, enemy_pos, |_, _, prev| {
      euclidean(width, height, enemy_pos, prev) <= attack_radius2
    }, |pos, _, _| { pos != ant_pos && is_players_ant(world[pos].clone(), 0) });
    clear_tags(tags, tagged);
    if !result.is_none() {
      return false;
    }
  }
  true
}

fn log_ants<'r, T: Iterator<Item=&'r uint>>(ants: &mut T) -> Box<DList<uint>> {
  let mut result = box DList::new();
  for &ant in ants {
    result.push_back(ant);
  }
  result
}

fn get_other_ours(width: uint, height: uint, world: &Vec<Cell>, groups: &Vec<uint>, tmp: &mut Vec<uint>, group_index: uint, attack_radius2: uint, enemies: &Vec<uint>, other_ours: &mut Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) {
  other_ours.clear();
  for &enemy_pos in enemies.iter() {
    simple_wave(width, height, tags, tagged, enemy_pos, |pos, _, prev| {
      if euclidean(width, height, enemy_pos, prev) <= attack_radius2 {
        if is_players_ant(world[pos].clone(), 0) && groups[pos] != group_index && tmp[pos] == 0 {
          *tmp.get_mut(pos).unwrap() = 1;
          other_ours.push(pos);
        }
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(tags, tagged);
  }
  for &ant_pos in other_ours.iter() {
    *tmp.get_mut(ant_pos).unwrap() = 0;
  }
}

fn add_attack(width: uint, height: uint, attack_radius2: uint, ant_pos: uint, attack_place: &mut Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) {
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| {
    if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
      *attack_place.get_mut(pos).unwrap() += 1;
      true
    } else {
      false
    }
  }, |_, _, _| { false });
  clear_tags(tags, tagged);
}

fn remove_attack(width: uint, height: uint, attack_radius2: uint, ant_pos: uint, attack_place: &mut Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) {
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| {
    if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
      *attack_place.get_mut(pos).unwrap() -= 1;
      true
    } else {
      false
    }
  }, |_, _, _| { false });
  clear_tags(tags, tagged);
}

fn attack(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::Attack);
  let mut ours = Vec::new();
  let mut other_ours = Vec::new();
  let mut enemies = Vec::new();
  let mut minimax_moved = DList::new();
  let mut best_moves = Vec::new();
  let mut group_index = 1;
  for &pos in colony.ours_ants.iter() {
    if is_minimax_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
    if colony.moved[pos] || colony.groups[pos] != 0 {
      continue;
    }
    let ours_moves_count = get_group(colony.width, colony.height, pos, colony.attack_radius2, &colony.world, &colony.moved, &colony.dangerous_place, &colony.standing_ants, &mut colony.groups, group_index, &mut colony.tags, &mut colony.tagged, &mut ours, &mut enemies);
    group_index += 1;
    if !enemies.is_empty() {
      let mut aggression = 0u;
      for &pos in ours.iter() {
        if colony.aggressive_place[pos] > aggression {
          aggression = colony.aggressive_place[pos];
        }
      }
      if ours.len() == 1 && ENEMIES_DEAD_ESTIMATION[aggression] < OURS_DEAD_ESTIMATION[aggression] && is_alone(colony.width, colony.height, colony.attack_radius2, &colony.world, ours[0], &enemies, &mut colony.tags, &mut colony.tagged) {
        colony.alone_ants.push(ours[0]);
        continue;
      }
      colony.log.push_back(LogMessage::Group(group_index));
      colony.log.push_back(LogMessage::GroupSize(ours_moves_count, enemies.len()));
      let mut alpha = int::MIN;
      colony.log.push_back(LogMessage::Aggression(aggression));
      ours.sort_by(|&pos1, &pos2| {
        let pos1_dangerous = colony.dangerous_place[pos1] > 0;
        let pos2_dangerous = colony.dangerous_place[pos2] > 0;
        if pos1_dangerous && !pos2_dangerous {
          cmp::Ordering::Less
        } else if !pos1_dangerous && pos2_dangerous {
          cmp::Ordering::Greater
        } else if pos1_dangerous && pos2_dangerous {
          get_escape_moves_count(colony.width, colony.height, pos1, &colony.world, &colony.dangerous_place).cmp(&get_escape_moves_count(colony.width, colony.height, pos2, &colony.world, &colony.dangerous_place))
        } else {
          cmp::Ordering::Equal
        }
      });
      for &pos in ours.iter() {
        remove_ant(&mut colony.world, pos);
      }
      colony.log.push_back(LogMessage::OursAnts(log_ants(&mut ours.iter())));
      colony.log.push_back(LogMessage::EnemiesAnts(log_ants(&mut enemies.iter())));
      get_other_ours(colony.width, colony.height, &colony.world, &colony.groups, &mut colony.tmp, group_index, colony.attack_radius2, &enemies, &mut other_ours, &mut colony.tags, &mut colony.tagged);
      colony.log.push_back(LogMessage::OtherOursAnts(log_ants(&mut other_ours.iter())));
      for &pos in other_ours.iter() {
        add_attack(colony.width, colony.height, colony.attack_radius2, pos, &mut colony.tmp, &mut colony.tags, &mut colony.tagged);
        colony.board.get_mut(pos).unwrap().ant = 1;
      }
      minimax_max(colony.width, colony.height, 0, &mut minimax_moved, &ours, &mut enemies, &other_ours, &colony.world, &colony.dangerous_place, &mut colony.tmp, colony.attack_radius2, &mut colony.board, &colony.standing_ants, &mut colony.tags, &mut colony.tagged, &mut alpha, aggression, colony.start_time, colony.turn_time, &mut best_moves, &mut colony.log);
      colony.log.push_back(LogMessage::Estimate(alpha));
      for &pos in other_ours.iter() {
        remove_attack(colony.width, colony.height, colony.attack_radius2, pos, &mut colony.tmp, &mut colony.tags, &mut colony.tagged);
        colony.board.get_mut(pos).unwrap().ant = 0;
      }
      for &pos in ours.iter() {
        set_ant(&mut colony.world, pos, 0);
      }
      if alpha != int::MIN {
        let mut moves = DList::new();
        for i in range(0u, ours.len()) {
          let pos = ours[i];
          let next_pos = best_moves[i];
          if pos == next_pos {
            *colony.moved.get_mut(pos).unwrap() = true;
          } else {
            moves.push_back((pos, next_pos));
          }
        }
        move_all(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, &moves, &mut colony.log);
        for &pos in enemies.iter() {
          *colony.fighting.get_mut(pos).unwrap() = true;
        }
      }
    }
  }
}

fn escape_estimate(width: uint, height: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, estimate_pos: uint, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) -> int {
  let mut estimate = 0;
  simple_wave(width, height, tags, tagged, estimate_pos, |pos, path_size, _| {
    let cell = world[pos].clone();
    if path_size > ESCAPE_PATH_SIZE || cell == Cell::Water {
      false
    } else {
      estimate += (ESCAPE_PATH_SIZE + 1 - path_size) as int * if is_enemy_ant(cell.clone()) { //TODO: Move to constants.
        -7
      } else if is_players_ant(cell.clone(), 0) {
        7
      } else if cell == Cell::Food {
        3
      } else if dangerous_place[pos] == 0 {
        1
      } else {
        0
      };
      true
    }
  }, |_, _, _| { false });
  clear_tags(tags, tagged);
  estimate
}

fn escape(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::Escape);
  let mut moves = Vec::with_capacity(5);
  let mut safe_moves = Vec::with_capacity(5);
  for &ant_pos in colony.alone_ants.iter() {
    if colony.moved[ant_pos] {
      continue;
    }
    moves.clear();
    safe_moves.clear();
    moves.push(ant_pos);
    let n_pos = n(colony.width, colony.height, ant_pos);
    let s_pos = s(colony.width, colony.height, ant_pos);
    let w_pos = w(colony.width, ant_pos);
    let e_pos = e(colony.width, ant_pos);
    if is_free(colony.world[n_pos].clone()) {
      moves.push(n_pos);
    }
    if is_free(colony.world[s_pos].clone()) {
      moves.push(s_pos);
    }
    if is_free(colony.world[w_pos].clone()) {
      moves.push(w_pos);
    }
    if is_free(colony.world[e_pos].clone()) {
      moves.push(e_pos);
    }
    if moves.is_empty() {
      *colony.moved.get_mut(ant_pos).unwrap() = true;
      colony.log.push_back(LogMessage::Goal(ant_pos, ant_pos));
      continue;
    }
    for &pos in moves.iter() {
      if colony.dangerous_place[pos] == 0 {
        safe_moves.push(pos);
      }
    }
    let mut next_pos;
    if safe_moves.is_empty() {
      next_pos = moves[0];
      let mut min_danger = colony.dangerous_place[moves[0]];
      for i in range(1u, moves.len()) {
        let cur_danger = colony.dangerous_place[moves[i]];
        if cur_danger < min_danger || cur_danger == min_danger && colony.rng.gen() {
          min_danger = cur_danger;
          next_pos = moves[i];
        }
      }
    } else {
      next_pos = safe_moves[0];
      let mut max_estimate = escape_estimate(colony.width, colony.height, &colony.world, &colony.dangerous_place, safe_moves[0], &mut colony.tags, &mut colony.tagged);
      for i in range(1u, safe_moves.len()) {
        let cur_estimate = escape_estimate(colony.width, colony.height, &colony.world, &colony.dangerous_place, safe_moves[i], &mut colony.tags, &mut colony.tagged);
        if cur_estimate > max_estimate || cur_estimate == max_estimate && colony.rng.gen() {
          max_estimate = cur_estimate;
          next_pos = safe_moves[i];
        }
      }
    }
    if next_pos != ant_pos {
      move_one(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, ant_pos, next_pos, &mut colony.log);
      colony.log.push_back(LogMessage::Goal(ant_pos, next_pos));
    } else {
      *colony.moved.get_mut(ant_pos).unwrap() = true;
      colony.log.push_back(LogMessage::Goal(ant_pos, ant_pos));
    }
  }
}

fn approach_enemies(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::ApproachEnemies);
  let width = colony.width;
  let height = colony.height;
  let approach_path_size = colony.max_attack_radius_manhattan + APPROACH_PATH_SIZE_CONST;
  let fighting = &colony.fighting;
  let dangerous_place = &colony.dangerous_place;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  let log = &mut colony.log;
  wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, &mut colony.enemies_ants.iter().filter(|&&pos| { fighting[pos] }), |pos, start_pos, path_size, prev| {
    if path_size > approach_path_size {
      return false;
    }
    let cell = world[pos].clone();
    if !is_free(cell.clone()) {
      if is_players_ant(cell.clone(), 0) && !moved[pos] {
        log.push_back(LogMessage::Goal(pos, start_pos));
        if dangerous_place[prev] == 0 {
          move_one(width, height, world, moved, output, pos, prev, log);
          true
        } else {
          *moved.get_mut(pos).unwrap() = true;
          false
        }
      } else {
        false
      }
    } else {
      true
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn calculate_aggressive_place(colony: &mut Colony) {
  let aggressive_place = &mut colony.aggressive_place;
  for &pos in colony.ours_ants.iter() {
    let mut neighbors = 0;
    if colony.world[n(colony.width, colony.height, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[w(colony.width, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[s(colony.width, colony.height, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[e(colony.width, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[nw(colony.width, colony.height, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[ne(colony.width, colony.height, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[sw(colony.width, colony.height, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    if colony.world[se(colony.width, colony.height, pos)] == Cell::Ant(0) {
      neighbors += 1;
    }
    *aggressive_place.get_mut(pos).unwrap() = NEIGHBORS_AGGRESSION[neighbors];
  }
  if colony.ours_anthills.len() > DANGEROUS_ANTHILLS_COUNT {
    return;
  }
  wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_anthills.iter(), |pos, _, path_size, _| {
    if path_size <= OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE {
      *aggressive_place.get_mut(pos).unwrap() = cmp::max(aggressive_place[pos], OURS_ANTHILLS_AGGRESSION);
      true
    } else {
      false
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn calculate_dangerous_place(colony: &mut Colony) { //TODO: standing_ants.
  let width = colony.width;
  let height = colony.height;
  let attack_radius2 = colony.attack_radius2;
  let dangerous_place = &mut colony.dangerous_place;
  for &ant_pos in colony.enemies_ants.iter() {
    let n_pos = n(colony.width, colony.height, ant_pos);
    let s_pos = s(colony.width, colony.height, ant_pos);
    let w_pos = w(colony.width, ant_pos);
    let e_pos = e(colony.width, ant_pos);
    let n_pos_water_or_food = is_water_or_food(colony.world[n_pos].clone());
    let s_pos_water_or_food = is_water_or_food(colony.world[s_pos].clone());
    let w_pos_water_or_food = is_water_or_food(colony.world[w_pos].clone());
    let e_pos_water_or_food = is_water_or_food(colony.world[e_pos].clone());
    simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 ||
         euclidean(width, height, n_pos, pos) <= attack_radius2 && !n_pos_water_or_food ||
         euclidean(width, height, s_pos, pos) <= attack_radius2 && !s_pos_water_or_food ||
         euclidean(width, height, w_pos, pos) <= attack_radius2 && !w_pos_water_or_food ||
         euclidean(width, height, e_pos, pos) <= attack_radius2 && !e_pos_water_or_food {
        *dangerous_place.get_mut(pos).unwrap() += 1;
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
}

fn defend_anhills(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::DefendAnthills);
  if colony.ours_anthills.len() > DANGEROUS_ANTHILLS_COUNT {
    return;
  }
  let world = &mut colony.world;
  let dangerous_place = &colony.dangerous_place;
  let tmp = &mut colony.tmp;
  let mut defended_anhills = 0u;
  let mut path = Vec::new();
  let mut defenders = DList::new();
  for &anthill_pos in colony.ours_anthills.iter() {
    let mut defended = false;
    let mut enemies_ants = DList::new();
    simple_wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, anthill_pos, |pos, path_size, _| {
      let cell = world[pos].clone();
      if path_size > DEFEND_ANTHILLS_PATH_SIZE || cell == Cell::Water {
        false
      } else {
        if is_enemy_ant(cell) {
          enemies_ants.push_back(pos);
        }
        true
      }
    }, |_, _, _| { false });
    for &ant_pos in enemies_ants.iter() {
      find_inverse_path(&colony.tags, anthill_pos, ant_pos, &mut path);
      let mut maybe_defender = None;
      for &pos in path.iter() {
        if is_players_ant(world[pos].clone(), 0) && tmp[pos] == 0 {
          maybe_defender = Some(pos);
          break;
        }
        let n_pos = n(colony.width, colony.height, pos);
        if is_players_ant(world[n_pos].clone(), 0) && tmp[n_pos] == 0 {
          maybe_defender = Some(n_pos);
          break;
        }
        let w_pos = w(colony.width, pos);
        if is_players_ant(world[w_pos].clone(), 0) && tmp[w_pos] == 0 {
          maybe_defender = Some(w_pos);
          break;
        }
        let s_pos = s(colony.width, colony.height, pos);
        if is_players_ant(world[s_pos].clone(), 0) && tmp[s_pos] == 0 {
          maybe_defender = Some(s_pos);
          break;
        }
        let e_pos = e(colony.width, pos);
        if is_players_ant(world[e_pos].clone(), 0) && tmp[e_pos] == 0 {
          maybe_defender = Some(e_pos);
          break;
        }
      }
      if maybe_defender.is_none() {
        let three_fourth_pos = path[path.len() * 3 / 4];
        maybe_defender = simple_wave(colony.width, colony.height, &mut colony.tags2, &mut colony.tagged2, three_fourth_pos, |pos, path_size, _| {
          path_size <= DEFENDER_PATH_SIZE && world[pos] != Cell::Water
        }, |pos, _, _| { is_players_ant(world[pos].clone(), 0) && tmp[pos] == 0 });
        clear_tags(&mut colony.tags2, &mut colony.tagged2);
      }
      if maybe_defender.is_none() {
        continue;
      }
      defended = true;
      let defender = maybe_defender.unwrap();
      colony.log.push_back(LogMessage::Defender(anthill_pos, ant_pos, defender));
      defenders.push_back(defender);
      *tmp.get_mut(defender).unwrap() = 1;
      if colony.moved[defender] {
        continue;
      }
      let center_pos = path[path.len() / 2];
      if defender == center_pos {
        *colony.moved.get_mut(defender).unwrap() = true;
        colony.log.push_back(LogMessage::Goal(defender, defender));
        continue;
      }
      if simple_wave(colony.width, colony.height, &mut colony.tags2, &mut colony.tagged2, defender, |pos, _, prev| { //TODO: A*.
        let cell = world[pos].clone();
        pos == defender || cell != Cell::Water && (prev != defender || is_free(cell) && dangerous_place[pos] == 0)
      }, |pos, _, _| { pos == center_pos }).is_none() {
        clear_tags(&mut colony.tags2, &mut colony.tagged2);
        *colony.moved.get_mut(defender).unwrap() = true;
        colony.log.push_back(LogMessage::Goal(defender, defender));
        continue;
      }
      let mut defender_path = DList::new();
      find_path(&colony.tags2, defender, center_pos, &mut defender_path);
      clear_tags(&mut colony.tags2, &mut colony.tagged2);
      let next_pos = *defender_path.front().unwrap();
      move_one(colony.width, colony.height, world, &mut colony.moved, output, defender, next_pos, &mut colony.log);
      colony.log.push_back(LogMessage::Goal(defender, center_pos));
      defenders.push_back(next_pos);
      *tmp.get_mut(next_pos).unwrap() = 1;
    }
    clear_tags(&mut colony.tags, &mut colony.tagged);
    if defended {
      defended_anhills += 1;
      if defended_anhills > DEFEND_ANTHILLS_COUNT {
        break;
      }
    }
  }
  for &defender in defenders.iter() {
    *tmp.get_mut(defender).unwrap() = 0;
  }
}

fn get_random_move(width: uint, height: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, rng: &mut XorShiftRng, pos: uint) -> uint {
  let mut moves = Vec::new();
  moves.push(pos);
  let n_pos = n(width, height, pos);
  if is_free(world[n_pos].clone()) && dangerous_place[n_pos] == 0 {
    moves.push(n_pos);
  }
  let w_pos = w(width, pos);
  if is_free(world[w_pos].clone()) && dangerous_place[w_pos] == 0 {
    moves.push(w_pos);
  }
  let s_pos = s(width, height, pos);
  if is_free(world[s_pos].clone()) && dangerous_place[s_pos] == 0 {
    moves.push(s_pos);
  }
  let e_pos = e(width, pos);
  if is_free(world[e_pos].clone()) && dangerous_place[e_pos] == 0 {
    moves.push(e_pos);
  }
  moves[rng.gen_range(0, moves.len())]
}

fn move_random(colony: &mut Colony, output: &mut DList<Step>) {
  colony.log.push_back(LogMessage::MoveRandom);
  for &ant_pos in colony.ours_ants.iter() {
    if colony.moved[ant_pos] {
      continue;
    }
    let next_pos = get_random_move(colony.width, colony.height, &colony.world, &colony.dangerous_place, &mut colony.rng, ant_pos);
    if next_pos != ant_pos {
      move_one(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, ant_pos, next_pos, &mut colony.log);
      colony.log.push_back(LogMessage::Goal(ant_pos, next_pos));
    } else {
      *colony.moved.get_mut(ant_pos).unwrap() = true;
      colony.log.push_back(LogMessage::Goal(ant_pos, ant_pos));
    }
  }
}

fn shuffle(colony: &mut Colony) {
  colony.rng.shuffle(colony.ours_ants.as_mut_slice());
  colony.rng.shuffle(colony.enemies_ants.as_mut_slice());
  colony.ours_anthills.sort();
  colony.enemies_anthills.sort();
}

fn update_world<'r, T: Iterator<Item=&'r Input>>(colony: &mut Colony, input: &mut T) {
  let view_radius2 = colony.view_radius2;
  let attack_radius2 = colony.attack_radius2;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let width = colony.width;
  let height = colony.height;
  let visible_area = &mut colony.visible_area;
  let discovered_area = &mut colony.discovered_area;
  let last_world = &mut colony.last_world;
  let world = &mut colony.world;
  let len = length(width, height);
  for pos in range(0u, len) {
    *last_world.get_mut(pos).unwrap() = world[pos].clone();
    *world.get_mut(pos).unwrap() = Cell::Unknown;
    *colony.moved.get_mut(pos).unwrap() = false;
    *colony.gathered_food.get_mut(pos).unwrap() = 0;
    *visible_area.get_mut(pos).unwrap() += 1;
    *discovered_area.get_mut(pos).unwrap() += 1;
    *colony.territory.get_mut(pos).unwrap() = 0;
    *colony.groups.get_mut(pos).unwrap() = 0;
    *colony.dangerous_place.get_mut(pos).unwrap() = 0;
    *colony.aggressive_place.get_mut(pos).unwrap() = 0;
    *colony.fighting.get_mut(pos).unwrap() = false;
  }
  colony.ours_ants.clear();
  colony.enemies_ants.clear();
  colony.enemies_anthills.clear();
  colony.ours_anthills.clear();
  colony.food.clear();
  colony.alone_ants.clear();
  for i in input {
    match i.clone() {
      Input::InputWater(point) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos).unwrap() = Cell::Water;
      },
      Input::InputFood(point) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos).unwrap() = Cell::Food;
        colony.food.push(pos);
      },
      Input::InputAnthill(point, player) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos).unwrap() = if world[pos] == Cell::Ant(player) { Cell::AnthillWithAnt(player) } else { Cell::Anthill(player) };
        if player == 0 {
          colony.ours_anthills.push(pos);
        } else {
          colony.enemies_anthills.push(pos);
          if player > colony.enemies_count {
            colony.enemies_count = player;
          }
        }
      },
      Input::InputAnt(point, player) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos).unwrap() = if world[pos] == Cell::Anthill(player) { Cell::AnthillWithAnt(player) } else { Cell::Ant(player) };
        if player == 0 {
          colony.ours_ants.push(pos);
        } else {
          colony.enemies_ants.push(pos);
          if player > colony.enemies_count {
            colony.enemies_count = player;
          }
        }
      },
      Input::InputDead(_, _) => { }
    }
  }
  for &ant_pos in colony.ours_ants.iter() {
    simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, _| {
      if euclidean(width, height, pos, ant_pos) <= view_radius2 {
        if manhattan(width, height, pos, ant_pos) <= min_view_radius_manhattan {
          *discovered_area.get_mut(pos).unwrap() = 0;
        }
        *visible_area.get_mut(pos).unwrap() = 0;
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
  for pos in range(0u, len) {
    if visible_area[pos] == 0 {
      if world[pos] == Cell::Unknown {
        *world.get_mut(pos).unwrap() = match last_world[pos] {
          Cell::Water => Cell::Water,
          _ => Cell::Land
        }
      }
      if is_enemy_ant(world[pos].clone()) {
        *colony.standing_ants.get_mut(pos).unwrap() += 1;
      } else {
        *colony.standing_ants.get_mut(pos).unwrap() = 0;
      }
    } else {
      *world.get_mut(pos).unwrap() = match last_world[pos] {
         Cell::Water => {
          *visible_area.get_mut(pos).unwrap() = 0;
          Cell::Water
        },
        Cell::Food => {
          colony.food.push(pos);
          Cell::Food
        },
        Cell::Land => Cell::Land,
        Cell::Unknown => Cell::Unknown,
        Cell::Ant(0) | Cell::AnthillWithAnt(0) => Cell::Land,
        Cell::Anthill(0) => {
          colony.ours_anthills.push(pos);
          Cell::Anthill(0)
        },
        Cell::Ant(player) => {
          colony.enemies_ants.push(pos);
          Cell::Ant(player)
        },
        Cell::Anthill(player) => {
          colony.enemies_anthills.push(pos);
          Cell::Anthill(player)
        }
        Cell::AnthillWithAnt(player) => {
          colony.enemies_anthills.push(pos);
          colony.enemies_ants.push(pos);
          Cell::AnthillWithAnt(player)
        }
      };
      *colony.standing_ants.get_mut(pos).unwrap() = 0;
    }
  }
  for ant_pos in range(0u, len) {
    if colony.standing_ants[ant_pos] > STANDING_ANTS_CONST {
      if !simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |_, _, prev| { euclidean(width, height, prev, ant_pos) <= attack_radius2 }, |pos, _, _| { last_world[pos] != Cell::Unknown && last_world[pos] != world[pos] }).is_none() {
        *colony.standing_ants.get_mut(ant_pos).unwrap() = STANDING_ANTS_WITH_CHANGED_ENVIRONMENT_CONST;
      }
      clear_tags(&mut colony.tags, &mut colony.tagged);
    }
  }
}

fn is_timeout(start_time: u64, turn_time: uint, log: &mut DList<LogMessage>) -> bool {
  if elapsed_time(start_time) + CRITICAL_TIME > turn_time {
    log.push_back(LogMessage::Timeout);
    true
  } else {
    false
  }
}

pub fn turn<'r, T: Iterator<Item=&'r Input>>(colony: &mut Colony, input: &mut T, output: &mut DList<Step>) {
  colony.start_time = get_time();
  output.clear();
  colony.cur_turn += 1;
  colony.log.push_back(LogMessage::Turn(colony.cur_turn));
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  update_world(colony, input);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  shuffle(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  calculate_dangerous_place(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  attack_anthills(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  gather_food(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  calculate_aggressive_place(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  attack(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  escape(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  defend_anhills(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  approach_enemies(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  discover(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  calculate_territory(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  travel(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  move_random(colony, output);
}
