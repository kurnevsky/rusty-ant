// TODO: аггрессивность, если игра долго идет, а противник известен только один (с учетом контроллируемой территории).
// TODO: динамический подбор констатнт минимакса путем определения производительности на этапе инициализации. Динамическое уменьшение этих констант при таймаутах.
// TODO: захват муравейников вместо уничтожения.
// TODO: три варианта атаки:
// TODO: 1. для каждого противника вычислять минимаксом его осторожный ход (как это сейчас для себя делаем), а затем вычислить свой лучший ход на эти ходы.
// TODO: 2. попыка вычислить итеративно точку равновесие Нэша - для своих стоящих на месте муравьев вычислить лучший ход противника, затем для этого лучшего хода вычислить наш лучший ход, и так далее. Делать так либо пока не достигнут предел итераций либо лучшие ответы перестанут меняться.
// TODO: 3. в минимаксе вычислять не худший для нас, а лучший для врага ход. При этом придется отказаться от альфа-отсечния.

use crate::cell::*;
use crate::coordinates::*;
use crate::input::*;
use crate::log::*;
use crate::step::*;
use crate::time::*;
use crate::wave::*;
use rand::{Rng, SeedableRng};
use rand::seq::SliceRandom;
use rand_xorshift::XorShiftRng;
use std::{
  cmp::{self, Ordering},
  collections::VecDeque,
};

const TERRITORY_PATH_SIZE_CONST: u32 = 5;

const APPROACH_PATH_SIZE_CONST: u32 = 6;

const ESCAPE_PATH_SIZE: u32 = 8;

/// Maximum manhattan distance from food to our ant when this ant will go to it.
const GATHERING_FOOD_PATH_SIZE: u32 = 30;

/// Maximum manhattan distance from the nearest our ant to enemy's anthill when
/// this ant will go to it.
const ATTACK_ANTHILLS_PATH_SIZE: u32 = 20;

const DEFEND_ANTHILLS_PATH_SIZE: u32 = 15;

const DEFENDER_PATH_SIZE: u32 = 20;

/// Maximum number of our ants that attack enemy's anthills.
const ATTACK_ANTHILLS_ANTS_COUNT: u32 = 2;

/// If we have more than this number of anthills don't protect them at all.
const DANGEROUS_ANTHILLS_COUNT: u32 = 3;

/// Maximum number of our attacked anthills that we will defend.
const DEFEND_ANTHILLS_COUNT: u32 = 2;

/// If we have left time for turn less then this number of milliseconds then we
/// stop minimax and continue calculating moves with other methods.
const MINIMAX_CRITICAL_TIME: u32 = 100;

/// If we have left time for turn less then this number of milliseconds then we
/// stop doing anything but returning moves that are made already.
const CRITICAL_TIME: u32 = 50;

/// Number of dead hostile ants will be multiplied by this constant when we
/// estimate the position.
const ENEMIES_DEAD_ESTIMATION: &[u32] = &[3000, 4000, 6000, 9000, 12000, 18000];

/// Number of dead our ants will be multiplied by this constant when we estimate
/// the position.
const OURS_DEAD_ESTIMATION: &[u32] = &[6000, 6000, 6000, 6000, 6000, 6000];

/// Number of our ants that are located at a food gathering distance will be
/// multiplied by this constant when we estimate the position.
const OUR_FOOD_ESTIMATION: &[u32] = &[2000, 2000, 2000, 2000, 2000, 2000];

/// Number of hostile ants that are located at a food gathering distance will be
/// multiplied by this constant when we estimate the position.
const ENEMY_FOOD_ESTIMATION: &[u32] = &[1000, 1500, 2000, 3000, 4000, 6000];

const DESTROYED_ENEMY_ANTHILL_ESTIMATION: &[u32] = &[50000, 50000, 50000, 50000, 50000, 50000];

const DESTROYED_OUR_ANTHILL_ESTIMATION: &[u32] = &[50000, 50000, 50000, 50000, 50000, 50000];

const DISTANCE_TO_ENEMIES_ESTIMATION: &[u32] = &[1, 1, 1, 1, 1, 1];

/// If an ant stays in the same cell more than this number of turns we expect
/// that it won't move anymore.
const STANDING_ANTS_CONST: u32 = 4;

const STANDING_ANTS_WITH_CHANGED_ENVIRONMENT_CONST: u32 = 4;

/// Aggression levels depending on number of neighbors.
const NEIGHBORS_AGGRESSION: &[u32] = &[0, 0, 1, 1, 1, 2, 3, 4, 5];

/// Maximum manhattan distance from our anthill to our ant when it is considered
/// aggressive (and the whole group with it).
const OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE: u32 = 6;

/// Aggression level for our ants that are close to our anthills.
const OURS_ANTHILLS_AGGRESSION: u32 = 2;

const ENEMY_ANT_ESCAPE_CONST: i32 = -7;

const OURS_ANT_ESCAPE_CONST: i32 = 7;

const FOOD_ESCAPE_CONST: i32 = 3;

const SAFE_PLACE_ESCAPE_CONST: i32 = 1;

const LOG_CAPACITY_CONST: u32 = 100;

#[derive(Clone)]
struct BoardCell {
  /// Index of player whose ant made a move in the current cell, +1.
  ant: u32,
  /// Number of enemies that attack the ant.
  attack: u32,
  /// Cell from which the ant made a move to the current cell. Needed for
  /// preventing ants to move in a cycle.
  cycle: Pos,
  /// Mark for our or enemy's dead in battle ants.
  dead: bool,
}

#[derive(Clone)]
pub struct Colony {
  /// Field width.
  width: u32,
  /// Field height.
  height: u32,
  /// Time for one turn.
  turn_time: u32,
  /// Number of turns in the game.
  turns_count: u32,
  view_radius2: u32,
  attack_radius2: u32,
  spawn_radius2: u32,
  /// Current turn number.
  cur_turn: u32,
  /// Time when the current turn started.
  start_time: u64,
  /// Random numbers generator.
  rng: XorShiftRng,
  min_view_radius_manhattan: u32,
  max_view_radius_manhattan: u32,
  max_attack_radius_manhattan: u32,
  /// Known number of enemies.
  enemies_count: u32,
  /// Current state of the world. When our ant makes a move it sifts to a new
  /// cell.
  world: Vec<Cell>,
  /// Previous state of the world with our ants made a move.
  last_world: Vec<Cell>,
  /// 0 for visible cells and water, otherwise incremented by 1 before each
  /// turn.
  visible_area: Vec<u32>,
  discovered_area: Vec<u32>,
  /// Every turn increases by 1 for hostile ants (if their neighborhood is
  /// unchanged) and resets to 0 for all other cells. It means that this value
  /// shows how long a hostile ant stand still.
  standing_ants: Vec<u32>,
  /// All cells from or to which our ant made a move will be marked with this
  /// flag.
  moved: Vec<bool>,
  /// Cell with food to which our ant is sent. The value is ant coordinate + 1.
  gathered_food: Vec<Pos>,
  territory: Vec<u32>,
  /// Number of hostile ants that can attack the cell on the next turn.
  dangerous_place: Vec<u32>,
  aggressive_place: Vec<u32>,
  groups: Vec<u32>,
  fighting: Vec<bool>,
  board: Vec<BoardCell>,
  tmp: Vec<u32>,
  alone_ants: Vec<Pos>,
  /// Tags for the Lee algorithm.
  tags: Vec<Tag>,
  /// List of coordinates start_tags and path_tags with non zero values.
  tagged: Vec<Pos>,
  /// Second tags for the Lee algorithm.
  tags2: Vec<Tag>,
  /// List of coordinates start_tags and path_tags in tags2 with non zero
  /// values.
  tagged2: Vec<Pos>,
  /// List of our ants. If an ant made a move the cell is marked as moved.
  ours_ants: Vec<Pos>,
  enemies_ants: Vec<Pos>,
  enemies_anthills: Vec<Pos>,
  /// List of cells with our anthills (both visible and beyond the fog of war).
  ours_anthills: Vec<Pos>,
  /// List of cells with food (both visible and beyond the fog of war).
  food: Vec<Pos>,
  log: Vec<LogMessage>,
}

