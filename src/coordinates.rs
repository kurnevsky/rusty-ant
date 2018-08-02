use std::cmp;

pub type Pos = usize;

#[derive(Clone, Copy, Debug)]
pub struct Point {
  pub y: u32,
  pub x: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
  North,
  South,
  West,
  East,
}

pub fn length(width: u32, height: u32) -> Pos {
  (width * height) as Pos
}

pub fn to_pos(width: u32, point: Point) -> Pos {
  (point.y * width + point.x) as Pos
}

pub fn from_pos(width: u32, pos: Pos) -> Point {
  Point {
    x: pos as u32 % width,
    y: pos as u32 / width,
  }
}

pub fn to_direction(width: u32, height: u32, pos1: Pos, pos2: Pos) -> Option<Direction> {
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

pub fn point_n(height: u32, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y + height - 1) % height,
  }
}

pub fn point_s(height: u32, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y + 1) % height,
  }
}

pub fn point_w(width: u32, point: Point) -> Point {
  Point {
    x: (point.x + width - 1) % width,
    y: point.y,
  }
}

pub fn point_e(width: u32, point: Point) -> Point {
  Point {
    x: (point.x + 1) % width,
    y: point.y,
  }
}

pub fn n(width: u32, height: u32, pos: Pos) -> Pos {
  let len = length(width, height);
  (pos + len - width as Pos) % len
}

pub fn s(width: u32, height: u32, pos: Pos) -> Pos {
  (pos + width as Pos) % length(width, height)
}

pub fn w(width: u32, pos: Pos) -> Pos {
  if pos as u32 % width == 0 {
    pos + width as Pos - 1
  } else {
    pos - 1
  }
}

pub fn e(width: u32, pos: Pos) -> Pos {
  if pos as u32 % width == width - 1 {
    pos + 1 - width as Pos
  } else {
    pos + 1
  }
}

pub fn nw(width: u32, height: u32, pos: Pos) -> Pos {
  n(width, height, w(width, pos))
}

pub fn ne(width: u32, height: u32, pos: Pos) -> Pos {
  n(width, height, e(width, pos))
}

pub fn sw(width: u32, height: u32, pos: Pos) -> Pos {
  s(width, height, w(width, pos))
}

pub fn se(width: u32, height: u32, pos: Pos) -> Pos {
  s(width, height, e(width, pos))
}

pub fn point_manhattan(width: u32, height: u32, point1: Point, point2: Point) -> u32 {
  let diff_x = (point1.x as i32 - point2.x as i32).abs() as u32;
  let diff_y = (point1.y as i32 - point2.y as i32).abs() as u32;
  cmp::min(diff_x, width - diff_x) + cmp::min(diff_y, height - diff_y)
}

pub fn point_euclidean(width: u32, height: u32, point1: Point, point2: Point) -> u32 {
  let diff_x = (point1.x as i32 - point2.x as i32).abs() as u32;
  let diff_y = (point1.y as i32 - point2.y as i32).abs() as u32;
  cmp::min(diff_x, width - diff_x).pow(2) + cmp::min(diff_y, height - diff_y).pow(2)
}

pub fn manhattan(width: u32, height: u32, pos1: Pos, pos2: Pos) -> u32 {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_manhattan(width, height, point1, point2)
}

pub fn euclidean(width: u32, height: u32, pos1: Pos, pos2: Pos) -> u32 {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_euclidean(width, height, point1, point2)
}
