use std::cmp;
use std::num::*;

#[derive(Clone)]
pub struct Point {
  pub y: uint,
  pub x: uint
}

#[derive(Clone)]
pub enum Direction {
  North,
  South,
  West,
  East
}

pub fn length(width: uint, height: uint) -> uint {
  width * height
}

pub fn to_pos(width: uint, point: Point) -> uint {
  point.y * width + point.x
}

pub fn from_pos(width: uint, pos: uint) -> Point {
  Point {
    x: pos % width,
    y: pos / width
  }
}

pub fn to_direction(width: uint, height: uint, pos1: uint, pos2: uint) -> Option<Direction> {
  if n(width, height, pos1) == pos2 {
    Some(Direction::North)
  } else if s(width, height, pos1) == pos2 {
    Some(Direction::South)
  } else if w(width, pos1) == pos2 {
    Some(Direction::West)
  } else if e(width, pos1) == pos2 {
    Some(Direction::East)
  } else {
    None
  }
}

/*
pub fn point_n(height: uint, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y - 1 + height) % height
  }
}

pub fn point_s(height: uint, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y + 1) % height
  }
}

pub fn point_w(width: uint, point: Point) -> Point {
  Point {
    x: (point.x - 1 + width) % width,
    y: point.y
  }
}

pub fn point_e(width: uint, point: Point) -> Point {
  Point {
    x: (point.x + 1) % width,
    y: point.y
  }
}
*/

pub fn n(width: uint, height: uint, pos: uint) -> uint {
  let len = length(width, height);
  (pos - width + len) % len
}

pub fn s(width: uint, height: uint, pos: uint) -> uint {
  (pos + width) % length(width, height)
}

pub fn w(width: uint, pos: uint) -> uint {
  if pos % width == 0 {
    pos + width - 1
  } else {
    pos - 1
  }
}

pub fn e(width: uint, pos: uint) -> uint {
  if pos % width == width - 1 {
    pos - width + 1
  } else {
    pos + 1
  }
}

pub fn nw(width: uint, height: uint, pos: uint) -> uint {
  n(width, height, w(width, pos))
}

pub fn ne(width: uint, height: uint, pos: uint) -> uint {
  n(width, height, e(width, pos))
}

pub fn sw(width: uint, height: uint, pos: uint) -> uint {
  s(width, height, w(width, pos))
}

pub fn se(width: uint, height: uint, pos: uint) -> uint {
  s(width, height, e(width, pos))
}

pub fn point_manhattan(width: uint, height: uint, point1: Point, point2: Point) -> uint {
  let diff_x = (point1.x as int - point2.x as int).abs() as uint;
  let diff_y = (point1.y as int - point2.y as int).abs() as uint;
  cmp::min(diff_x, width - diff_x) + cmp::min(diff_y, height - diff_y)
}

pub fn point_euclidean(width: uint, height: uint, point1: Point, point2: Point) -> uint {
  let diff_x = (point1.x as int - point2.x as int).abs() as uint;
  let diff_y = (point1.y as int - point2.y as int).abs() as uint;
  cmp::min(diff_x, width - diff_x).pow(2) + cmp::min(diff_y, height - diff_y).pow(2)
}

pub fn manhattan(width: uint, height: uint, pos1: uint, pos2: uint) -> uint {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_manhattan(width, height, point1, point2)
}

pub fn euclidean(width: uint, height: uint, pos1: uint, pos2: uint) -> uint {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_euclidean(width, height, point1, point2)
}
