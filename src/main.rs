#![cfg_attr(feature="clippy", feature(plugin))]

#![cfg_attr(feature="clippy", plugin(clippy))]

#![allow(dead_code)]

extern crate rand;

use std::collections::LinkedList;
use std::str::FromStr;
use std::io;
use std::io::{BufRead, BufReader, Write};
use coordinates::*;
use step::*;
use colony::*;
use input::*;
use log::*;

mod coordinates;
mod time;
mod cell;
mod step;
mod input;
mod wave;
mod log;
mod colony;

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

fn read_turn<T: BufRead>(reader: &mut T) -> Option<usize> {
  let input = read_nonempty_line(reader);
  let split: Vec<&str> = input.as_str().trim().split(' ').collect();
  if split.len() != 2 || split[0] != "turn" {
    None
  } else {
    usize::from_str(split[1]).ok()
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
    if split.len() == 0 {
      return None;
    }
    match split[0] {
      "ready" => {
        if split.len() != 1 {
          return None;
        }
        if load_time_option.is_none() ||
           turn_time_option.is_none() ||
           width_option.is_none() ||
           height_option.is_none() ||
           turns_option.is_none() ||
           view_radius2_option.is_none() ||
           attack_radius2_option.is_none() ||
           spawn_radius2_option.is_none() ||
           seed_option.is_none() {
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
          seed_option.unwrap() as u64
        )));
      },
      "loadtime" => {
        if split.len() != 2 {
          return None;
        }
        load_time_option = usize::from_str(split[1]).ok();
        if load_time_option.is_none() {
          return None;
        }
      },
      "turntime" => {
        if split.len() != 2 {
          return None;
        }
        turn_time_option = u32::from_str(split[1]).ok();
        if turn_time_option.is_none() {
          return None;
        }
      },
      "rows" => {
        if split.len() != 2 {
          return None;
        }
        height_option = usize::from_str(split[1]).ok();
        if height_option.is_none() {
          return None;
        }
      },
      "cols" => {
        if split.len() != 2 {
          return None;
        }
        width_option = usize::from_str(split[1]).ok();
        if width_option.is_none() {
          return None;
        }
      },
      "turns" => {
        if split.len() != 2 {
          return None;
        }
        turns_option = usize::from_str(split[1]).ok();
        if turns_option.is_none() {
          return None;
        }
      },
      "viewradius2" => {
        if split.len() != 2 {
          return None;
        }
        view_radius2_option = usize::from_str(split[1]).ok();
        if view_radius2_option.is_none() {
          return None;
        }
      },
      "attackradius2" => {
        if split.len() != 2 {
          return None;
        }
        attack_radius2_option = usize::from_str(split[1]).ok();
        if attack_radius2_option.is_none() {
          return None;
        }
      },
      "spawnradius2" => {
        if split.len() != 2 {
          return None;
        }
        spawn_radius2_option = usize::from_str(split[1]).ok();
        if spawn_radius2_option.is_none() {
          return None;
        }
      },
      "player_seed" => {
        if split.len() != 2 {
          return None;
        }
        seed_option = i64::from_str(split[1]).ok();
        if seed_option.is_none() {
          return None;
        }
      },
      _ => { }
    }
  }
}

fn turn_info<T: BufRead>(reader: &mut T) -> Option<Box<LinkedList<Input>>> {
  let mut input = Box::new(LinkedList::new());
  loop {
    let string = read_nonempty_line(reader);
    let split: Vec<&str> = string.as_str().trim().split(' ').collect();
    if split.len() == 0 {
      return None;
    }
    match split[0] {
      "go" => {
        if split.len() != 1 {
          return None;
        }
        return Some(input);
      },
      "w" => {
        if split.len() != 3 {
          return None;
        }
        if let (Some(row), Some(col)) = (usize::from_str(split[1]).ok(), usize::from_str(split[2]).ok()) {
          input.push_back(Input::Water(Point { x: col, y: row }));
        } else {
          return None;
        }
      },
      "f" => {
        if split.len() != 3 {
          return None;
        }
        if let (Some(row), Some(col)) = (usize::from_str(split[1]).ok(), usize::from_str(split[2]).ok()) {
          input.push_back(Input::Food(Point { x: col, y: row }));
        } else {
          return None;
        }
      },
      "h" => {
        if split.len() != 4 {
          return None;
        }
        if let (Some(row), Some(col), Some(player)) = (usize::from_str(split[1]).ok(), usize::from_str(split[2]).ok(), usize::from_str(split[3]).ok()) {
          input.push_back(Input::Anthill(Point { x: col, y: row }, player));
        } else {
          return None;
        }
      },
      "a" => {
        if split.len() != 4 {
          return None;
        }
        if let (Some(row), Some(col), Some(player)) = (usize::from_str(split[1]).ok(), usize::from_str(split[2]).ok(), usize::from_str(split[3]).ok()) {
          input.push_back(Input::Ant(Point { x: col, y: row }, player));
        } else {
          return None;
        }
      },
      "d" => {
        if split.len() != 4 {
          return None;
        }
        if let (Some(row), Some(col), Some(player)) = (usize::from_str(split[1]).ok(), usize::from_str(split[2]).ok(), usize::from_str(split[3]).ok()) {
          input.push_back(Input::Dead(Point { x: col, y: row }, player));
        } else {
          return None;
        }
      },
      _ => {
        return None;
      }
    }
  }
}

fn print_output<T: Write>(writer: &mut T, output: &mut LinkedList<Step>) {
  for i in output.iter() {
    writeln!(writer, "o {0} {1} {2}", i.point.y, i.point.x, match i.direction {
      Direction::North => 'N',
      Direction::South => 'S',
      Direction::West => 'W',
      Direction::East => 'E'
    }).ok();
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
  let mut output: LinkedList<Step> = LinkedList::new();
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
            turn(&mut *colony, &mut input.iter(), &mut output);
            print_output(&mut stdout, &mut output)
          },
          None => {
            writeln!(stderr, "Icorrect input 3!").ok();
            return;
          }
        }
      }
      final_colony(&*colony, &mut stdin, &mut stderr);
    },
    None => {
      writeln!(stderr, "Icorrect input 4!").ok();
      return;
    }
  }
}
