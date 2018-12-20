#![allow(dead_code)]
#![cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
#![cfg_attr(feature = "cargo-clippy", allow(cyclomatic_complexity))]

extern crate rand;

use crate::colony::*;
use crate::coordinates::*;
use crate::input::*;
use crate::log::*;
use std::{
  io::{self, BufRead, BufReader, Write},
  str::FromStr,
};
use crate::step::*;

mod cell;
mod colony;
mod coordinates;
mod input;
mod log;
mod step;
mod time;
mod wave;

fn read_nonempty_line<T: BufRead>(reader: &mut T) -> String {
  let mut input = String::new();
  loop {
    reader.read_line(&mut input).ok();
    if !input.is_empty() {
      break;
    }
  }
  input
}

fn read_turn<T: BufRead>(reader: &mut T) -> Option<u32> {
  let input = read_nonempty_line(reader);
  let split: Vec<&str> = input.as_str().trim().split(' ').collect();
  if split.len() != 2 || split[0] != "turn" {
    None
  } else {
    u32::from_str(split[1]).ok()
  }
}

fn init_colony<T: BufRead>(reader: &mut T) -> Option<Box<Colony>> {
  let mut load_time_option = None;
  let mut turn_time_option = None;
  let mut width_option = None;
  let mut height_option = None;
  let mut turns_option = None;
  let mut view_radius2_option = None;
  let mut attack_radius2_option = None;
  let mut spawn_radius2_option = None;
  let mut seed_option: Option<i64> = None;
  loop {
    let input = read_nonempty_line(reader);
    let split: Vec<&str> = input.as_str().trim().split(' ').collect();
    if split.is_empty() {
      return None;
    }
    match split[0] {
      "ready" => {
        if split.len() != 1 {
          return None;
        }
        if load_time_option.is_none()
          || turn_time_option.is_none()
          || width_option.is_none()
          || height_option.is_none()
          || turns_option.is_none()
          || view_radius2_option.is_none()
          || attack_radius2_option.is_none()
          || spawn_radius2_option.is_none()
          || seed_option.is_none()
        {
          return None;
        }
        return Some(Box::new(Colony::new(
          width_option.unwrap(),
          height_option.unwrap(),
          turn_time_option.unwrap(),
          turns_option.unwrap(),
          view_radius2_option.unwrap(),
          attack_radius2_option.unwrap(),
          spawn_radius2_option.unwrap(),
          seed_option.unwrap() as u64,
        )));
      }
      "loadtime" => {
        if split.len() != 2 {
          return None;
        }
        load_time_option = u32::from_str(split[1]).ok();
        load_time_option?;
      }
      "turntime" => {
        if split.len() != 2 {
          return None;
        }
        turn_time_option = u32::from_str(split[1]).ok();
        turn_time_option?;
      }
      "rows" => {
        if split.len() != 2 {
          return None;
        }
        height_option = u32::from_str(split[1]).ok();
        height_option?;
      }
      "cols" => {
        if split.len() != 2 {
          return None;
        }
        width_option = u32::from_str(split[1]).ok();
        width_option?;
      }
      "turns" => {
        if split.len() != 2 {
          return None;
        }
        turns_option = u32::from_str(split[1]).ok();
        turns_option?;
      }
      "viewradius2" => {
        if split.len() != 2 {
          return None;
        }
        view_radius2_option = u32::from_str(split[1]).ok();
        view_radius2_option?;
      }
      "attackradius2" => {
        if split.len() != 2 {
          return None;
        }
        attack_radius2_option = u32::from_str(split[1]).ok();
        attack_radius2_option?;
      }
      "spawnradius2" => {
        if split.len() != 2 {
          return None;
        }
        spawn_radius2_option = u32::from_str(split[1]).ok();
        spawn_radius2_option?;
      }
      "player_seed" => {
        if split.len() != 2 {
          return None;
        }
        seed_option = i64::from_str(split[1]).ok();
        seed_option?;
      }
      _ => {}
    }
  }
}