impl Colony {
  pub fn new(
    width: u32,
    height: u32,
    turn_time: u32,
    turns_count: u32,
    view_radius2: u32,
    attack_radius2: u32,
    spawn_radius2: u32,
    seed: u64,
  ) -> Colony {
    let len = length(width, height);
    let seed_array = [
      2,
      (seed & 0xff) as u8,
      3,
      ((seed >> 8) & 0xff) as u8,
      5,
      ((seed >> 16) & 0xff) as u8,
      7,
      ((seed >> 24) & 0xff) as u8,
      11,
      ((seed >> 32) & 0xff) as u8,
      13,
      ((seed >> 40) & 0xff) as u8,
      17,
      ((seed >> 48) & 0xff) as u8,
      19,
      ((seed >> 56) & 0xff) as u8,
    ];
    Colony {
      width,
      height,
      turn_time,
      turns_count,
      view_radius2,
      attack_radius2,
      spawn_radius2,
      cur_turn: 0,
      start_time: get_time(),
      rng: XorShiftRng::from_seed(seed_array),
      min_view_radius_manhattan: (view_radius2 as f32).sqrt() as u32,
      max_view_radius_manhattan: ((view_radius2 * 2) as f32).sqrt() as u32,
      max_attack_radius_manhattan: ((attack_radius2 * 2) as f32).sqrt() as u32,
      enemies_count: 0,
      world: vec![Cell::Unknown; len],
      last_world: vec![Cell::Unknown; len],
      visible_area: vec![0; len],
      discovered_area: vec![0; len],
      standing_ants: vec![0; len],
      moved: vec![false; len],
      gathered_food: vec![0; len],
      territory: vec![0; len],
      dangerous_place: vec![0; len],
      aggressive_place: vec![0; len],
      groups: vec![0; len],
      fighting: vec![false; len],
      board: vec![
        BoardCell {
          ant: 0,
          attack: 0,
          cycle: 0,
          dead: false,
        };
        len
      ],
      tmp: vec![0; len],
      alone_ants: Vec::with_capacity(len),
      tags: vec![Tag::new(); len],
      tagged: Vec::with_capacity(len),
      tags2: vec![Tag::new(); len],
      tagged2: Vec::with_capacity(len),
      ours_ants: Vec::with_capacity(len),
      enemies_ants: Vec::with_capacity(len),
      enemies_anthills: Vec::with_capacity(len),
      ours_anthills: Vec::with_capacity(len),
      food: Vec::with_capacity(len),
      log: Vec::with_capacity((turns_count * LOG_CAPACITY_CONST) as usize),
    }
  }

  pub fn log(&self) -> &Vec<LogMessage> {
    &self.log
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn cur_turn(&self) -> u32 {
    self.cur_turn
  }
}

fn remove_ant(world: &mut Vec<Cell>, pos: Pos) {
  world[pos] = match world[pos] {
    Cell::AnthillWithAnt(player) => Cell::Anthill(player),
    _ => Cell::Land,
  };
}

fn set_ant(world: &mut Vec<Cell>, pos: Pos, player: u32) {
  world[pos] = if world[pos] == Cell::Anthill(player) {
    Cell::AnthillWithAnt(player)
  } else {
    Cell::Ant(player)
  };
}

fn move_one(
  width: u32,
  height: u32,
  world: &mut Vec<Cell>,
  moved: &mut Vec<bool>,
  output: &mut Vec<Step>,
  pos: Pos,
  next_pos: Pos,
  log: &mut Vec<LogMessage>,
) {
  if !is_players_ant(world[pos], 0) {
    log.push(LogMessage::Multitask(pos, next_pos));
    return;
  }
  if let Some(direction) = to_direction(width, height, pos, next_pos) {
    remove_ant(world, pos);
    moved[pos] = true;
    set_ant(world, next_pos, 0);
    moved[next_pos] = true;
    output.push(Step {
      point: from_pos(width, pos),
      direction,
    })
  } else {
    log.push(LogMessage::Jump(pos, next_pos));
  }
}

fn move_all(
  width: u32,
  height: u32,
  world: &mut Vec<Cell>,
  moved: &mut Vec<bool>,
  output: &mut Vec<Step>,
  moves: &[(Pos, Pos)],
  log: &mut Vec<LogMessage>,
) {
  for &(pos, next_pos) in moves {
    if is_players_ant(world[pos], 0) {
      remove_ant(world, pos);
      moved[pos] = true;
    } else {
      log.push(LogMessage::Multitask(pos, next_pos));
    }
  }
  for &(pos, next_pos) in moves {
    if let Some(direction) = to_direction(width, height, pos, next_pos) {
      set_ant(world, next_pos, 0);
      moved[next_pos] = true;
      output.push(Step {
        point: from_pos(width, pos),
        direction,
      });
    } else {
      log.push(LogMessage::Jump(pos, next_pos));
    }
  }
}

fn discover_direction(
  width: u32,
  height: u32,
  min_view_radius_manhattan: u32,
  world: &[Cell],
  discovered_area: &[u32],
  dangerous_place: &[u32],
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  ant_pos: Pos,
  rng: &mut XorShiftRng,
) -> Option<Pos> {
  let mut n_score = 0;
  let mut s_score = 0;
  let mut w_score = 0;
  let mut e_score = 0;
  let n_pos = n(width, height, ant_pos);
  let s_pos = s(width, height, ant_pos);
  let w_pos = w(width, ant_pos);
  let e_pos = e(width, ant_pos);
  if is_free(world[n_pos]) && dangerous_place[n_pos] == 0 {
    simple_wave(
      width,
      height,
      tags,
      tagged,
      n_pos,
      |pos, path_size, prev| {
        if pos == s(width, height, prev)
          || path_size > manhattan(width, height, n_pos, pos)
          || manhattan(width, height, n_pos, pos) > min_view_radius_manhattan
          || world[pos] == Cell::Water
        {
          false
        } else {
          if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
            n_score += discovered_area[pos];
          }
          true
        }
      },
      |_, _, _| false,
    );
    clear_tags(tags, tagged);
  }
  if is_free(world[s_pos]) && dangerous_place[s_pos] == 0 {
    simple_wave(
      width,
      height,
      tags,
      tagged,
      s_pos,
      |pos, path_size, prev| {
        if pos == n(width, height, prev)
          || path_size > manhattan(width, height, s_pos, pos)
          || manhattan(width, height, s_pos, pos) > min_view_radius_manhattan
          || world[pos] == Cell::Water
        {
          false
        } else {
          if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
            s_score += discovered_area[pos];
          }
          true
        }
      },
      |_, _, _| false,
    );
    clear_tags(tags, tagged);
  }
  if is_free(world[w_pos]) && dangerous_place[w_pos] == 0 {
    simple_wave(
      width,
      height,
      tags,
      tagged,
      w_pos,
      |pos, path_size, prev| {
        if pos == e(width, prev)
          || path_size > manhattan(width, height, w_pos, pos)
          || manhattan(width, height, w_pos, pos) > min_view_radius_manhattan
          || world[pos] == Cell::Water
        {
          false
        } else {
          if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
            w_score += discovered_area[pos];
          }
          true
        }
      },
      |_, _, _| false,
    );
    clear_tags(tags, tagged);
  }
  if is_free(world[e_pos]) && dangerous_place[e_pos] == 0 {
    simple_wave(
      width,
      height,
      tags,
      tagged,
      e_pos,
      |pos, path_size, prev| {
        if pos == w(width, prev)
          || path_size > manhattan(width, height, e_pos, pos)
          || manhattan(width, height, e_pos, pos) > min_view_radius_manhattan
          || world[pos] == Cell::Water
        {
          false
        } else {
          if manhattan(width, height, ant_pos, pos) > min_view_radius_manhattan {
            e_score += discovered_area[pos];
          }
          true
        }
      },
      |_, _, _| false,
    );
    clear_tags(tags, tagged);
  }
  if n_score == 0 && s_score == 0 && w_score == 0 && e_score == 0 {
    // It'd be better to calculate the distance to our ants but it requires too
    // many calculations so we choose move randomly.
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

fn discover(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::Discover);
  let width = colony.width;
  let height = colony.height;
  let min_view_radius_manhattan = colony.min_view_radius_manhattan;
  let discovered_area = &mut colony.discovered_area;
  for &pos in &colony.ours_ants {
    if colony.moved[pos] {
      continue;
    }
    if let Some(next_pos) = discover_direction(
      width,
      height,
      min_view_radius_manhattan,
      &colony.world,
      discovered_area,
      &colony.dangerous_place,
      &mut colony.tags,
      &mut colony.tagged,
      pos,
      &mut colony.rng,
    ) {
      simple_wave(
        width,
        height,
        &mut colony.tags,
        &mut colony.tagged,
        next_pos,
        |pos, _, _| {
          if manhattan(width, height, pos, next_pos) <= min_view_radius_manhattan {
            discovered_area[pos] = 0;
            true
          } else {
            false
          }
        },
        |_, _, _| false,
      );
      clear_tags(&mut colony.tags, &mut colony.tagged);
      move_one(
        colony.width,
        colony.height,
        &mut colony.world,
        &mut colony.moved,
        output,
        pos,
        next_pos,
        &mut colony.log,
      );
      colony.log.push(LogMessage::Goal(pos, next_pos));
    }
  }
}

