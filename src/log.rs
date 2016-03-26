use std::io::Write;
use coordinates::*;

#[derive(Clone, PartialEq)]
pub enum LogMessage {
  Turn(u32),
  Attack,
  AttackAnthills,
  GatherFood,
  Discover,
  Travel,
  MoveRandom,
  Escape,
  ApproachEnemies,
  DefendAnthills,
  Group(u32),
  Aggression(u32),
  Estimate(i32),
  OursAnts(Vec<Pos>),
  OtherOursAnts(Vec<Pos>),
  EnemiesAnts(Vec<Pos>),
  GroupSize(u32, u32),
  Goal(Pos, Pos),
  Defender(Pos, Pos, Pos),
  Timeout,
  MinimaxTimeout,
  Multitask(Pos, Pos),
  Jump(Pos, Pos)
}

fn write_pos<T: Write>(width: u32, pos: Pos, writer: &mut T) {
  let point = from_pos(width, pos);
  write!(writer, "{0}:{1}", point.y, point.x).ok();
}

fn write_ants<T: Write>(width: u32, ants: &[Pos], writer: &mut T) {
  for &pos in ants {
    write_pos(width, pos, writer);
    write!(writer, " ").ok();
  }
}

pub fn write_log<T: Write>(width: u32, log: &[LogMessage], writer: &mut T) {
  for log_message in log {
    match *log_message {
      LogMessage::Turn(turn) => {
        writeln!(writer, "Turn number: {}", turn).ok();
      },
      LogMessage::Attack => {
        writeln!(writer, "  Attack.").ok();
      },
      LogMessage::AttackAnthills => {
        writeln!(writer, "  Attack anthills.").ok();
      },
      LogMessage::GatherFood => {
        writeln!(writer, "  Gather food.").ok();
      },
      LogMessage::Discover => {
        writeln!(writer, "  Discover.").ok();
      },
      LogMessage::Travel => {
        writeln!(writer, "  Travel.").ok();
      },
      LogMessage::MoveRandom => {
        writeln!(writer, "  Move random.").ok();
      },
      LogMessage::Escape => {
        writeln!(writer, "  Escape.").ok();
      },
      LogMessage::ApproachEnemies => {
        writeln!(writer, "  Approach enemies.").ok();
      },
      LogMessage::DefendAnthills => {
        writeln!(writer, "  Defend anthills.").ok();
      },
      LogMessage::Group(group_index) => {
        writeln!(writer, "    Group number: {}", group_index).ok();
      },
      LogMessage::Aggression(aggression) => {
        writeln!(writer, "    Aggression level: {}", aggression).ok();
      },
      LogMessage::Estimate(estimate) => {
        writeln!(writer, "    Estimation: {}", estimate).ok();
      },
      LogMessage::OursAnts(ref ants) => {
        write!(writer, "    Ours ants: ").ok();
        write_ants(width, ants, writer);
        writeln!(writer, "").ok();
      },
      LogMessage::OtherOursAnts(ref ants) => {
        write!(writer, "    Other ours ants: ").ok();
        write_ants(width, ants, writer);
        writeln!(writer, "").ok();
      },
      LogMessage::EnemiesAnts(ref ants) => {
        write!(writer, "    Enemies ants: ").ok();
        write_ants(width, ants, writer);
        writeln!(writer, "").ok();
      },
      LogMessage::GroupSize(ours_moves_count, enemies_count) => {
        writeln!(writer, "    Group size: {0} our moves; {1} enemies.", ours_moves_count, enemies_count).ok();
      },
      LogMessage::Goal(ant_pos, goal_pos) => {
        write!(writer, "    Ours ant ").ok();
        write_pos(width, ant_pos, writer);
        write!(writer, " has goal ").ok();
        write_pos(width, goal_pos, writer);
        writeln!(writer, ".").ok();
      },
      LogMessage::Defender(anthill_pos, enemy_pos, ant_pos) => {
        write!(writer, "    Ours anthill ").ok();
        write_pos(width, anthill_pos, writer);
        write!(writer, " has defender ").ok();
        write_pos(width, ant_pos, writer);
        write!(writer, " from enemy ").ok();
        write_pos(width, enemy_pos, writer);
        writeln!(writer, ".").ok();
      },
      LogMessage::Timeout => {
        writeln!(writer, "  Timeout.").ok();
      },
      LogMessage::MinimaxTimeout => {
        writeln!(writer, "    Minimax timeout.").ok();
      },
      LogMessage::Multitask(ant_pos, next_pos) => {
        write!(writer, "    Multitask from ").ok();
        write_pos(width, ant_pos, writer);
        write!(writer, " to ").ok();
        write_pos(width, next_pos, writer);
        writeln!(writer, ".").ok();
      },
      LogMessage::Jump(ant_pos, next_pos) => {
        write!(writer, "    Jump from ").ok();
        write_pos(width, ant_pos, writer);
        write!(writer, " to ").ok();
        write_pos(width, next_pos, writer);
        writeln!(writer, ".").ok();
      }
    }
  }
}
