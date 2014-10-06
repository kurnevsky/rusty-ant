use std::collections::*;
use coordinates::*;

#[deriving(Clone)]
pub struct Tag {
  start: uint, // Координата ячейки, из которой стартовала волна.
  prev: uint, // Координата ячейки, из которой волна пришла в текущую ячейку.
  length: uint // Расстояние от стартовой ячейки до текущей плюс один.
}

impl Tag {
  pub fn new() -> Tag {
    Tag { start: 0, prev: 0, length: 0 }
  }
}

pub fn wave<'r, T1: Iterator<&'r uint>, T2: MutableSeq<uint>>(width: uint, height: uint, tags: &mut Vec<Tag>, tagged: &mut T2, start: &mut T1, cond: |uint, uint, uint, uint| -> bool, stop_cond: |uint, uint, uint, uint| -> bool) -> Option<uint> {
  let mut q = DList::new();
  for &pos in *start {
    if cond(pos, pos, 0, pos) {
      q.push(pos);
      let tag = tags.get_mut(pos);
      tag.start = pos;
      tag.prev = pos;
      tag.length = 1;
    }
  }
  while !q.is_empty() {
    let pos = q.pop_front().unwrap();
    tagged.push(pos);
    let tag = (*tags)[pos];
    if stop_cond(pos, tag.start, tag.length - 1, tag.prev) {
      return Some(pos);
    }
    let start_pos = tag.start;
    let n_pos = n(width, height, pos);
    if (*tags)[n_pos].length == 0 && cond(n_pos, start_pos, tag.length, pos) {
      let n_tag = tags.get_mut(n_pos);
      n_tag.start = start_pos;
      n_tag.prev = pos;
      n_tag.length = tag.length + 1;
      tagged.push(n_pos);
      q.push(n_pos);
    }
    let w_pos = w(width, pos);
    if (*tags)[w_pos].length == 0 && cond(w_pos, start_pos, tag.length, pos) {
      let w_tag = tags.get_mut(w_pos);
      w_tag.start = start_pos;
      w_tag.prev = pos;
      w_tag.length = tag.length + 1;
      tagged.push(w_pos);
      q.push(w_pos);
    }
    let s_pos = s(width, height, pos);
    if (*tags)[s_pos].length == 0 && cond(s_pos, start_pos, tag.length, pos) {
      let s_tag = tags.get_mut(s_pos);
      s_tag.start = start_pos;
      s_tag.prev = pos;
      s_tag.length = tag.length + 1;
      tagged.push(s_pos);
      q.push(s_pos);
    }
    let e_pos = e(width, pos);
    if (*tags)[e_pos].length == 0 && cond(e_pos, start_pos, tag.length, pos) {
      let e_tag = tags.get_mut(e_pos);
      e_tag.start = start_pos;
      e_tag.prev = pos;
      e_tag.length = tag.length + 1;
      tagged.push(e_pos);
      q.push(e_pos);
    }
  }
  return None;
}

pub fn simple_wave<T: MutableSeq<uint>>(width: uint, height: uint, tags: &mut Vec<Tag>, tagged: &mut T, start: uint, cond: |uint, uint, uint| -> bool, stop_cond: |uint, uint, uint| -> bool) -> Option<uint> {
  wave(width, height, tags, tagged, &mut Some(start).iter(), |pos, _, path_size, prev| { cond(pos, path_size, prev) }, |pos, _, path_size, prev| { stop_cond(pos, path_size, prev) })
}

pub fn clear_tags(tags: &mut Vec<Tag>, tagged: &mut Vec<uint>) {
  for &pos in tagged.iter() {
    let tag = tags.get_mut(pos);
    tag.start = 0;
    tag.prev = 0;
    tag.length = 0;
  }
  tagged.clear();
}

pub fn find_path<T: Deque<uint>>(tags: &Vec<Tag>, from: uint, to: uint, path: &mut T) {
  path.clear();
  if tags[to].start != from {
    return;
  }
  let mut pos = to;
  while pos != from {
    path.push_front(pos);
    pos = tags[pos].prev;
  }
}
