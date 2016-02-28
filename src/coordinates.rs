use std::cmp;

type Pos = usize;

#[derive(Clone, Copy)]
pub struct Point {
  pub y: usize,
  pub x: usize
}

#[derive(Clone, Copy)]
pub enum Direction {
  North,
  South,
  West,
  East
}

pub fn length(width: usize, height: usize) -> usize {
  width * height
}

pub fn to_pos(width: usize, point: Point) -> usize {
  point.y * width + point.x
}

pub fn from_pos(width: usize, pos: usize) -> Point {
  Point {
    x: pos % width,
    y: pos / width
  }
}

pub fn to_direction(width: usize, height: usize, pos1: usize, pos2: usize) -> Option<Direction> {
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

pub fn point_n(height: usize, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y - 1 + height) % height
  }
}

pub fn point_s(height: usize, point: Point) -> Point {
  Point {
    x: point.x,
    y: (point.y + 1) % height
  }
}

pub fn point_w(width: usize, point: Point) -> Point {
  Point {
    x: (point.x - 1 + width) % width,
    y: point.y
  }
}

pub fn point_e(width: usize, point: Point) -> Point {
  Point {
    x: (point.x + 1) % width,
    y: point.y
  }
}

pub fn n(width: usize, height: usize, pos: usize) -> usize {
  let len = length(width, height);
  (pos - width + len) % len
}

pub fn s(width: usize, height: usize, pos: usize) -> usize {
  (pos + width) % length(width, height)
}

pub fn w(width: usize, pos: usize) -> usize {
  if pos % width == 0 {
    pos + width - 1
  } else {
    pos - 1
  }
}

pub fn e(width: usize, pos: usize) -> usize {
  if pos % width == width - 1 {
    pos - width + 1
  } else {
    pos + 1
  }
}

pub fn nw(width: usize, height: usize, pos: usize) -> usize {
  n(width, height, w(width, pos))
}

pub fn ne(width: usize, height: usize, pos: usize) -> usize {
  n(width, height, e(width, pos))
}

pub fn sw(width: usize, height: usize, pos: usize) -> usize {
  s(width, height, w(width, pos))
}

pub fn se(width: usize, height: usize, pos: usize) -> usize {
  s(width, height, e(width, pos))
}

pub fn point_manhattan(width: usize, height: usize, point1: Point, point2: Point) -> usize {
  let diff_x = (point1.x as i32 - point2.x as i32).abs() as usize;
  let diff_y = (point1.y as i32 - point2.y as i32).abs() as usize;
  cmp::min(diff_x, width - diff_x) + cmp::min(diff_y, height - diff_y)
}

pub fn point_euclidean(width: usize, height: usize, point1: Point, point2: Point) -> usize {
  let diff_x = (point1.x as i32 - point2.x as i32).abs() as usize;
  let diff_y = (point1.y as i32 - point2.y as i32).abs() as usize;
  cmp::min(diff_x, width - diff_x).pow(2) + cmp::min(diff_y, height - diff_y).pow(2)
}

pub fn manhattan(width: usize, height: usize, pos1: usize, pos2: usize) -> usize {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_manhattan(width, height, point1, point2)
}

pub fn euclidean(width: usize, height: usize, pos1: usize, pos2: usize) -> usize {
  let point1 = from_pos(width, pos1);
  let point2 = from_pos(width, pos2);
  point_euclidean(width, height, point1, point2)
}
