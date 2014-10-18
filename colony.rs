//TODO: аггрессивность, если игра долго идет, а противник известен только один.
//TODO: динамический подбор констатнт минимакса путем определения производительности на этапе инициализации. Динамическое уменьшение этих констант при таймаутах.
//TODO: агрессия возле муравейника в случае если враг его не видел.
//TODO: захват муравейников вместо уничтожения.
//TODO: обнуление стоящих муравьев после нашего прогноза на стояние. Либо не считать муравьев стоящими если у них меняется окружение.
//TODO: в случае безвыходной ситуации ходить на вражеского муравья.

use std::{int, uint, cmp};
use std::collections::*;
use std::rand::*;
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

static NEIGHBORS_AGGRESSION: &'static [uint] = &[0, 0, 1, 1, 1, 2, 3, 4, 5]; // Уровни агрессии для муравья от числа его соседей.

static OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE: uint = 4; // Максимальное манхэттенское расстояние от нашего муравейника до нашего муравья, при котором он считается агрессивным, а с ним и вся группа.

static OURS_ANTHILLS_AGGRESSION: uint = 2; // Уровень агрессии для наших муравьев, близких к нашим муравейникам.

#[deriving(Clone)]
struct BoardCell {
  ant: uint, // Номер игрока, чей муравей сделал ход в текущую клетку, плюс один.
  attack: uint, // Количество врагов, атакующих муравья.
  cycle: uint, // Клитка, из которой сделал ход муравей в текущую. Нужно для отсечения циклов (хождений муравьев по кругу).
  dead: bool // Помечаются флагом погибшие в битве свои и чужие муравьи.
}

pub struct Colony { //TODO: make all fields private.
  pub width: uint, // Ширина поля.
  pub height: uint, // Высота поля.
  pub turn_time: uint, // Время на один ход.
  pub turns_count: uint, // Количество ходов в игре.
  pub view_radius2: uint,
  pub attack_radius2: uint,
  pub spawn_radius2: uint,
  pub cur_turn: uint, // Номер текущего хода.
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
  standing_ants: Vec<uint>, // Каждый ход увеличивается на 1 для вражеских муравьев и сбрасывается в 0 для всех остальных клеток. То есть показывает, сколько ходов на месте стоит вражеский муравей.
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
  pub log: DList<LogMessage>
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
      world: Vec::from_elem(len, Unknown),
      last_world: Vec::from_elem(len, Unknown),
      visible_area: Vec::from_elem(len, 0u),
      discovered_area: Vec::from_elem(len, 0u),
      standing_ants: Vec::from_elem(len, 0u),
      moved: Vec::from_elem(len, false),
      gathered_food: Vec::from_elem(len, 0u),
      territory: Vec::from_elem(len, 0u),
      dangerous_place: Vec::from_elem(len, 0u),
      aggressive_place: Vec::from_elem(len, 0u),
      groups: Vec::from_elem(len, 0u),
      fighting: Vec::from_elem(len, false),
      board: Vec::from_elem(len, BoardCell { ant: 0, attack: 0, cycle: 0, dead: false }),
      tmp: Vec::from_elem(len, 0u),
      alone_ants: Vec::with_capacity(len),
      tags: Vec::from_elem(len, Tag::new()),
      tagged: Vec::with_capacity(len),
      tags2: Vec::from_elem(len, Tag::new()),
      tagged2: Vec::with_capacity(len),
      ours_ants: Vec::with_capacity(len),
      enemies_ants: Vec::with_capacity(len),
      enemies_anthills: Vec::with_capacity(len),
      ours_anthills: Vec::with_capacity(len),
      food: Vec::with_capacity(len),
      log: DList::new()
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

fn move_one<T: MutableSeq<Step>>(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut T, pos: uint, next_pos: uint) {
  remove_ant(world, pos);
  *moved.get_mut(pos) = true;
  *world.get_mut(next_pos) = if (*world)[next_pos] == Anthill(0) { AnthillWithAnt(0) } else { Ant(0) };
  *moved.get_mut(next_pos) = true;
  output.push(Step { point: from_pos(width, pos), direction: to_direction(width, height, pos, next_pos).unwrap() })
}

fn move_all<T: MutableSeq<Step>>(width: uint, height: uint, world: &mut Vec<Cell>, moved: &mut Vec<bool>, output: &mut T, moves: &DList<(uint, uint)>) {
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
    output.push(Step { point: from_pos(width, pos), direction: to_direction(width, height, pos, next_pos).unwrap() });
  }
}