fn travel(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::Travel);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let territory = &colony.territory;
  let moved = &mut colony.moved;
  let tmp = &mut colony.tmp;
  let dangerous_place = &colony.dangerous_place;
  let mut path = Vec::new();
  for &ant_pos in &colony.ours_ants {
    if moved[ant_pos] {
      continue;
    }
    tmp[ant_pos] = 1;
    let goal_option = simple_wave(
      width,
      height,
      &mut colony.tags,
      &mut colony.tagged,
      ant_pos,
      |pos, _, prev| {
        let cell = world[pos];
        let is_column = tmp[prev];
        if cell == Cell::Water
          || is_column == 1
            && (moved[pos] && is_players_ant(cell, 0)
              || cell == Cell::Food
              || dangerous_place[pos] > 0 && pos != ant_pos)
        {
          false
        } else {
          tmp[pos] = if is_players_ant(cell, 0) { is_column } else { 0 };
          true
        }
      },
      |pos, _, _| territory[pos] != 1,
    );
    if let Some(goal) = goal_option {
      find_path(&colony.tags, ant_pos, goal, &mut path);
      for &pos in &colony.tagged {
        tmp[pos] = 0;
      }
      clear_tags(&mut colony.tags, &mut colony.tagged);
      let mut path_pos = ant_pos;
      let mut moves = Vec::new();
      for &pos in path.iter().rev() {
        moves.push((path_pos, pos));
        colony.log.push(LogMessage::Goal(path_pos, goal));
        if !is_players_ant(world[pos], 0) {
          break;
        }
        path_pos = pos;
      }
      move_all(width, height, world, moved, output, &moves, &mut colony.log);
    } else {
      for &pos in &colony.tagged {
        tmp[pos] = 0;
      }
      clear_tags(&mut colony.tags, &mut colony.tagged);
    }
  }
}

fn calculate_territory(colony: &mut Colony) {
  let territory_path_size = colony.max_view_radius_manhattan + TERRITORY_PATH_SIZE_CONST;
  let world = &colony.world;
  let territory = &mut colony.territory;
  wave(
    colony.width,
    colony.height,
    &mut colony.tags,
    &mut colony.tagged,
    &mut colony
      .ours_ants
      .iter()
      .chain(colony.enemies_ants.iter())
      .chain(colony.enemies_anthills.iter()),
    |pos, start_pos, path_size, _| {
      if path_size <= territory_path_size && world[pos] != Cell::Water {
        match world[start_pos] {
          Cell::AnthillWithAnt(player) | Cell::Ant(player) | Cell::Anthill(player) => territory[pos] = player + 1,
          _ => territory[pos] = 1,
        }
        true
      } else {
        false
      }
    },
    |_, _, _, _| false,
  );
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn attack_anthills(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::AttackAnthills);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  let dangerous_place = &colony.dangerous_place;
  let tmp = &mut colony.tmp;
  let log = &mut colony.log;
  wave(
    width,
    height,
    &mut colony.tags,
    &mut colony.tagged,
    &mut colony.enemies_anthills.iter(),
    |pos, start_pos, path_size, prev| {
      if pos != start_pos && dangerous_place[pos] > 0
        || path_size > ATTACK_ANTHILLS_PATH_SIZE
        || tmp[start_pos] > ATTACK_ANTHILLS_ANTS_COUNT
      {
        return false;
      }
      match world[pos] {
        Cell::Ant(0) | Cell::AnthillWithAnt(0) if !moved[pos] => {
          if !is_free(world[prev]) {
            false
          } else {
            tmp[start_pos] += 1;
            move_one(width, height, world, moved, output, pos, prev, log);
            log.push(LogMessage::Goal(pos, start_pos));
            true
          }
        }
        Cell::Unknown | Cell::Water => false,
        _ => true,
      }
    },
    |_, _, _, _| false,
  );
  clear_tags(&mut colony.tags, &mut colony.tagged);
  for &pos in &colony.enemies_anthills {
    tmp[pos] = 0;
  }
}

