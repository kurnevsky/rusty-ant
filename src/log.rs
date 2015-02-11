use std::collections::DList;
use coordinates::*;

#[derive(Clone, PartialEq)]
pub enum LogMessage {
  Turn(uint),
  Attack,
  AttackAnthills,
  GatherFood,
  Discover,
  Travel,
  MoveRandom,
  Escape,
  ApproachEnemies,
  DefendAnthills,
  Group(uint),
  Aggression(uint),
  Estimate(int),
  OursAnts(Box<DList<uint>>),
  OtherOursAnts(Box<DList<uint>>),
  EnemiesAnts(Box<DList<uint>>),
  GroupSize(uint, uint),
  Goal(uint, uint),
  Defender(uint, uint, uint),
  Timeout,
  MinimaxTimeout,
  Multitask(uint, uint),
  Jump(uint, uint)
}

fn write_pos<T: Writer>(width: uint, pos: uint, writer: &mut T) {
  let point = from_pos(width, pos);
  writer.write_uint(point.y).ok();
  writer.write_str(":").ok();
  writer.write_uint(point.x).ok();
}

fn write_ants<T: Writer>(width: uint, ants: &DList<uint>, writer: &mut T) {
  for &pos in ants.iter() {
    write_pos(width, pos, writer);
    writer.write_str(" ").ok();
  }
}

pub fn write_log<T: Writer>(width: uint, log: &DList<LogMessage>, writer: &mut T) {
  for log_message in log.iter() {
    match *log_message {
      LogMessage::Turn(turn) => {
        writer.write_str("Turn number: ").ok();
        writer.write_uint(turn).ok();
        writer.write_line("").ok();
      },
      LogMessage::Attack => {
        writer.write_line("  Attack.").ok();
      },
      LogMessage::AttackAnthills => {
        writer.write_line("  Attack anthills.").ok();
      },
      LogMessage::GatherFood => {
        writer.write_line("  Gather food.").ok();
      },
      LogMessage::Discover => {
        writer.write_line("  Discover.").ok();
      },
      LogMessage::Travel => {
        writer.write_line("  Travel.").ok();
      },
      LogMessage::MoveRandom => {
        writer.write_line("  Move random.").ok();
      },
      LogMessage::Escape => {
        writer.write_line("  Escape.").ok();
      },
      LogMessage::ApproachEnemies => {
        writer.write_line("  Approach enemies.").ok();
      },
      LogMessage::DefendAnthills => {
        writer.write_line("  Defend anthills.").ok();
      },
      LogMessage::Group(group_index) => {
        writer.write_str("    Group number: ").ok();
        writer.write_uint(group_index).ok();
        writer.write_line("").ok();
      },
      LogMessage::Aggression(aggression) => {
        writer.write_str("    Aggression level: ").ok();
        writer.write_uint(aggression).ok();
        writer.write_line("").ok();
      },
      LogMessage::Estimate(estimate) => {
        writer.write_str("    Estimation: ").ok();
        writer.write_int(estimate).ok();
        writer.write_line("").ok();
      },
      LogMessage::OursAnts(ref ants) => {
        writer.write_str("    Ours ants: ").ok();
        write_ants(width, &**ants, writer);
        writer.write_line("").ok();
      },
      LogMessage::OtherOursAnts(ref ants) => {
        writer.write_str("    Other ours ants: ").ok();
        write_ants(width, &**ants, writer);
        writer.write_line("").ok();
      },
      LogMessage::EnemiesAnts(ref ants) => {
        writer.write_str("    Enemies ants: ").ok();
        write_ants(width, &**ants, writer);
        writer.write_line("").ok();
      },
      LogMessage::GroupSize(ours_moves_count, enemies_count) => {
        writer.write_str("    Group size: ").ok();
        writer.write_uint(ours_moves_count).ok();
        writer.write_str(" our moves; ").ok();
        writer.write_uint(enemies_count).ok();
        writer.write_str(" enemies.").ok();
        writer.write_line("").ok();
      },
      LogMessage::Goal(ant_pos, goal_pos) => {
        writer.write_str("    Ours ant ").ok();
        write_pos(width, ant_pos, writer);
        writer.write_str(" has goal ").ok();
        write_pos(width, goal_pos, writer);
        writer.write_line(".").ok();
      },
      LogMessage::Defender(anthill_pos, enemy_pos, ant_pos) => {
        writer.write_str("    Ours anthill ").ok();
        write_pos(width, anthill_pos, writer);
        writer.write_str(" has defender ").ok();
        write_pos(width, ant_pos, writer);
        writer.write_str(" from enemy ").ok();
        write_pos(width, enemy_pos, writer);
        writer.write_line(".").ok();
      },
      LogMessage::Timeout => {
        writer.write_line("  Timeout.").ok();
      },
      LogMessage::MinimaxTimeout => {
        writer.write_line("    Minimax timeout.").ok();
      },
      LogMessage::Multitask(ant_pos, next_pos) => {
        writer.write_str("    Multitask from ").ok();
        write_pos(width, ant_pos, writer);
        writer.write_str(" to ").ok();
        write_pos(width, next_pos, writer);
        writer.write_line(".").ok();
      },
      LogMessage::Jump(ant_pos, next_pos) => {
        writer.write_str("    Jump from ").ok();
        write_pos(width, ant_pos, writer);
        writer.write_str(" to ").ok();
        write_pos(width, next_pos, writer);
        writer.write_line(".").ok();
      }
    }
  }
}