fn turn_info<T: BufRead>(reader: &mut T) -> Option<Vec<Input>> {
  let mut input = Vec::new();
  loop {
    let string = read_nonempty_line(reader);
    let split: Vec<&str> = string.as_str().trim().split(' ').collect();
    if split.is_empty() {
      return None;
    }
    match split[0] {
      "go" => {
        if split.len() != 1 {
          return None;
        }
        return Some(input);
      }
      "w" => {
        if split.len() != 3 {
          return None;
        }
        if let (Some(row), Some(col)) = (u32::from_str(split[1]).ok(), u32::from_str(split[2]).ok()) {
          input.push(Input::Water(Point { x: col, y: row }));
        } else {
          return None;
        }
      }
      "f" => {
        if split.len() != 3 {
          return None;
        }
        if let (Some(row), Some(col)) = (u32::from_str(split[1]).ok(), u32::from_str(split[2]).ok()) {
          input.push(Input::Food(Point { x: col, y: row }));
        } else {
          return None;
        }
      }
      "h" => {
        if split.len() != 4 {
          return None;
        }
        if let (Some(row), Some(col), Some(player)) = (
          u32::from_str(split[1]).ok(),
          u32::from_str(split[2]).ok(),
          u32::from_str(split[3]).ok(),
        ) {
          input.push(Input::Anthill(Point { x: col, y: row }, player));
        } else {
          return None;
        }
      }
      "a" => {
        if split.len() != 4 {
          return None;
        }
        if let (Some(row), Some(col), Some(player)) = (
          u32::from_str(split[1]).ok(),
          u32::from_str(split[2]).ok(),
          u32::from_str(split[3]).ok(),
        ) {
          input.push(Input::Ant(Point { x: col, y: row }, player));
        } else {
          return None;
        }
      }
      "d" => {
        if split.len() != 4 {
          return None;
        }
        if let (Some(row), Some(col), Some(player)) = (
          u32::from_str(split[1]).ok(),
          u32::from_str(split[2]).ok(),
          u32::from_str(split[3]).ok(),
        ) {
          input.push(Input::Dead(Point { x: col, y: row }, player));
        } else {
          return None;
        }
      }
      _ => {
        return None;
      }
    }
  }
}

fn print_output<T: Write>(writer: &mut T, output: &mut Vec<Step>) {
  for i in output.iter() {
    writeln!(
      writer,
      "o {0} {1} {2}",
      i.point.y,
      i.point.x,
      match i.direction {
        Direction::North => 'N',
        Direction::South => 'S',
        Direction::West => 'W',
        Direction::East => 'E',
      }
    ).ok();
  }
  writeln!(writer, "go").ok();
}

fn final_colony<T1: BufRead, T2: Write>(colony: &Colony, reader: &mut T1, writer: &mut T2) {
  read_nonempty_line(reader);
  read_nonempty_line(reader);
  turn_info(reader);
  write_log(colony.width(), colony.log(), writer);
}

fn main() {
  let mut stdin = BufReader::new(io::stdin());
  let mut stderr = io::stderr();
  let mut stdout = io::stdout();
  let mut output = Vec::new();
  if read_turn(&mut stdin) != Some(0) {
    writeln!(stderr, "Icorrect input 1!").ok();
    return;
  }
  match init_colony(&mut stdin) {
    Some(mut colony) => {
      writeln!(stdout, "go").ok();
      loop {
        let turn_number = read_turn(&mut stdin);
        if turn_number != Some(colony.cur_turn() + 1) {
          break;
        }
        match turn_info(&mut stdin) {
          Some(input) => {
            turn(&mut *colony, &input, &mut output);
            print_output(&mut stdout, &mut output)
          }
          None => {
            writeln!(stderr, "Icorrect input 3!").ok();
            return;
          }
        }
      }
      final_colony(&*colony, &mut stdin, &mut stderr);
    }
    None => {
      writeln!(stderr, "Icorrect input 4!").ok();
      return;
    }
  }
}