fn gather_food(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::GatherFood);
  let width = colony.width;
  let height = colony.height;
  let world = &mut colony.world;
  let gathered_food = &mut colony.gathered_food;
  let moved = &mut colony.moved;
  let dangerous_place = &colony.dangerous_place;
  let log = &mut colony.log;
  for &pos in &colony.ours_ants {
    if moved[pos] || dangerous_place[pos] > 0 {
      continue;
    }
    let n_pos = n(width, height, pos);
    if world[n_pos] == Cell::Food && gathered_food[n_pos] == 0 {
      moved[pos] = true;
      gathered_food[n_pos] = pos + 1;
      log.push(LogMessage::Goal(pos, n_pos));
    }
    let s_pos = s(width, height, pos);
    if world[s_pos] == Cell::Food && gathered_food[s_pos] == 0 {
      moved[pos] = true;
      gathered_food[s_pos] = pos + 1;
      log.push(LogMessage::Goal(pos, s_pos));
    }
    let w_pos = w(width, pos);
    if world[w_pos] == Cell::Food && gathered_food[w_pos] == 0 {
      moved[pos] = true;
      gathered_food[w_pos] = pos + 1;
      log.push(LogMessage::Goal(pos, w_pos));
    }
    let e_pos = e(width, pos);
    if world[e_pos] == Cell::Food && gathered_food[e_pos] == 0 {
      moved[pos] = true;
      gathered_food[e_pos] = pos + 1;
      log.push(LogMessage::Goal(pos, e_pos));
    }
  }
  wave(
    width,
    height,
    &mut colony.tags,
    &mut colony.tagged,
    &mut colony.food.iter(),
    |pos, start_pos, path_size, prev| {
      if pos != start_pos && dangerous_place[pos] > 0 || path_size > GATHERING_FOOD_PATH_SIZE {
        return false;
      }
      match world[pos] {
        Cell::Ant(0) | Cell::AnthillWithAnt(0) if gathered_food[start_pos] == 0 && !moved[pos] => {
          if pos != start_pos && !is_free(world[prev]) {
            false
          } else {
            move_one(width, height, world, moved, output, pos, prev, log);
            gathered_food[start_pos] = pos + 1;
            log.push(LogMessage::Goal(pos, start_pos));
            true
          }
        }
        Cell::Unknown | Cell::Water => false,
        _ => true,
      }
    },
    |_, _, _, _| false,
  );
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn in_one_group(
  width: u32,
  height: u32,
  pos1: Pos,
  pos2: Pos,
  pos1_standing: bool,
  attack_radius2: u32,
  world: &[Cell],
) -> bool {
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
  let n_pos1_busy = is_water_or_food(n_pos1_cell);
  let s_pos1_busy = is_water_or_food(s_pos1_cell);
  let w_pos1_busy = is_water_or_food(w_pos1_cell);
  let e_pos1_busy = is_water_or_food(e_pos1_cell);
  let n_pos2_busy = is_water_or_food(n_pos2_cell);
  let s_pos2_busy = is_water_or_food(s_pos2_cell);
  let w_pos2_busy = is_water_or_food(w_pos2_cell);
  let e_pos2_busy = is_water_or_food(e_pos2_cell);
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
  if pos1_standing {
    return false;
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

fn find_near_enemies(
  width: u32,
  height: u32,
  ant_pos: Pos,
  attack_radius2: u32,
  world: &[Cell],
  standing_ants: &[u32],
  groups: &mut Vec<u32>,
  group_index: u32,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  group: &mut VecDeque<Pos>,
) {
  simple_wave(
    width,
    height,
    tags,
    tagged,
    ant_pos,
    |pos, _, prev| {
      if groups[pos] == 0 && is_enemy_ant(world[pos]) {
        let standing = standing_ants[pos] > STANDING_ANTS_CONST;
        if in_one_group(width, height, pos, ant_pos, standing, attack_radius2, world) {
          group.push_back(pos);
          groups[pos] = group_index;
        }
      }
      euclidean(width, height, ant_pos, prev) <= attack_radius2
    },
    |_, _, _| false,
  );
  clear_tags(tags, tagged);
}

fn find_near_ours(
  width: u32,
  height: u32,
  ant_pos: Pos,
  standing: bool,
  attack_radius2: u32,
  world: &[Cell],
  moved: &[bool],
  groups: &mut Vec<u32>,
  group_index: u32,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  group: &mut VecDeque<Pos>,
) {
  simple_wave(
    width,
    height,
    tags,
    tagged,
    ant_pos,
    |pos, _, prev| {
      if groups[pos] == 0
        && is_players_ant(world[pos], 0)
        && !moved[pos]
        && in_one_group(width, height, ant_pos, pos, standing, attack_radius2, world)
      {
        group.push_back(pos);
        groups[pos] = group_index;
      }
      euclidean(width, height, ant_pos, if standing { pos } else { prev }) <= attack_radius2
    },
    |_, _, _| false,
  );
  clear_tags(tags, tagged);
}

fn group_enough(ours_moves_count: u32, enemies_count: u32) -> bool {
  ours_moves_count > 21 || ours_moves_count > 15 && enemies_count > 4 || ours_moves_count > 11 && enemies_count > 7
}

fn get_group(
  width: u32,
  height: u32,
  ant_pos: Pos,
  attack_radius2: u32,
  world: &[Cell],
  moved: &[bool],
  dangerous_place: &[u32],
  standing_ants: &[u32],
  groups: &mut Vec<u32>,
  group_index: u32,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  ours: &mut Vec<Pos>,
  enemies: &mut Vec<Pos>,
) -> u32 {
  ours.clear();
  enemies.clear();
  let mut ours_moves_count = 0;
  let mut enemies_count = 0;
  let mut ours_q = VecDeque::new();
  let mut enemies_q = VecDeque::new();
  ours_q.push_back(ant_pos);
  groups[ant_pos] = group_index;
  while !ours_q.is_empty() && !group_enough(ours_moves_count, enemies_count) {
    let pos = ours_q.pop_front().unwrap();
    ours.push(pos);
    ours_moves_count += get_moves_count(width, height, pos, world, dangerous_place);
    find_near_enemies(
      width,
      height,
      pos,
      attack_radius2,
      world,
      standing_ants,
      groups,
      group_index,
      tags,
      tagged,
      &mut enemies_q,
    );
    while let Some(pos) = enemies_q.pop_front() {
      enemies.push(pos);
      let standing = if standing_ants[pos] <= STANDING_ANTS_CONST {
        enemies_count += 1;
        false
      } else {
        true
      };
      find_near_ours(
        width,
        height,
        pos,
        standing,
        attack_radius2,
        world,
        moved,
        groups,
        group_index,
        tags,
        tagged,
        &mut ours_q,
      );
    }
  }
  for &pos in ours_q.iter().chain(enemies.iter()) {
    groups[pos] = 0;
  }
  ours_moves_count
}

fn is_near_food(width: u32, height: u32, world: &[Cell], pos: Pos) -> bool {
  // TODO: spawn_radius2
  world[n(width, height, pos)] == Cell::Food
    || world[s(width, height, pos)] == Cell::Food
    || world[w(width, pos)] == Cell::Food
    || world[e(width, pos)] == Cell::Food
}

fn is_dead(
  width: u32,
  height: u32,
  ant_pos: Pos,
  attack_radius2: u32,
  board: &[BoardCell],
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
) -> bool {
  let mut result = false;
  let attack_value = board[ant_pos].attack;
  let ant_number = board[ant_pos].ant;
  simple_wave(
    width,
    height,
    tags,
    tagged,
    ant_pos,
    |pos, _, _| euclidean(width, height, ant_pos, pos) <= attack_radius2,
    |pos, _, _| {
      let board_cell = &board[pos];
      if board_cell.ant != 0 && board_cell.ant != ant_number && board_cell.attack <= attack_value {
        result = true;
        true
      } else {
        false
      }
    },
  );
  clear_tags(tags, tagged);
  result
}

fn estimate(
  width: u32,
  height: u32,
  world: &[Cell],
  attack_radius2: u32,
  ants: &[Pos],
  other_ours: &[Pos],
  board: &mut Vec<BoardCell>,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  aggression: u32,
) -> i32 {
  let mut ours_dead_count = 0;
  let mut enemies_dead_count = 0;
  let mut our_food = 0;
  let mut enemy_food = 0;
  let mut destroyed_enemy_anthills = 0;
  let mut destroyed_our_anthills = 0;
  let mut distance_to_enemies = 0;
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    if board[ant_pos].ant == 0 {
      continue;
    }
    if is_dead(width, height, ant_pos, attack_radius2, board, tags, tagged) {
      board[ant_pos].dead = true;
    }
  }
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    let ant_board_cell = &board[ant_pos];
    if ant_board_cell.ant == 0 {
      continue;
    }
    if ant_board_cell.dead {
      if ant_board_cell.ant == 1 {
        ours_dead_count += 1;
      } else {
        enemies_dead_count += 1;
      }
    } else if ant_board_cell.ant == 1 {
      if is_near_food(width, height, world, ant_pos) {
        our_food += 1;
      }
      if is_enemy_anthill(world[ant_pos]) {
        destroyed_enemy_anthills += 1;
      }
      let mut min_distance_to_enemy = u32::max_value();
      for &enemy_pos in ants {
        let enemy_board_cell = &board[enemy_pos];
        if enemy_board_cell.ant < 2 || enemy_board_cell.dead {
          continue;
        }
        let cur_distance = euclidean(width, height, ant_pos, enemy_pos);
        if cur_distance < min_distance_to_enemy {
          min_distance_to_enemy = cur_distance;
        }
      }
      if min_distance_to_enemy != u32::max_value() {
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
  for &ant_pos in ants.iter().chain(other_ours.iter()) {
    board[ant_pos].dead = false;
  }
  (enemies_dead_count * ENEMIES_DEAD_ESTIMATION[aggression as usize]) as i32
    - (ours_dead_count * OURS_DEAD_ESTIMATION[aggression as usize]) as i32
    + (our_food * OUR_FOOD_ESTIMATION[aggression as usize]) as i32
    - (enemy_food * ENEMY_FOOD_ESTIMATION[aggression as usize]) as i32
    + (destroyed_enemy_anthills * DESTROYED_ENEMY_ANTHILL_ESTIMATION[aggression as usize]) as i32
    - (destroyed_our_anthills * DESTROYED_OUR_ANTHILL_ESTIMATION[aggression as usize]) as i32
    - (distance_to_enemies * DISTANCE_TO_ENEMIES_ESTIMATION[aggression as usize]) as i32 // TODO: штраф своему муравью за стояние на муравейнике. штраф своему муравью за стояние на одном месте. близость врага к муравейнику. точное вычисление того, кому достанется еда.
}

fn get_chain_begin(mut pos: Pos, board: &[BoardCell]) -> Pos {
  loop {
    let next_pos = board[pos].cycle;
    if next_pos == 0 {
      break;
    }
    pos = next_pos - 1;
  }
  pos
}

fn get_moves_count(width: u32, height: u32, pos: Pos, world: &[Cell], dangerous_place: &[u32]) -> u32 {
  let mut result = 1;
  let mut escape = dangerous_place[pos] == 0;
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

fn get_escape_moves_count(width: u32, height: u32, pos: Pos, world: &[Cell], dangerous_place: &[u32]) -> u32 {
  let mut result = 0;
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

fn get_our_moves(
  width: u32,
  height: u32,
  pos: Pos,
  world: &[Cell],
  dangerous_place: &[u32],
  board: &[BoardCell],
  moves: &mut Vec<Pos>,
) {
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
fn get_enemy_moves(
  width: u32,
  height: u32,
  pos: Pos,
  world: &[Cell],
  dangerous_place: &[u32],
  board: &[BoardCell],
  standing_ants: &[u32],
  moves: &mut Vec<Pos>,
) {
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
      if !escape || n_cell == Cell::Anthill(0) {
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
      if !escape || w_cell == Cell::Anthill(0) {
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
      if !escape || s_cell == Cell::Anthill(0) {
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
      if !escape || e_cell == Cell::Anthill(0) {
        moves.push(e_pos);
      }
    } else {
      moves.push(e_pos);
    }
  }
}

fn is_minimax_timeout(start_time: u64, turn_time: u32, log: &mut Vec<LogMessage>) -> bool {
  if elapsed_time(start_time) + MINIMAX_CRITICAL_TIME > turn_time {
    log.push(LogMessage::MinimaxTimeout);
    true
  } else {
    false
  }
}

fn minimax_min(
  width: u32,
  height: u32,
  idx: usize,
  minimax_moved: &mut Vec<Pos>,
  enemies: &[Pos],
  other_ours: &[Pos],
  world: &[Cell],
  dangerous_place_for_enemies: &[u32],
  attack_radius2: u32,
  board: &mut Vec<BoardCell>,
  standing_ants: &[u32],
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  alpha: i32,
  start_time: u64,
  turn_time: u32,
  aggression: u32,
  log: &mut Vec<LogMessage>,
) -> i32 {
  if idx < enemies.len() {
    let pos = enemies[idx];
    let mut moves = Vec::with_capacity(5);
    get_enemy_moves(
      width,
      height,
      pos,
      world,
      dangerous_place_for_enemies,
      board,
      standing_ants,
      &mut moves,
    );
    let mut min_estimation = i32::max_value();
    for &next_pos in &moves {
      if is_minimax_timeout(start_time, turn_time, log) {
        return i32::min_value();
      }
      minimax_moved.push(next_pos);
      let ant_number = ant_owner(world[pos]).unwrap() + 1;
      board[next_pos].ant = ant_number;
      board[next_pos].cycle = pos + 1;
      board[next_pos].attack = dangerous_place_for_enemies[next_pos];
      simple_wave(
        width,
        height,
        tags,
        tagged,
        next_pos,
        |pos, _, _| {
          if euclidean(width, height, next_pos, pos) <= attack_radius2 {
            if board[pos].ant != 0 && board[pos].ant != ant_number {
              board[pos].attack += 1;
              if board[pos].ant != 1 {
                board[next_pos].attack += 1;
              }
            }
            true
          } else {
            false
          }
        },
        |_, _, _| false,
      );
      clear_tags(tags, tagged);
      let cur_estimation = minimax_min(
        width,
        height,
        idx + 1,
        minimax_moved,
        enemies,
        other_ours,
        world,
        dangerous_place_for_enemies,
        attack_radius2,
        board,
        standing_ants,
        tags,
        tagged,
        alpha,
        start_time,
        turn_time,
        aggression,
        log,
      );
      simple_wave(
        width,
        height,
        tags,
        tagged,
        next_pos,
        |pos, _, _| {
          if euclidean(width, height, next_pos, pos) <= attack_radius2 {
            let board_cell = &mut board[pos];
            if board_cell.ant != 0 && board_cell.ant != ant_number {
              board_cell.attack -= 1;
            }
            true
          } else {
            false
          }
        },
        |_, _, _| false,
      );
      clear_tags(tags, tagged);
      board[next_pos].attack = 0;
      board[next_pos].ant = 0;
      board[next_pos].cycle = 0;
      minimax_moved.pop();
      if cur_estimation < min_estimation {
        min_estimation = cur_estimation;
        if cur_estimation <= alpha {
          break;
        }
      }
    }
    min_estimation
  } else {
    estimate(
      width,
      height,
      world,
      attack_radius2,
      minimax_moved,
      other_ours,
      board,
      tags,
      tagged,
      aggression,
    )
  }
}

fn minimax_max(
  width: u32,
  height: u32,
  idx: usize,
  minimax_moved: &mut Vec<Pos>,
  ours: &[Pos],
  enemies: &mut Vec<Pos>,
  other_ours: &[Pos],
  world: &[Cell],
  dangerous_place: &[u32],
  dangerous_place_for_enemies: &mut Vec<u32>,
  attack_radius2: u32,
  board: &mut Vec<BoardCell>,
  standing_ants: &[u32],
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  alpha: &mut i32,
  aggression: u32,
  start_time: u64,
  turn_time: u32,
  best_moves: &mut Vec<Pos>,
  log: &mut Vec<LogMessage>,
) {
  if idx < ours.len() {
    let pos = ours[idx];
    let mut moves = Vec::with_capacity(5);
    get_our_moves(width, height, pos, world, dangerous_place, board, &mut moves);
    for &next_pos in &moves {
      if is_minimax_timeout(start_time, turn_time, log) {
        return;
      }
      minimax_moved.push(next_pos);
      board[next_pos].ant = 1;
      board[next_pos].cycle = pos + 1;
      add_attack(
        width,
        height,
        attack_radius2,
        next_pos,
        dangerous_place_for_enemies,
        tags,
        tagged,
      );
      minimax_max(
        width,
        height,
        idx + 1,
        minimax_moved,
        ours,
        enemies,
        other_ours,
        world,
        dangerous_place,
        dangerous_place_for_enemies,
        attack_radius2,
        board,
        standing_ants,
        tags,
        tagged,
        alpha,
        aggression,
        start_time,
        turn_time,
        best_moves,
        log,
      );
      remove_attack(
        width,
        height,
        attack_radius2,
        next_pos,
        dangerous_place_for_enemies,
        tags,
        tagged,
      );
      board[next_pos].ant = 0;
      board[next_pos].cycle = 0;
      minimax_moved.pop();
    }
  } else {
    enemies.sort_by(|&pos1, &pos2| {
      let pos1_dangerous = dangerous_place_for_enemies[pos1] > 0;
      let pos2_dangerous = dangerous_place_for_enemies[pos2] > 0;
      if pos1_dangerous && !pos2_dangerous {
        Ordering::Less
      } else if !pos1_dangerous && pos2_dangerous {
        Ordering::Greater
      } else if pos1_dangerous && pos2_dangerous {
        let escape_moves_count_1 = get_escape_moves_count(width, height, pos1, world, dangerous_place_for_enemies);
        let escape_moves_count_2 = get_escape_moves_count(width, height, pos2, world, dangerous_place_for_enemies);
        escape_moves_count_1.cmp(&escape_moves_count_2)
      } else {
        Ordering::Equal
      }
    });
    let cur_estimation = minimax_min(
      width,
      height,
      0,
      minimax_moved,
      enemies,
      other_ours,
      world,
      dangerous_place_for_enemies,
      attack_radius2,
      board,
      standing_ants,
      tags,
      tagged,
      *alpha,
      start_time,
      turn_time,
      aggression,
      log,
    );
    if cur_estimation > *alpha {
      // TODO: среди всех одинаковых выбирать ту оценку, которая больше при условии, что враг останется на месте.
      *alpha = cur_estimation;
      best_moves.clear();
      for &pos in minimax_moved.iter() {
        best_moves.push(pos);
      }
    }
  }
}

fn is_alone(
  width: u32,
  height: u32,
  attack_radius2: u32,
  world: &[Cell],
  ant_pos: Pos,
  enemies: &[Pos],
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
) -> bool {
  for &enemy_pos in enemies {
    let result = simple_wave(
      width,
      height,
      tags,
      tagged,
      enemy_pos,
      |_, _, prev| euclidean(width, height, enemy_pos, prev) <= attack_radius2,
      |pos, _, _| pos != ant_pos && is_players_ant(world[pos], 0),
    );
    clear_tags(tags, tagged);
    if result.is_some() {
      return false;
    }
  }
  true
}

fn get_other_ours(
  width: u32,
  height: u32,
  world: &[Cell],
  standing_ants: &[u32],
  tmp: &mut Vec<u32>,
  attack_radius2: u32,
  enemies: &[Pos],
  other_ours: &mut Vec<Pos>,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
) {
  other_ours.clear();
  for &enemy_pos in enemies {
    let standing = standing_ants[enemy_pos] > STANDING_ANTS_CONST;
    simple_wave(
      width,
      height,
      tags,
      tagged,
      enemy_pos,
      |pos, _, prev| {
        if euclidean(width, height, enemy_pos, if standing { pos } else { prev }) <= attack_radius2 {
          if is_players_ant(world[pos], 0) && tmp[pos] == 0 {
            tmp[pos] = 1;
            other_ours.push(pos);
          }
          true
        } else {
          false
        }
      },
      |_, _, _| false,
    );
    clear_tags(tags, tagged);
  }
  for &ant_pos in other_ours.iter() {
    tmp[ant_pos] = 0;
  }
}

fn add_attack(
  width: u32,
  height: u32,
  attack_radius2: u32,
  ant_pos: Pos,
  attack_place: &mut Vec<u32>,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
) {
  simple_wave(
    width,
    height,
    tags,
    tagged,
    ant_pos,
    |pos, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
        attack_place[pos] += 1;
        true
      } else {
        false
      }
    },
    |_, _, _| false,
  );
  clear_tags(tags, tagged);
}

fn remove_attack(
  width: u32,
  height: u32,
  attack_radius2: u32,
  ant_pos: Pos,
  attack_place: &mut Vec<u32>,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
) {
  simple_wave(
    width,
    height,
    tags,
    tagged,
    ant_pos,
    |pos, _, _| {
      if euclidean(width, height, ant_pos, pos) <= attack_radius2 {
        attack_place[pos] -= 1;
        true
      } else {
        false
      }
    },
    |_, _, _| false,
  );
  clear_tags(tags, tagged);
}

fn attack(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::Attack);
  let mut ours = Vec::new();
  let mut other_ours = Vec::new();
  let mut enemies = Vec::new();
  let mut minimax_moved = Vec::new();
  let mut best_moves = Vec::new();
  let mut group_index = 0;
  for &pos in &colony.ours_ants {
    if colony.moved[pos] || colony.groups[pos] != 0 {
      continue;
    }
    group_index += 1;
    let ours_moves_count = get_group(
      colony.width,
      colony.height,
      pos,
      colony.attack_radius2,
      &colony.world,
      &colony.moved,
      &colony.dangerous_place,
      &colony.standing_ants,
      &mut colony.groups,
      group_index,
      &mut colony.tags,
      &mut colony.tagged,
      &mut ours,
      &mut enemies,
    );
    if !enemies.is_empty() {
      let mut aggression = 0;
      for &pos in &ours {
        if colony.aggressive_place[pos] > aggression {
          aggression = colony.aggressive_place[pos];
        }
      }
      if ours.len() == 1
        && ENEMIES_DEAD_ESTIMATION[aggression as usize] < OURS_DEAD_ESTIMATION[aggression as usize]
        && is_alone(
          colony.width,
          colony.height,
          colony.attack_radius2,
          &colony.world,
          ours[0],
          &enemies,
          &mut colony.tags,
          &mut colony.tagged,
        )
      {
        colony.alone_ants.push(ours[0]);
        continue;
      }
      if is_minimax_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
        continue;
      }
      colony.log.push(LogMessage::Group(group_index));
      colony
        .log
        .push(LogMessage::GroupSize(ours_moves_count, enemies.len() as u32));
      let mut alpha = i32::min_value();
      colony.log.push(LogMessage::Aggression(aggression));
      ours.sort_by(|&pos1, &pos2| {
        let pos1_dangerous = colony.dangerous_place[pos1] > 0;
        let pos2_dangerous = colony.dangerous_place[pos2] > 0;
        if pos1_dangerous && !pos2_dangerous {
          Ordering::Less
        } else if !pos1_dangerous && pos2_dangerous {
          Ordering::Greater
        } else if pos1_dangerous && pos2_dangerous {
          let escape_moves_count_1 = get_escape_moves_count(
            colony.width,
            colony.height,
            pos1,
            &colony.world,
            &colony.dangerous_place,
          );
          let escape_moves_count_2 = get_escape_moves_count(
            colony.width,
            colony.height,
            pos2,
            &colony.world,
            &colony.dangerous_place,
          );
          escape_moves_count_1.cmp(&escape_moves_count_2)
        } else {
          Ordering::Equal
        }
      });
      for &pos in &ours {
        remove_ant(&mut colony.world, pos);
      }
      colony.log.push(LogMessage::OursAnts(ours.clone()));
      colony.log.push(LogMessage::EnemiesAnts(enemies.clone()));
      get_other_ours(
        colony.width,
        colony.height,
        &colony.world,
        &colony.standing_ants,
        &mut colony.tmp,
        colony.attack_radius2,
        &enemies,
        &mut other_ours,
        &mut colony.tags,
        &mut colony.tagged,
      );
      colony.log.push(LogMessage::OtherOursAnts(other_ours.clone()));
      for &pos in &other_ours {
        add_attack(
          colony.width,
          colony.height,
          colony.attack_radius2,
          pos,
          &mut colony.tmp,
          &mut colony.tags,
          &mut colony.tagged,
        );
        colony.board[pos].ant = 1;
      }
      minimax_max(
        colony.width,
        colony.height,
        0,
        &mut minimax_moved,
        &ours,
        &mut enemies,
        &other_ours,
        &colony.world,
        &colony.dangerous_place,
        &mut colony.tmp,
        colony.attack_radius2,
        &mut colony.board,
        &colony.standing_ants,
        &mut colony.tags,
        &mut colony.tagged,
        &mut alpha,
        aggression,
        colony.start_time,
        colony.turn_time,
        &mut best_moves,
        &mut colony.log,
      );
      colony.log.push(LogMessage::Estimate(alpha));
      for &pos in &other_ours {
        remove_attack(
          colony.width,
          colony.height,
          colony.attack_radius2,
          pos,
          &mut colony.tmp,
          &mut colony.tags,
          &mut colony.tagged,
        );
        colony.board[pos].ant = 0;
      }
      for &pos in &ours {
        set_ant(&mut colony.world, pos, 0);
      }
      if alpha != i32::min_value() {
        let mut moves = Vec::with_capacity(ours.len());
        for (i, &pos) in ours.iter().enumerate() {
          let next_pos = best_moves[i];
          if pos == next_pos {
            colony.moved[pos] = true;
          } else {
            moves.push((pos, next_pos));
          }
        }
        move_all(
          colony.width,
          colony.height,
          &mut colony.world,
          &mut colony.moved,
          output,
          &moves,
          &mut colony.log,
        );
        for &pos in &enemies {
          colony.fighting[pos] = true;
        }
      }
    }
  }
}

fn escape_estimation(
  width: u32,
  height: u32,
  world: &[Cell],
  dangerous_place: &[u32],
  estimate_pos: Pos,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
) -> i32 {
  let mut estimation = 0;
  simple_wave(
    width,
    height,
    tags,
    tagged,
    estimate_pos,
    |pos, path_size, _| {
      let cell = world[pos];
      if path_size > ESCAPE_PATH_SIZE || cell == Cell::Water {
        false
      } else {
        estimation += (ESCAPE_PATH_SIZE + 1 - path_size) as i32
          * if is_enemy_ant(cell) {
            ENEMY_ANT_ESCAPE_CONST
          } else if is_players_ant(cell, 0) {
            OURS_ANT_ESCAPE_CONST
          } else if cell == Cell::Food {
            FOOD_ESCAPE_CONST
          } else if dangerous_place[pos] == 0 {
            SAFE_PLACE_ESCAPE_CONST
          } else {
            0
          };
        true
      }
    },
    |_, _, _| false,
  );
  clear_tags(tags, tagged);
  estimation
}

fn escape(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::Escape);
  let mut moves = Vec::with_capacity(5);
  let mut safe_moves = Vec::with_capacity(5);
  for &ant_pos in &colony.alone_ants {
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
      colony.moved[ant_pos] = true;
      colony.log.push(LogMessage::Goal(ant_pos, ant_pos));
      continue;
    }
    for &pos in &moves {
      if colony.dangerous_place[pos] == 0 {
        safe_moves.push(pos);
      }
    }
    let mut next_pos;
    if safe_moves.is_empty() {
      next_pos = moves[0];
      let mut min_danger = colony.dangerous_place[next_pos];
      for &pos in moves.iter().skip(1) {
        let cur_danger = colony.dangerous_place[pos];
        if cur_danger < min_danger || cur_danger == min_danger && colony.rng.gen() {
          min_danger = cur_danger;
          next_pos = pos;
        }
      }
    } else {
      next_pos = safe_moves[0];
      let mut max_estimation = escape_estimation(
        colony.width,
        colony.height,
        &colony.world,
        &colony.dangerous_place,
        next_pos,
        &mut colony.tags,
        &mut colony.tagged,
      );
      for &pos in safe_moves.iter().skip(1) {
        let cur_estimation = escape_estimation(
          colony.width,
          colony.height,
          &colony.world,
          &colony.dangerous_place,
          pos,
          &mut colony.tags,
          &mut colony.tagged,
        );
        if cur_estimation > max_estimation || cur_estimation == max_estimation && colony.rng.gen() {
          max_estimation = cur_estimation;
          next_pos = pos;
        }
      }
    }
    if next_pos != ant_pos {
      move_one(
        colony.width,
        colony.height,
        &mut colony.world,
        &mut colony.moved,
        output,
        ant_pos,
        next_pos,
        &mut colony.log,
      );
      colony.log.push(LogMessage::Goal(ant_pos, next_pos));
    } else {
      colony.moved[ant_pos] = true;
      colony.log.push(LogMessage::Goal(ant_pos, ant_pos));
    }
  }
}

fn approach_enemies(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::ApproachEnemies);
  let width = colony.width;
  let height = colony.height;
  let approach_path_size = colony.max_attack_radius_manhattan + APPROACH_PATH_SIZE_CONST;
  let fighting = &colony.fighting;
  let dangerous_place = &colony.dangerous_place;
  let world = &mut colony.world;
  let moved = &mut colony.moved;
  let log = &mut colony.log;
  wave(
    colony.width,
    colony.height,
    &mut colony.tags,
    &mut colony.tagged,
    &mut colony.enemies_ants.iter().filter(|&&pos| fighting[pos]),
    |pos, start_pos, path_size, prev| {
      if path_size > approach_path_size {
        return false;
      }
      let cell = world[pos];
      if !is_free(cell) {
        if is_players_ant(cell, 0) && !moved[pos] {
          log.push(LogMessage::Goal(pos, start_pos));
          if dangerous_place[prev] == 0 {
            move_one(width, height, world, moved, output, pos, prev, log);
            true
          } else {
            moved[pos] = true;
            false
          }
        } else {
          false
        }
      } else {
        true
      }
    },
    |_, _, _, _| false,
  );
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn calculate_aggressive_place(colony: &mut Colony) {
  let aggressive_place = &mut colony.aggressive_place;
  for &pos in &colony.ours_ants {
    let mut neighbors = 0;
    if is_players_ant(colony.world[n(colony.width, colony.height, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[w(colony.width, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[s(colony.width, colony.height, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[e(colony.width, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[nw(colony.width, colony.height, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[ne(colony.width, colony.height, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[sw(colony.width, colony.height, pos)], 0) {
      neighbors += 1;
    }
    if is_players_ant(colony.world[se(colony.width, colony.height, pos)], 0) {
      neighbors += 1;
    }
    aggressive_place[pos] = NEIGHBORS_AGGRESSION[neighbors];
  }
  if colony.ours_anthills.len() as u32 > DANGEROUS_ANTHILLS_COUNT {
    return;
  }
  wave(
    colony.width,
    colony.height,
    &mut colony.tags,
    &mut colony.tagged,
    &mut colony.ours_anthills.iter(),
    |pos, _, path_size, _| {
      if path_size <= OURS_ANTHILLS_PATH_SIZE_FOR_AGGRESSIVE {
        aggressive_place[pos] = cmp::max(aggressive_place[pos], OURS_ANTHILLS_AGGRESSION);
        true
      } else {
        false
      }
    },
    |_, _, _, _| false,
  );
  clear_tags(&mut colony.tags, &mut colony.tagged);
}

fn calculate_dangerous_place(colony: &mut Colony) {
  let width = colony.width;
  let height = colony.height;
  let attack_radius2 = colony.attack_radius2;
  let dangerous_place = &mut colony.dangerous_place;
  for &ant_pos in &colony.enemies_ants {
    let not_standing = colony.standing_ants[ant_pos] <= STANDING_ANTS_CONST;
    let n_pos = n(colony.width, colony.height, ant_pos);
    let s_pos = s(colony.width, colony.height, ant_pos);
    let w_pos = w(colony.width, ant_pos);
    let e_pos = e(colony.width, ant_pos);
    let n_pos_not_water_nor_food = !is_water_or_food(colony.world[n_pos]);
    let s_pos_not_water_nor_food = !is_water_or_food(colony.world[s_pos]);
    let w_pos_not_water_nor_food = !is_water_or_food(colony.world[w_pos]);
    let e_pos_not_water_nor_food = !is_water_or_food(colony.world[e_pos]);
    simple_wave(
      width,
      height,
      &mut colony.tags,
      &mut colony.tagged,
      ant_pos,
      |pos, _, _| {
        if euclidean(width, height, ant_pos, pos) <= attack_radius2
          || not_standing
            && (n_pos_not_water_nor_food && euclidean(width, height, n_pos, pos) <= attack_radius2
              || s_pos_not_water_nor_food && euclidean(width, height, s_pos, pos) <= attack_radius2
              || w_pos_not_water_nor_food && euclidean(width, height, w_pos, pos) <= attack_radius2
              || e_pos_not_water_nor_food && euclidean(width, height, e_pos, pos) <= attack_radius2)
        {
          dangerous_place[pos] += 1;
          true
        } else {
          false
        }
      },
      |_, _, _| false,
    );
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
}

fn defend_anhills(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::DefendAnthills);
  if colony.ours_anthills.len() as u32 > DANGEROUS_ANTHILLS_COUNT {
    return;
  }
  let world = &mut colony.world;
  let dangerous_place = &colony.dangerous_place;
  let tmp = &mut colony.tmp;
  let mut defended_anhills = 0;
  let mut path = Vec::new();
  let mut defenders = Vec::new();
  for &anthill_pos in &colony.ours_anthills {
    let mut defended = false;
    let mut enemies_ants = Vec::new();
    simple_wave(
      colony.width,
      colony.height,
      &mut colony.tags,
      &mut colony.tagged,
      anthill_pos,
      |pos, path_size, _| {
        let cell = world[pos];
        if path_size > DEFEND_ANTHILLS_PATH_SIZE || cell == Cell::Water {
          false
        } else {
          if is_enemy_ant(cell) {
            enemies_ants.push(pos);
          }
          true
        }
      },
      |_, _, _| false,
    );
    for &ant_pos in &enemies_ants {
      find_path(&colony.tags, anthill_pos, ant_pos, &mut path);
      let mut maybe_defender = None;
      for &pos in &path {
        if is_players_ant(world[pos], 0) && tmp[pos] == 0 {
          maybe_defender = Some(pos);
          break;
        }
        let n_pos = n(colony.width, colony.height, pos);
        if is_players_ant(world[n_pos], 0) && tmp[n_pos] == 0 {
          maybe_defender = Some(n_pos);
          break;
        }
        let w_pos = w(colony.width, pos);
        if is_players_ant(world[w_pos], 0) && tmp[w_pos] == 0 {
          maybe_defender = Some(w_pos);
          break;
        }
        let s_pos = s(colony.width, colony.height, pos);
        if is_players_ant(world[s_pos], 0) && tmp[s_pos] == 0 {
          maybe_defender = Some(s_pos);
          break;
        }
        let e_pos = e(colony.width, pos);
        if is_players_ant(world[e_pos], 0) && tmp[e_pos] == 0 {
          maybe_defender = Some(e_pos);
          break;
        }
      }
      if maybe_defender.is_none() {
        let three_fourth_pos = path[path.len() * 3 / 4];
        maybe_defender = simple_wave(
          colony.width,
          colony.height,
          &mut colony.tags2,
          &mut colony.tagged2,
          three_fourth_pos,
          |pos, path_size, _| path_size <= DEFENDER_PATH_SIZE && world[pos] != Cell::Water,
          |pos, _, _| is_players_ant(world[pos], 0) && tmp[pos] == 0,
        );
        clear_tags(&mut colony.tags2, &mut colony.tagged2);
      }
      if let Some(defender) = maybe_defender {
        defended = true;
        colony.log.push(LogMessage::Defender(anthill_pos, ant_pos, defender));
        defenders.push(defender);
        tmp[defender] = 1;
        if colony.moved[defender] {
          continue;
        }
        let center_pos = path[path.len() / 2];
        if defender == center_pos {
          colony.moved[defender] = true;
          colony.log.push(LogMessage::Goal(defender, defender));
          continue;
        }
        let path_not_found = simple_wave(
          colony.width,
          colony.height,
          &mut colony.tags2,
          &mut colony.tagged2,
          defender,
          |pos, _, prev| {
            // TODO: A*.
            let cell = world[pos];
            pos == defender || cell != Cell::Water && (prev != defender || is_free(cell) && dangerous_place[pos] == 0)
          },
          |pos, _, _| pos == center_pos,
        )
        .is_none();
        if path_not_found {
          clear_tags(&mut colony.tags2, &mut colony.tagged2);
          colony.moved[defender] = true;
          colony.log.push(LogMessage::Goal(defender, defender));
          continue;
        }
        let mut defender_path = Vec::with_capacity(colony.tags2[center_pos].length() as usize - 1);
        find_path(&colony.tags2, defender, center_pos, &mut defender_path);
        clear_tags(&mut colony.tags2, &mut colony.tagged2);
        let next_pos = *defender_path.last().unwrap();
        move_one(
          colony.width,
          colony.height,
          world,
          &mut colony.moved,
          output,
          defender,
          next_pos,
          &mut colony.log,
        );
        colony.log.push(LogMessage::Goal(defender, center_pos));
        defenders.push(next_pos);
        tmp[next_pos] = 1;
      }
    }
    clear_tags(&mut colony.tags, &mut colony.tagged);
    if defended {
      defended_anhills += 1;
      if defended_anhills > DEFEND_ANTHILLS_COUNT {
        break;
      }
    }
  }
  for &defender in &defenders {
    tmp[defender] = 0;
  }
}

fn get_random_move(
  width: u32,
  height: u32,
  world: &[Cell],
  dangerous_place: &[u32],
  rng: &mut XorShiftRng,
  pos: Pos,
) -> Pos {
  let mut moves = Vec::with_capacity(5);
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

fn move_random(colony: &mut Colony, output: &mut Vec<Step>) {
  colony.log.push(LogMessage::MoveRandom);
  for &ant_pos in &colony.ours_ants {
    if colony.moved[ant_pos] {
      continue;
    }
    let next_pos = get_random_move(
      colony.width,
      colony.height,
      &colony.world,
      &colony.dangerous_place,
      &mut colony.rng,
      ant_pos,
    );
    if next_pos != ant_pos {
      move_one(
        colony.width,
        colony.height,
        &mut colony.world,
        &mut colony.moved,
        output,
        ant_pos,
        next_pos,
        &mut colony.log,
      );
      colony.log.push(LogMessage::Goal(ant_pos, next_pos));
    } else {
      colony.moved[ant_pos] = true;
      colony.log.push(LogMessage::Goal(ant_pos, ant_pos));
    }
  }
}

fn shuffle(colony: &mut Colony) {
  colony.ours_ants.shuffle(&mut colony.rng);
  colony.enemies_ants.shuffle(&mut colony.rng);
  colony.ours_anthills.sort_unstable();
  colony.enemies_anthills.sort_unstable();
}

fn update_world(colony: &mut Colony, input: &[Input]) {
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
  for pos in 0..len {
    last_world[pos] = world[pos];
    world[pos] = Cell::Unknown;
    colony.moved[pos] = false;
    colony.gathered_food[pos] = 0;
    visible_area[pos] += 1;
    discovered_area[pos] += 1;
    colony.territory[pos] = 0;
    colony.groups[pos] = 0;
    colony.dangerous_place[pos] = 0;
    colony.aggressive_place[pos] = 0;
    colony.fighting[pos] = false;
  }
  colony.ours_ants.clear();
  colony.enemies_ants.clear();
  colony.enemies_anthills.clear();
  colony.ours_anthills.clear();
  colony.food.clear();
  colony.alone_ants.clear();
  for i in input {
    match *i {
      Input::Water(point) => {
        let pos = to_pos(width, point);
        world[pos] = Cell::Water;
      }
      Input::Food(point) => {
        let pos = to_pos(width, point);
        world[pos] = Cell::Food;
        colony.food.push(pos);
      }
      Input::Anthill(point, player) => {
        let pos = to_pos(width, point);
        world[pos] = if world[pos] == Cell::Ant(player) {
          Cell::AnthillWithAnt(player)
        } else {
          Cell::Anthill(player)
        };
        if player == 0 {
          colony.ours_anthills.push(pos);
        } else {
          colony.enemies_anthills.push(pos);
          if player > colony.enemies_count {
            colony.enemies_count = player;
          }
        }
      }
      Input::Ant(point, player) => {
        let pos = to_pos(width, point);
        world[pos] = if world[pos] == Cell::Anthill(player) {
          Cell::AnthillWithAnt(player)
        } else {
          Cell::Ant(player)
        };
        if player == 0 {
          colony.ours_ants.push(pos);
        } else {
          colony.enemies_ants.push(pos);
          if player > colony.enemies_count {
            colony.enemies_count = player;
          }
        }
      }
      Input::Dead(..) => {}
    }
  }
  for &ant_pos in &colony.ours_ants {
    simple_wave(
      width,
      height,
      &mut colony.tags,
      &mut colony.tagged,
      ant_pos,
      |pos, _, _| {
        if euclidean(width, height, pos, ant_pos) <= view_radius2 {
          if manhattan(width, height, pos, ant_pos) <= min_view_radius_manhattan {
            discovered_area[pos] = 0;
          }
          visible_area[pos] = 0;
          true
        } else {
          false
        }
      },
      |_, _, _| false,
    );
    clear_tags(&mut colony.tags, &mut colony.tagged);
  }
  for pos in 0..len {
    if visible_area[pos] == 0 {
      if world[pos] == Cell::Unknown {
        world[pos] = match last_world[pos] {
          Cell::Water => Cell::Water,
          _ => Cell::Land,
        }
      }
      if is_enemy_ant(world[pos]) {
        colony.standing_ants[pos] += 1;
      } else {
        colony.standing_ants[pos] = 0;
      }
    } else {
      world[pos] = match last_world[pos] {
        Cell::Water => {
          visible_area[pos] = 0;
          Cell::Water
        }
        Cell::Food => {
          colony.food.push(pos);
          Cell::Food
        }
        Cell::Unknown => Cell::Unknown,
        Cell::Land | Cell::Ant(0) => Cell::Land,
        Cell::Anthill(0) | Cell::AnthillWithAnt(0) => {
          colony.ours_anthills.push(pos);
          Cell::Anthill(0)
        }
        Cell::Ant(player) => {
          colony.enemies_ants.push(pos);
          Cell::Ant(player)
        }
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
      colony.standing_ants[pos] = 0;
    }
  }
  for ant_pos in 0..len {
    if colony.standing_ants[ant_pos] > STANDING_ANTS_CONST {
      let environment_change = simple_wave(
        width,
        height,
        &mut colony.tags,
        &mut colony.tagged,
        ant_pos,
        |_, _, prev| euclidean(width, height, prev, ant_pos) <= attack_radius2,
        |pos, _, _| last_world[pos] != Cell::Unknown && last_world[pos] != world[pos],
      );
      if environment_change.is_some() {
        colony.standing_ants[ant_pos] = STANDING_ANTS_WITH_CHANGED_ENVIRONMENT_CONST;
      }
      clear_tags(&mut colony.tags, &mut colony.tagged);
    }
  }
}

fn is_timeout(start_time: u64, turn_time: u32, log: &mut Vec<LogMessage>) -> bool {
  if elapsed_time(start_time) + CRITICAL_TIME > turn_time {
    log.push(LogMessage::Timeout);
    true
  } else {
    false
  }
}

pub fn turn(colony: &mut Colony, input: &[Input], output: &mut Vec<Step>) {
  colony.start_time = get_time();
  output.clear();
  colony.cur_turn += 1;
  colony.log.push(LogMessage::Turn(colony.cur_turn));
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  update_world(colony, input);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  shuffle(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  calculate_dangerous_place(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  attack_anthills(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  gather_food(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  calculate_aggressive_place(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  attack(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  escape(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  defend_anhills(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  approach_enemies(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  discover(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  calculate_territory(colony);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  travel(colony, output);
  if is_timeout(colony.start_time, colony.turn_time, &mut colony.log) {
    return;
  }
  move_random(colony, output);
}
