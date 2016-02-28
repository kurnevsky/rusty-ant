use std::collections::VecDeque;
use coordinates::*;

#[derive(Clone)]
pub struct Tag {
  start: usize, // Координата ячейки, из которой стартовала волна.
  prev: usize, // Координата ячейки, из которой волна пришла в текущую ячейку.
  length: usize // Расстояние от стартовой ячейки до текущей плюс один.
}

impl Tag {
  pub fn new() -> Tag {
    Tag { start: 0, prev: 0, length: 0 }
  }
}

pub fn wave<'r, T: Iterator<Item=&'r usize>, F1: FnMut(usize, usize, usize, usize) -> bool, F2: FnMut(usize, usize, usize, usize) -> bool>(width: usize, height: usize, tags: &mut Vec<Tag>, tagged: &mut Vec<usize>, start: &mut T, mut cond: F1, mut stop_cond: F2) -> Option<usize> {
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
  while !q.is_empty() {
    let pos = q.pop_front().unwrap();
    tagged.push(pos);
    let tag = tags[pos].clone();
    if stop_cond(pos, tag.start, tag.length - 1, tag.prev) {
      return Some(pos);
    }
    let start_pos = tag.start;
    let n_pos = n(width, height, pos);
    if tags[n_pos].length == 0 && cond(n_pos, start_pos, tag.length, pos) {
      let n_tag = &mut tags[n_pos];
      n_tag.start = start_pos;
      n_tag.prev = pos;
      n_tag.length = tag.length + 1;
      tagged.push(n_pos);
      q.push_back(n_pos);
    }
    let w_pos = w(width, pos);
    if tags[w_pos].length == 0 && cond(w_pos, start_pos, tag.length, pos) {
      let w_tag = &mut tags[w_pos];
      w_tag.start = start_pos;
      w_tag.prev = pos;
      w_tag.length = tag.length + 1;
      tagged.push(w_pos);
      q.push_back(w_pos);
    }
    let s_pos = s(width, height, pos);
    if tags[s_pos].length == 0 && cond(s_pos, start_pos, tag.length, pos) {
      let s_tag = &mut tags[s_pos];
      s_tag.start = start_pos;
      s_tag.prev = pos;
      s_tag.length = tag.length + 1;
      tagged.push(s_pos);
      q.push_back(s_pos);
    }
    let e_pos = e(width, pos);
    if tags[e_pos].length == 0 && cond(e_pos, start_pos, tag.length, pos) {
      let e_tag = &mut tags[e_pos];
      e_tag.start = start_pos;
      e_tag.prev = pos;
      e_tag.length = tag.length + 1;
      tagged.push(e_pos);
      q.push_back(e_pos);
    }
  }
  return None;
}

pub fn simple_wave<F1: FnMut(usize, usize, usize) -> bool, F2: FnMut(usize, usize, usize) -> bool>(width: usize, height: usize, tags: &mut Vec<Tag>, tagged: &mut Vec<usize>, start: usize, mut cond: F1, mut stop_cond: F2) -> Option<usize> {
  wave(width, height, tags, tagged, &mut Some(start).iter(), |pos, _, path_size, prev| { cond(pos, path_size, prev) }, |pos, _, path_size, prev| { stop_cond(pos, path_size, prev) })
}

pub fn clear_tags(tags: &mut Vec<Tag>, tagged: &mut Vec<usize>) {
  for &pos in tagged.iter() {
    let tag = &mut tags[pos];
    tag.start = 0;
    tag.prev = 0;
    tag.length = 0;
  }
  tagged.clear();
}

pub fn find_path(tags: &Vec<Tag>, from: usize, to: usize, path: &mut Vec<usize>) { //TODO: merge with find_inverse_path; don't use Vec::reverse.
  path.clear();
  if tags[to].start != from {
    return;
  }
  let mut pos = to;
  while pos != from {
    path.push(pos);
    pos = tags[pos].prev;
  }
  path.reverse();
}

pub fn find_inverse_path(tags: &Vec<Tag>, from: usize, to: usize, path: &mut Vec<usize>) {
  path.clear();
  if tags[to].start != from {
    return;
  }
  let mut pos = to;
  while pos != from {
    pos = tags[pos].prev;
    path.push(pos);
  }
}
