#![feature(globs)]

use std::collections::*;
use std::io;
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

fn read_nonempty_line<T: Buffer>(reader: &mut T) -> String {
   loop {
    let input = reader.read_line().ok().expect("Failed to read line!");
    if !input.is_empty() {
      return input;
    }
  }
}

fn read_turn<T: Buffer>(reader: &mut T) -> Option<uint> {
  let input = read_nonempty_line(reader);
  let split: Vec<&str> = input.as_slice().trim().split(' ').collect();
  if split.len() != 2 || split[0] != "turn" {
    return None;
  } else {
    return from_str(split[1]);
  }
}

fn init<T: Buffer>(reader: &mut T) -> Option<Box<Colony>> {
  let mut load_time = None;
  let mut turn_time = None;
  let mut width = None;
  let mut height = None;
  let mut turns = None;
  let mut view_radius2 = None;
  let mut attack_radius2 = None;
  let mut spawn_radius2 = None;
  let mut seed: Option<i64> = None;
  loop {
    let input = read_nonempty_line(reader);
    let split: Vec<&str> = input.as_slice().trim().split(' ').collect();
    if split.len() == 0 {
      return None;
    }
    match split[0] {
      "ready" => {
        if split.len() != 1 {
          return None;
        }
        if load_time.is_none() ||
           turn_time.is_none() ||
           width.is_none() ||
           height.is_none() ||
           turns.is_none() ||
           view_radius2.is_none() ||
           attack_radius2.is_none() ||
           spawn_radius2.is_none() ||
           seed.is_none() {
          return None;
        }
        return Some(box Colony::new(
          width.unwrap(),
          height.unwrap(),
          turn_time.unwrap(),
          turns.unwrap(),
          view_radius2.unwrap(),
          attack_radius2.unwrap(),
          spawn_radius2.unwrap(),
          seed.unwrap() as u64
        ));
      },
      "loadtime" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => load_time = Some(x),
          None => return None
        }
      },
      "turntime" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => turn_time = Some(x),
          None => return None
        }
      },
      "rows" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => height = Some(x),
          None => return None
        }
      },
      "cols" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => width = Some(x),
          None => return None
        }
      },
      "turns" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => turns = Some(x),
          None => return None
        }
      },
      "viewradius2" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => view_radius2 = Some(x),
          None => return None
        }
      },
      "attackradius2" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => attack_radius2 = Some(x),
          None => return None
        }
      },
      "spawnradius2" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<uint>(split[1]) {
          Some(x) => spawn_radius2 = Some(x),
          None => return None
        }
      },
      "player_seed" => {
        if split.len() != 2 {
          return None;
        }
        match from_str::<i64>(split[1]) {
          Some(x) => seed = Some(x),
          None => return None
        }
      },
      _ => { }
    }
  }
}

fn turn_info<T: Buffer>(reader: &mut T) -> Option<Box<DList<Input>>> {
  let mut input = box DList::new();
  loop {
    let string = read_nonempty_line(reader);
    let split: Vec<&str> = string.as_slice().trim().split(' ').collect();
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
        let row = from_str::<uint>(split[1]);
        let col = from_str::<uint>(split[2]);
        if row.is_none() || col.is_none() {
          return None;
        }
        input.push(InputWater(Point { x: col.unwrap(), y: row.unwrap() }));
      },
      "f" => {
        if split.len() != 3 {
          return None;
        }
        let row = from_str::<uint>(split[1]);
        let col = from_str::<uint>(split[2]);
        if row.is_none() || col.is_none() {
          return None;
        }
        input.push(InputFood(Point { x: col.unwrap(), y: row.unwrap() }));
      },
      "h" => {
        if split.len() != 4 {
          return None;
        }
        let row = from_str::<uint>(split[1]);
        let col = from_str::<uint>(split[2]);
        let player = from_str::<uint>(split[3]);
        if row.is_none() || col.is_none() || player.is_none() {
          return None;
        }
        input.push(InputAnthill(Point { x: col.unwrap(), y: row.unwrap() }, player.unwrap()));
      },
      "a" => {
        if split.len() != 4 {
          return None;
        }
        let row = from_str::<uint>(split[1]);
        let col = from_str::<uint>(split[2]);
        let player = from_str::<uint>(split[3]);
        if row.is_none() || col.is_none() || player.is_none() {
          return None;
        }
        input.push(InputAnt(Point { x: col.unwrap(), y: row.unwrap() }, player.unwrap()));
      },
      "d" => {
        if split.len() != 4 {
          return None;
        }
        let row = from_str::<uint>(split[1]);
        let col = from_str::<uint>(split[2]);
        let player = from_str::<uint>(split[3]);
        if row.is_none() || col.is_none() || player.is_none() {
          return None;
        }
        input.push(InputDead(Point { x: col.unwrap(), y: row.unwrap() }, player.unwrap()));
      },
      _ => return None
    }
  }
}

fn print_output<T: Writer>(writer: &mut T, output: &mut DList<Step>) {
  for i in output.iter() {
    writer.write_str("o ").ok();
    writer.write_uint(i.point.y).ok();
    writer.write_char(' ').ok();
    writer.write_uint(i.point.x).ok();
    writer.write_char(' ').ok();
    match i.direction {
      North => writer.write_char('N').ok(),
      South => writer.write_char('S').ok(),
      West => writer.write_char('W').ok(),
      East => writer.write_char('E').ok()
    };
    writer.write_char('\n').ok();
  }
  writer.write_line("go").ok();
}

fn final<T1: Buffer, T2: Writer>(colony: &Colony, reader: &mut T1, writer: &mut T2) {
  read_nonempty_line(reader);
  read_nonempty_line(reader);
  turn_info(reader);
  write_log(colony.width, &colony.log, writer);
}

fn main() {
  let mut stdin = io::stdin();
  let mut stderr = io::stderr();
  let mut stdout = io::stdout();
  let mut output: DList<Step> = DList::new();
  if read_turn(&mut stdin) != Some(0) {
    stderr.write_line("Icorrect input 1!").ok();
    return;
  }
  match init(&mut stdin) {
    Some(mut colony) => {
      stdout.write_line("go").ok();
      loop {
        let turn_number = read_turn(&mut stdin);
        if turn_number != Some(colony.cur_turn + 1) {
          break;
        }
        match turn_info(&mut stdin) {
          Some(input) => {
            turn(&mut *colony, &mut input.iter(), &mut output);
            print_output(&mut stdout, &mut output)
          },
          None => {
            stderr.write_line("Icorrect input 3!").ok();
            return;
          }
        }
      }
      final(&*colony, &mut stdin, &mut stderr);
    },
    None => {
      stderr.write_line("Icorrect input 4!").ok();
      return;
    }
  }
}