fn discover_direction(width: uint, height: uint, min_view_radius_manhattan: uint, world: &Vec<Cell>, discovered_area: &Vec<uint>, dangerous_place: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, ant_pos: uint) -> Option<uint> {
  let mut n_score = 0u;
  let mut s_score = 0u;
  let mut w_score = 0u;
  let mut e_score = 0u;
  let n_pos = n(width, height, ant_pos);
  let s_pos = s(width, height, ant_pos);
  let w_pos = w(width, ant_pos);
  let e_pos = e(width, ant_pos);
  if is_free(world[n_pos]) && dangerous_place[n_pos] == 0 {
    simple_wave(width, height, tags, tagged, n_pos, |pos, path_size, prev| {
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
  if is_free(world[s_pos]) && dangerous_place[s_pos] == 0 {
    simple_wave(width, height, tags, tagged, s_pos, |pos, path_size, prev| {
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
  if is_free(world[w_pos]) && dangerous_place[w_pos] == 0 {
    simple_wave(width, height, tags, tagged, w_pos, |pos, path_size, prev| {
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
  if is_free(world[e_pos])  && dangerous_place[e_pos] == 0 {
    simple_wave(width, height, tags, tagged, e_pos, |pos, path_size, prev| {
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
  if n_score == 0 && s_score == 0 && w_score == 0 && e_score == 0 { //TODO: при равенстве учитывать расстояние до своих муравьев.
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

fn discover<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  colony.log.push(Discover);
  let width = colony.width;
  let height = colony.height;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let discovered_area = &mut colony.discovered_area;
  for &pos in colony.ours_ants.iter() {
    if colony.moved[pos] {
      continue;
    }
    match discover_direction(width, height, min_view_radius_manhattan, &colony.world, discovered_area, &colony.dangerous_place, &mut colony.tags, &mut colony.tagged, pos) {
      Some(next_pos) => {
        simple_wave(width, height, &mut colony.tags, &mut colony.tagged, next_pos, |pos, _, _| {
          if manhattan(width, height, pos, next_pos) <= min_view_radius_manhattan {
            *discovered_area.get_mut(pos) = 0;
            true
          } else {
            false
          }
        }, |_, _, _| { false });
        clear_tags(&mut colony.tags, &mut colony.tagged);
        move_one(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, pos, next_pos);
      },
      None => { }
    }
  }
}

fn travel<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  colony.log.push(Travel);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let territory = &mut colony.territory;
  let territory_path_size = colony.max_view_radius_manhattan + TERRITORY_PATH_SIZE_CONST;
  let moved = &mut colony.moved;
  let tmp = &mut colony.tmp;
  let dangerous_place = &colony.dangerous_place;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_ants.iter().chain(colony.enemies_ants.iter()).chain(colony.enemies_anthills.iter()), |pos, start_pos, path_size, _| {
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
  for &ant_pos in colony.ours_ants.iter() {
    if (*moved)[ant_pos] {
      continue;
    }
    *tmp.get_mut(ant_pos) = 1;
    let goal = simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, prev| {
      let cell = (*world)[pos];
      let is_column = (*tmp)[prev];
      if cell == Water || is_column == 1 && ((*moved)[pos] && is_players_ant(cell, 0) || cell == Food || dangerous_place[pos] > 0 && pos != ant_pos) {
        false
      } else {
        *tmp.get_mut(pos) = if is_players_ant(cell, 0) { is_column } else { 0 };
        true
      }
    }, |pos, _, _| { (*territory)[pos] != 1 });
    if goal.is_none() {
      for &pos in colony.tagged.iter() {
        *tmp.get_mut(pos) = 0;
      }
      clear_tags(&mut colony.tags, &mut colony.tagged);
      continue;
    }
    find_path(&mut colony.tags, ant_pos, goal.unwrap(), &mut path);
    for &pos in colony.tagged.iter() {
      *tmp.get_mut(pos) = 0;
    }
    clear_tags(&mut colony.tags, &mut colony.tagged);
    let mut path_pos = ant_pos;
    let mut moves = DList::new();
    for &pos in path.iter() {
      moves.push((path_pos, pos));
      if !is_players_ant((*world)[pos], 0) {
        break;
      }
      path_pos = pos;
    }
    move_all(width, height, world, moved, output, &moves);
  }
}

fn attack_anthills<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  colony.log.push(AttackAnthills);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  let dangerous_place = &colony.dangerous_place;
  let tmp = &mut colony.tmp;
  let log = &mut colony.log;
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.enemies_anthills.iter(), |pos, start_pos, path_size, prev| {
    if pos != start_pos && dangerous_place[pos] > 0 || path_size > ATTACK_ANTHILLS_PATH_SIZE || (*tmp)[start_pos] > ATTACK_ANTHILLS_ANTS_COUNT {
      return false;
    }
    match (*world)[pos] {
      Ant(0) | AnthillWithAnt(0) if !(*moved)[pos] => {
        if !is_free((*world)[prev]) {
          false
        } else {
          *tmp.get_mut(start_pos) += 1;
          move_one(width, height, world, moved, output, pos, prev);
          log.push(Goal(pos, start_pos));
          true
        }
      },
      Unknown | Water => false,
      _ => true
    }
  }, |_, _, _, _| { false });
  clear_tags(&mut colony.tags, &mut colony.tagged);
  for &pos in colony.enemies_anthills.iter() {
    *tmp.get_mut(pos) = 0;
  }
}

fn gather_food<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  colony.log.push(GatherFood);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let gathered_food = &mut colony.gathered_food;
  let moved = &mut colony.moved;
  let dangerous_place = &colony.dangerous_place;
  let log = &mut colony.log;
  for &pos in colony.ours_ants.iter() {
    if (*moved)[pos] || dangerous_place[pos] > 0 {
      continue;
    }
    let n_pos = n(width, height, pos);
    if (*world)[n_pos] == Food && (*gathered_food)[n_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(n_pos) = pos + 1;
      log.push(Goal(pos, n_pos));
    }
    let s_pos = s(width, height, pos);
    if (*world)[s_pos] == Food && (*gathered_food)[s_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(s_pos) = pos + 1;
      log.push(Goal(pos, s_pos));
    }
    let w_pos = w(width, pos);
    if (*world)[w_pos] == Food && (*gathered_food)[w_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(w_pos) = pos + 1;
      log.push(Goal(pos, w_pos));
    }
    let e_pos = e(width, pos);
    if (*world)[e_pos] == Food && (*gathered_food)[e_pos] == 0 {
      *moved.get_mut(pos) = true;
      *gathered_food.get_mut(e_pos) = pos + 1;
      log.push(Goal(pos, e_pos));
    }
  }
  wave(width, height, &mut colony.tags, &mut colony.tagged, &mut colony.food.iter(), |pos, start_pos, path_size, prev| {
    if pos != start_pos && dangerous_place[pos] > 0 || path_size > GATHERING_FOOD_PATH_SIZE {
      return false;
    }
    match (*world)[pos] {
      Ant(0) | AnthillWithAnt(0) if (*gathered_food)[start_pos] == 0 && !(*moved)[pos] => {
        if pos != start_pos && !is_free((*world)[prev]) {
          false
        } else {
          move_one(width, height, world, moved, output, pos, prev);
          *gathered_food.get_mut(start_pos) = pos + 1;
          log.push(Goal(pos, start_pos));
          true
        }
      },
      Unknown | Water => false,
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

fn find_near_ants<T: MutableSeq<uint>>(width: uint, height: uint, ant_pos: uint, attack_radius2: uint, world: &Vec<Cell>, moved: &Vec<bool>, groups: &mut Vec<uint>, group_index: uint, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, group: &mut T, ours: bool) {
  simple_wave(width, height, tags, tagged, ant_pos, |pos, _, prev| {
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
  ours_moves_count
}

fn is_near_food(width: uint, height: uint, world: &Vec<Cell>, pos: uint) -> bool { //TODO: spawn_radius2
  if world[n(width, height, pos)] == Food ||
     world[s(width, height, pos)] == Food ||
     world[w(width, pos)] == Food ||
     world[e(width, pos)] == Food {
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

fn estimate(width: uint, height: uint, world: &Vec<Cell>, attack_radius2: uint, ants: &DList<uint>, board: &mut Vec<BoardCell>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, aggression: uint) -> int { //TODO: для оптимизации юзать dangerous_place, чтобы не считать атаку своих муравьев.
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
    simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| {
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
    simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| {
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
  if !is_water_or_food(world[n_pos]) {
    if dangerous_place[n_pos] == 0 {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[w_pos]) {
    if dangerous_place[w_pos] == 0 {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[s_pos]) {
    if dangerous_place[s_pos] == 0 {
      if !escape {
        result += 1;
      }
      escape = true;
    } else {
      result += 1;
    }
  }
  if !is_water_or_food(world[e_pos]) {
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
  if !is_water_or_food(world[n_pos]) && dangerous_place[n_pos] == 0 {
    result += 1;
  }
  if !is_water_or_food(world[w_pos]) && dangerous_place[w_pos] == 0 {
    result += 1;
  }
  if !is_water_or_food(world[s_pos]) && dangerous_place[s_pos] == 0 {
    result += 1;
  }
  if !is_water_or_food(world[e_pos]) && dangerous_place[e_pos] == 0 {
    result += 1;
  }
  result
}

fn get_our_moves<T: MutableSeq<uint>>(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, board: &Vec<BoardCell>, moves: &mut T) {
  let mut escape = false;
  if board[pos].ant == 0 {
    moves.push(pos);
    if dangerous_place[pos] == 0 {
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
    if dangerous_place[n_pos] == 0 {
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
    if dangerous_place[w_pos] == 0 {
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
    if dangerous_place[s_pos] == 0 {
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
    if dangerous_place[e_pos] == 0 {
      if !escape {
        moves.push(e_pos);
      }
    } else {
      moves.push(e_pos);
    }
  }
}

// Рассматриваем также дополнительно сбегающие ходы на наши муравейники. Для своих муравьев такое делать не нужно, так как атака муравейников идет до сражения.
fn get_enemy_moves<T: MutableSeq<uint>>(width: uint, height: uint, pos: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, board: &Vec<BoardCell>, standing_ants: &Vec<uint>, moves: &mut T) {
  let mut escape = false;
  if board[pos].ant == 0 {
    moves.push(pos);
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
  let n_cell = world[n_pos];
  let chain_begin = get_chain_begin(pos, board);
  if !is_water_or_food(n_cell) && !is_players_ant(n_cell, 0) && board[n_pos].ant == 0 && n_pos != chain_begin {
    if dangerous_place[n_pos] == 0 {
      if !escape || n_cell == Anthill(0) {
        moves.push(n_pos);
      }
      escape = true;
    } else {
      moves.push(n_pos);
    }
  }
  let w_cell = world[w_pos];
  if !is_water_or_food(w_cell) && !is_players_ant(w_cell, 0) && board[w_pos].ant == 0 && w_pos != chain_begin {
    if dangerous_place[w_pos] == 0 {
      if !escape || w_cell == Anthill(0) {
        moves.push(w_pos);
      }
      escape = true;
    } else {
      moves.push(w_pos);
    }
  }
  let s_cell = world[s_pos];
  if !is_water_or_food(s_cell) && !is_players_ant(s_cell, 0) && board[s_pos].ant == 0 && s_pos != chain_begin {
    if dangerous_place[s_pos] == 0 {
      if !escape || s_cell == Anthill(0) {
        moves.push(s_pos);
      }
      escape = true;
    } else {
      moves.push(s_pos);
    }
  }
  let e_cell = world[e_pos];
  if !is_water_or_food(e_cell) && !is_players_ant(e_cell, 0) && board[e_pos].ant == 0 && e_pos != chain_begin {
    if dangerous_place[e_pos] == 0 {
      if !escape || e_cell == Anthill(0) {
        moves.push(e_pos);
      }
    } else {
      moves.push(e_pos);
    }
  }
}

fn minimax_min(width: uint, height: uint, idx: uint, minimax_moved: &mut DList<uint>, enemies: &Vec<uint>, world: &Vec<Cell>, dangerous_place_for_enemies: &Vec<uint>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, alpha: int, start_time: u64, turn_time: uint, aggression: uint) -> int {
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
      let cur_estimate = minimax_min(width, height, idx + 1, minimax_moved, enemies, world, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, start_time, turn_time, aggression);
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
    estimate(width, height, world, attack_radius2, minimax_moved, board, tags, tagged, aggression)
  }
}

fn minimax_max(width: uint, height: uint, idx: uint, minimax_moved: &mut DList<uint>, ours: &Vec<uint>, enemies: &mut Vec<uint>, world: &Vec<Cell>, dangerous_place: &Vec<uint>, dangerous_place_for_enemies: &mut Vec<uint>, attack_radius2: uint, board: &mut Vec<BoardCell>, standing_ants: &Vec<uint>, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>, alpha: &mut int, aggression: uint, start_time: u64, turn_time: uint, best_moves: &mut Vec<uint>) {
  if idx < ours.len() {
    let pos = ours[idx];
    let mut moves = DList::new();
    get_our_moves(width, height, pos, world, dangerous_place, board, &mut moves);
    for &next_pos in moves.iter() {
      if elapsed_time(start_time) + MINIMAX_CRITICAL_TIME > turn_time { return; }
      minimax_moved.push(next_pos);
      board.get_mut(next_pos).ant = 1;
      board.get_mut(next_pos).cycle = pos + 1;
      minimax_max(width, height, idx + 1, minimax_moved, ours, enemies, world, dangerous_place, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, alpha, aggression, start_time, turn_time, best_moves);
      board.get_mut(next_pos).ant = 0;
      board.get_mut(next_pos).cycle = 0;
      minimax_moved.pop();
    }
  } else {
    for &ant_pos in minimax_moved.iter() {
      simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| {
        if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
          *dangerous_place_for_enemies.get_mut(pos) += 1;
          true
        } else {
          false
        }
      }, |_, _, _| { false });
      clear_tags(tags, tagged);
    }
    enemies.sort_by(|&pos1, &pos2| {
      let pos1_dangerous = (*dangerous_place_for_enemies)[pos1] > 0;
      let pos2_dangerous = (*dangerous_place_for_enemies)[pos2] > 0;
      if pos1_dangerous && !pos2_dangerous {
        Less
      } else if !pos1_dangerous && pos2_dangerous {
        Greater
      } else if pos1_dangerous && pos2_dangerous {
        get_escape_moves_count(width, height, pos1, world, dangerous_place_for_enemies).cmp(&get_escape_moves_count(width, height, pos2, world, dangerous_place_for_enemies))
      } else {
        Equal
      }
    });
    let cur_estimate = minimax_min(width, height, 0, minimax_moved, enemies, world, dangerous_place_for_enemies, attack_radius2, board, standing_ants, tags, tagged, *alpha, start_time, turn_time, aggression);
    for &ant_pos in minimax_moved.iter() {
      simple_wave(width, height, tags, tagged, ant_pos, |pos, _, _| {
        if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
          *dangerous_place_for_enemies.get_mut(pos) = 0;
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
    }, |pos, _, _| { pos != ant_pos && is_players_ant(world[pos], 0) });
    clear_tags(tags, tagged);
    if !result.is_none() {
      return false;
    }
  }
  true
}

fn log_ants<'r, T: Iterator<&'r uint>>(ants: &mut T) -> Box<DList<uint>> {
  let mut result = box DList::new();
  for &ant in *ants {
    result.push(ant);
  }
  result
}

fn attack<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  colony.log.push(Attack);
  let mut ours = Vec::new();
  let mut enemies = Vec::new();
  let mut minimax_moved = DList::new();
  let mut best_moves = Vec::new();
  let mut group_index = 1;
  for &pos in colony.ours_ants.iter() {
    if elapsed_time(colony.start_time) + MINIMAX_CRITICAL_TIME > colony.turn_time { return; }
    if colony.moved[pos] || colony.groups[pos] != 0 {
      continue;
    }
    let ours_moves_count = get_group(colony.width, colony.height, pos, colony.attack_radius2, &colony.world, &colony.moved, &colony.dangerous_place, &colony.standing_ants, &mut colony.groups, group_index, &mut colony.tags, &mut colony.tagged, &mut ours, &mut enemies);
    group_index += 1;
    if !enemies.is_empty() {
      if ours.len() == 1 && colony.aggressive_place[ours[0]] == 0 && is_alone(colony.width, colony.height, colony.attack_radius2, &colony.world, ours[0], &enemies, &mut colony.tags, &mut colony.tagged) { //TODO: fix colony.aggressive_place[ours[0]] == 0
        colony.alone_ants.push(ours[0]);
        continue;
      }
      colony.log.push(Group(group_index));
      colony.log.push(GroupSize(ours_moves_count, enemies.len()));
      let mut alpha = int::MIN;
      let mut aggression = 0u;
      for &pos in ours.iter() {
        if colony.aggressive_place[pos] > aggression {
          aggression = colony.aggressive_place[pos];
        }
      }
      colony.log.push(Aggression(aggression));
      ours.sort_by(|&pos1, &pos2| {
        let pos1_dangerous = colony.dangerous_place[pos1] > 0;
        let pos2_dangerous = colony.dangerous_place[pos2] > 0;
        if pos1_dangerous && !pos2_dangerous {
          Less
        } else if !pos1_dangerous && pos2_dangerous {
          Greater
        } else if pos1_dangerous && pos2_dangerous {
          get_escape_moves_count(colony.width, colony.height, pos1, &colony.world, &colony.dangerous_place).cmp(&get_escape_moves_count(colony.width, colony.height, pos2, &colony.world, &colony.dangerous_place))
        } else {
          Equal
        }
      });
      for &pos in ours.iter() {
        remove_ant(&mut colony.world, pos);
      }
      colony.log.push(OursAnts(log_ants(&mut ours.iter())));
      colony.log.push(EnemiesAnts(log_ants(&mut enemies.iter())));
      minimax_max(colony.width, colony.height, 0, &mut minimax_moved, &ours, &mut enemies, &colony.world, &colony.dangerous_place, &mut colony.tmp, colony.attack_radius2, &mut colony.board, &colony.standing_ants, &mut colony.tags, &mut colony.tagged, &mut alpha, aggression, colony.start_time, colony.turn_time, &mut best_moves);
      colony.log.push(Estimate(alpha));
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
        for &pos in enemies.iter() {
          *colony.fighting.get_mut(pos) = true;
        }
      }
    }
  }
}

fn escape_estimate(width: uint, height: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, estimate_pos: uint, tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) -> int {
  let mut estimate = 0;
  simple_wave(width, height, tags, tagged, estimate_pos, |pos, path_size, _| {
    let cell = world[pos];
    if path_size > ESCAPE_PATH_SIZE || cell == Water {
      false
    } else {
      estimate += (ESCAPE_PATH_SIZE + 1 - path_size) as int * if is_enemy_ant(cell) { //TODO: Move to constants.
        -7
      } else if is_players_ant(cell, 0) {
        7
      } else if cell == Food {
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

fn escape<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  let mut moves = Vec::with_capacity(5);
  let mut safe_moves = Vec::with_capacity(5);
  for &ant_pos in colony.alone_ants.iter() {
    moves.clear();
    safe_moves.clear();
    moves.push(ant_pos);
    let n_pos = n(colony.width, colony.height, ant_pos);
    let s_pos = s(colony.width, colony.height, ant_pos);
    let w_pos = w(colony.width, ant_pos);
    let e_pos = e(colony.width, ant_pos);
    if is_free(colony.world[n_pos]) {
      moves.push(n_pos);
    }
    if is_free(colony.world[s_pos]) {
      moves.push(s_pos);
    }
    if is_free(colony.world[w_pos]) {
      moves.push(w_pos);
    }
    if is_free(colony.world[e_pos]) {
      moves.push(e_pos);
    }
    if moves.is_empty() {
      *colony.moved.get_mut(ant_pos) = true;
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
      move_one(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, ant_pos, next_pos);
    } else {
      *colony.moved.get_mut(ant_pos) = true;
    }
  }
}

fn approach_enemies<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  let width = colony.width;
  let height = colony.height;
  let approach_path_size = colony.max_attack_radius_manhattan + APPROACH_PATH_SIZE_CONST;
  let fighting = &colony.fighting;
  let dangerous_place = &colony.dangerous_place;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, &mut colony.enemies_ants.iter().filter(|&&pos| { fighting[pos] }), |pos, _, path_size, prev| {
    if path_size > approach_path_size {
      return false;
    }
    let cell = (*world)[pos];
    if !is_free(cell) {
      if is_players_ant(cell, 0) && !(*moved)[pos] {
        if dangerous_place[prev] == 0 {
          move_one(width, height, world, moved, output, pos, prev);
        } else {
          *moved.get_mut(pos) = true;
        }
        true
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
    *aggressive_place.get_mut(pos) = NEIGHBORS_AGGRESSION[neighbors];
  }
  if colony.ours_anthills.len() > DANGEROUS_ANTHILLS_COUNT {
    return;
  }
  wave(colony.width, colony.height, &mut colony.tags, &mut colony.tagged, &mut colony.ours_anthills.iter(), |pos, _, path_size, _| {
    if path_size <= OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE {
      *aggressive_place.get_mut(pos) = cmp::max((*aggressive_place)[pos], OURS_ANTHILLS_AGGRESSION);
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
    let n_pos = n(colony.width, colony.height, ant_pos);
    let s_pos = s(colony.width, colony.height, ant_pos);
    let w_pos = w(colony.width, ant_pos);
    let e_pos = e(colony.width, ant_pos);
    let n_pos_water_or_food = is_water_or_food(colony.world[n_pos]);
    let s_pos_water_or_food = is_water_or_food(colony.world[s_pos]);
    let w_pos_water_or_food = is_water_or_food(colony.world[w_pos]);
    let e_pos_water_or_food = is_water_or_food(colony.world[e_pos]);
    simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 ||
         euclidean(width, height, n_pos, pos) <= attack_radius2 && !n_pos_water_or_food ||
         euclidean(width, height, s_pos, pos) <= attack_radius2 && !s_pos_water_or_food ||
         euclidean(width, height, w_pos, pos) <= attack_radius2 && !w_pos_water_or_food ||
         euclidean(width, height, e_pos, pos) <= attack_radius2 && !e_pos_water_or_food {
        *dangerous_place.get_mut(pos) += 1;
        true
      } else {
        false
      }
    }, |_, _, _| { false });
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
}

fn defend_anhills<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
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
      let cell = (*world)[pos];
      if path_size > DEFEND_ANTHILLS_PATH_SIZE || cell == Water {
        false
      } else {
        if is_enemy_ant(cell) {
          enemies_ants.push(pos);
        }
        true
      }
    }, |_, _, _| { false });
    for &ant_pos in enemies_ants.iter() {
      find_inverse_path(&colony.tags, anthill_pos, ant_pos, &mut path);
      let mut maybe_defender = None;
      for &pos in path.iter() {
        if is_players_ant((*world)[pos], 0) && (*tmp)[pos] == 0 {
          maybe_defender = Some(pos);
          break;
        }
        let n_pos = n(colony.width, colony.height, pos);
        if is_players_ant((*world)[n_pos], 0) && (*tmp)[n_pos] == 0 {
          maybe_defender = Some(n_pos);
          break;
        }
        let w_pos = w(colony.width, pos);
        if is_players_ant((*world)[w_pos], 0) && (*tmp)[w_pos] == 0 {
          maybe_defender = Some(w_pos);
          break;
        }
        let s_pos = s(colony.width, colony.height, pos);
        if is_players_ant((*world)[s_pos], 0) && (*tmp)[s_pos] == 0 {
          maybe_defender = Some(s_pos);
          break;
        }
        let e_pos = e(colony.width, pos);
        if is_players_ant((*world)[e_pos], 0) && (*tmp)[e_pos] == 0 {
          maybe_defender = Some(e_pos);
          break;
        }
      }
      if maybe_defender.is_none() {
        let three_fourth_pos = path[path.len() * 3 / 4];
        maybe_defender = simple_wave(colony.width, colony.height, &mut colony.tags2, &mut colony.tagged2, three_fourth_pos, |pos, path_size, _| {
          path_size <= DEFENDER_PATH_SIZE && (*world)[pos] != Water
        }, |pos, _, _| { is_players_ant((*world)[pos], 0) && (*tmp)[pos] == 0 });
        clear_tags(&mut colony.tags2, &mut colony.tagged2);
      }
      if maybe_defender.is_none() {
        continue;
      }
      defended = true;
      let defender = maybe_defender.unwrap();
      defenders.push(defender);
      *tmp.get_mut(defender) = 1;
      if colony.moved[defender] {
        continue;
      }
      let center_pos = path[path.len() / 2];
      if defender == center_pos {
        *colony.moved.get_mut(defender) = true;
        continue;
      }
      if simple_wave(colony.width, colony.height, &mut colony.tags2, &mut colony.tagged2, defender, |pos, _, prev| { //TODO: A*.
        let cell = (*world)[pos];
        pos == defender || cell != Water && (prev != defender || is_free(cell) && dangerous_place[pos] == 0)
      }, |pos, _, _| { pos == center_pos }).is_none() {
        *colony.moved.get_mut(defender) = true;
        clear_tags(&mut colony.tags2, &mut colony.tagged2);
        continue;
      }
      let mut defender_path = DList::new();
      find_path(&colony.tags2, defender, center_pos, &mut defender_path);
      clear_tags(&mut colony.tags2, &mut colony.tagged2);
      let next_pos = *defender_path.front().unwrap();
      move_one(colony.width, colony.height, world, &mut colony.moved, output, defender, next_pos);
      defenders.push(next_pos);
      *tmp.get_mut(next_pos) = 1;
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
    *tmp.get_mut(defender) = 0;
  }
}

fn get_random_move(width: uint, height: uint, world: &Vec<Cell>, dangerous_place: &Vec<uint>, rng: &mut XorShiftRng, pos: uint) -> uint {
  let mut moves = Vec::new();
  moves.push(pos);
  let n_pos = n(width, height, pos);
  if is_free(world[n_pos]) && dangerous_place[n_pos] == 0 {
    moves.push(n_pos);
  }
  let w_pos = w(width, pos);
  if is_free(world[w_pos]) && dangerous_place[w_pos] == 0 {
    moves.push(w_pos);
  }
  let s_pos = s(width, height, pos);
  if is_free(world[s_pos]) && dangerous_place[s_pos] == 0 {
    moves.push(s_pos);
  }
  let e_pos = e(width, pos);
  if is_free(world[e_pos]) && dangerous_place[e_pos] == 0 {
    moves.push(e_pos);
  }
  moves[rng.gen_range(0, moves.len())]
}

fn move_random<T: MutableSeq<Step>>(colony: &mut Colony, output: &mut T) {
  for &ant_pos in colony.ours_ants.iter() {
    if colony.moved[ant_pos] {
      continue;
    }
    let next_pos = get_random_move(colony.width, colony.height, &colony.world, &colony.dangerous_place, &mut colony.rng, ant_pos);
    if next_pos != ant_pos {
      move_one(colony.width, colony.height, &mut colony.world, &mut colony.moved, output, ant_pos, next_pos);
    } else {
      *colony.moved.get_mut(ant_pos) = true;
    }
  }
}

fn shuffle(colony: &mut Colony) {
  colony.rng.shuffle(colony.ours_ants.as_mut_slice());
  colony.rng.shuffle(colony.enemies_ants.as_mut_slice());
  colony.ours_anthills.sort();
  colony.enemies_anthills.sort();
}

fn update_world<'r, T: Iterator<&'r Input>>(colony: &mut Colony, input: &mut T) {
  let view_radius2 = colony.view_radius2;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let width = colony.width;
  let height = colony.height;
  let visible_area = &mut colony.visible_area;
  let discovered_area = &mut colony.discovered_area;
  let world = &mut colony.world;
  let standing_ants = &mut colony.standing_ants;
  let len = length(width, height);
  for pos in range(0u, len) {
    *colony.last_world.get_mut(pos) = (*world)[pos];
    *world.get_mut(pos) = Unknown;
    *colony.moved.get_mut(pos) = false;
    *colony.gathered_food.get_mut(pos) = 0;
    *visible_area.get_mut(pos) += 1;
    *discovered_area.get_mut(pos) += 1;
    *colony.territory.get_mut(pos) = 0;
    *colony.groups.get_mut(pos) = 0;
    *colony.dangerous_place.get_mut(pos) = 0;
    *colony.aggressive_place.get_mut(pos) = 0;
    *colony.fighting.get_mut(pos) = false;
  }
  colony.ours_ants.clear();
  colony.enemies_ants.clear();
  colony.enemies_anthills.clear();
  colony.ours_anthills.clear();
  colony.food.clear();
  colony.alone_ants.clear();
  for &i in *input {
    match i {
      InputWater(point) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos) = Water;
      },
      InputFood(point) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos) = Food;
        colony.food.push(pos);
      },
      InputAnthill(point, player) => {
        let pos = to_pos(width, point);
        *world.get_mut(pos) = if (*world)[pos] == Ant(player) { AnthillWithAnt(player) } else { Anthill(player) };
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
        *world.get_mut(pos) = if (*world)[pos] == Anthill(player) { AnthillWithAnt(player) } else { Ant(player) };
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
    simple_wave(width, height, &mut colony.tags, &mut colony.tagged, ant_pos, |pos, _, _| {
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
      if (*world)[pos] == Unknown {
        *world.get_mut(pos) = match colony.last_world[pos] {
          Water => Water,
          _ => Land
        }
      }
      if is_enemy_ant((*world)[pos]) {
        *standing_ants.get_mut(pos) += 1;
      } else {
        *standing_ants.get_mut(pos) = 0;
      }
    } else {
      *world.get_mut(pos) = match colony.last_world[pos] {
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
      *standing_ants.get_mut(pos) = 0;
    }
  }
  for pos in range(0u, len) {
    if is_enemy_ant(colony.last_world[pos]) != is_enemy_ant((*world)[pos]) {
      simple_wave(width, height, &mut colony.tags, &mut colony.tagged, pos, |near_pos, _, _| {
        if is_enemy_ant((*world)[near_pos]) {
          *standing_ants.get_mut(near_pos) = 1;
          true
        } else {
          near_pos == pos
        }
      }, |_, _, _| { false });
      clear_tags(&mut colony.tags, &mut colony.tagged);
    }
  }
}

fn is_timeout(start_time: u64, turn_time: uint, log: &mut DList<LogMessage>) -> bool {
  if elapsed_time(start_time) + CRITICAL_TIME > turn_time {
    log.push(Timeout);
    true
  } else {
    false
  }
}

pub fn turn<'r, T1: Iterator<&'r Input>, T2: MutableSeq<Step>>(colony: &mut Colony, input: &mut T1, output: &mut T2) {
  colony.start_time = get_time();
  output.clear();
  colony.cur_turn += 1;
  colony.log.push(Turn(colony.cur_turn));
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  update_world(colony, input);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  shuffle(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  calculate_dangerous_place(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  calculate_aggressive_place(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  attack_anthills(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  gather_food(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  attack(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  defend_anhills(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  approach_enemies(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  escape(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  discover(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  travel(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) { return; }
  move_random(colony, output);
}
