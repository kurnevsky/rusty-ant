use crate::coordinates::*;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Tag {
  start: Pos,  // Координата ячейки, из которой стартовала волна.
  prev: Pos,   // Координата ячейки, из которой волна пришла в текущую ячейку.
  length: u32, // Расстояние от стартовой ячейки до текущей плюс один.
}

impl Tag {
  pub fn new() -> Tag {
    Tag {
      start: 0,
      prev: 0,
      length: 0,
    }
  }

  pub fn length(&self) -> u32 {
    self.length
  }
}

pub fn wave<'r, T, F1, F2>(
  width: u32,
  height: u32,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  start: &mut T,
  mut cond: F1,
  mut stop_cond: F2,
) -> Option<Pos>
where
  T: Iterator<Item = &'r Pos>,
  F1: FnMut(Pos, Pos, u32, Pos) -> bool,
  F2: FnMut(Pos, Pos, u32, Pos) -> bool,
{
  let mut q = VecDeque::new();
  for &pos in start {
    if cond(pos, pos, 0, pos) {
      q.push_back(pos);
      let tag = &mut tags[pos];
      tag.start = pos;
      tag.prev = pos;
      tag.length = 1;
    }
  }
  while let Some(pos) = q.pop_front() {
    tagged.push(pos);
    let length = tags[pos].length;
    if stop_cond(pos, tags[pos].start, length - 1, tags[pos].prev) {
      return Some(pos);
    }
    let start_pos = tags[pos].start;
    let n_pos = n(width, height, pos);
    if tags[n_pos].length == 0 && cond(n_pos, start_pos, length, pos) {
      let n_tag = &mut tags[n_pos];
      n_tag.start = start_pos;
      n_tag.prev = pos;
      n_tag.length = length + 1;
      tagged.push(n_pos);
      q.push_back(n_pos);
    }
    let w_pos = w(width, pos);
    if tags[w_pos].length == 0 && cond(w_pos, start_pos, length, pos) {
      let w_tag = &mut tags[w_pos];
      w_tag.start = start_pos;
      w_tag.prev = pos;
      w_tag.length = length + 1;
      tagged.push(w_pos);
      q.push_back(w_pos);
    }
    let s_pos = s(width, height, pos);
    if tags[s_pos].length == 0 && cond(s_pos, start_pos, length, pos) {
      let s_tag = &mut tags[s_pos];
      s_tag.start = start_pos;
      s_tag.prev = pos;
      s_tag.length = length + 1;
      tagged.push(s_pos);
      q.push_back(s_pos);
    }
    let e_pos = e(width, pos);
    if tags[e_pos].length == 0 && cond(e_pos, start_pos, length, pos) {
      let e_tag = &mut tags[e_pos];
      e_tag.start = start_pos;
      e_tag.prev = pos;
      e_tag.length = length + 1;
      tagged.push(e_pos);
      q.push_back(e_pos);
    }
  }
  None
}

pub fn simple_wave<F1, F2>(
  width: u32,
  height: u32,
  tags: &mut Vec<Tag>,
  tagged: &mut Vec<Pos>,
  start: Pos,
  mut cond: F1,
  mut stop_cond: F2,
) -> Option<Pos>
where
  F1: FnMut(Pos, u32, Pos) -> bool,
  F2: FnMut(Pos, u32, Pos) -> bool,
{
  wave(
    width,
    height,
    tags,
    tagged,
    &mut Some(start).iter(),
    |pos, _, path_size, prev| cond(pos, path_size, prev),
    |pos, _, path_size, prev| stop_cond(pos, path_size, prev),
  )
}

pub fn clear_tags(tags: &mut Vec<Tag>, tagged: &mut Vec<Pos>) {
  for &pos in tagged.iter() {
    let tag = &mut tags[pos];
    tag.start = 0;
    tag.prev = 0;
    tag.length = 0;
  }
  tagged.clear();
}

/// Find the inverse path to the goal. Path includes the goal and doesn't
/// include start position.
pub fn find_path(tags: &[Tag], from: Pos, to: Pos, path: &mut Vec<Pos>) {
  path.clear();
  if tags[to].start != from {
    return;
  }
  path.reserve(tags[to].length as usize - 1);
  let mut pos = to;
  while pos != from {
    path.push(pos);
    pos = tags[pos].prev;
  }
}
