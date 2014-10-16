use std::collections::DList;

#[deriving(Clone, PartialEq)]
pub enum LogMessage {
  Turn(uint),
  Attack,
  Group(uint),
  Aggression(uint),
  Estimate(int),
  OursAnts(Box<DList<uint>>),
  EnemiesAnts(Box<DList<uint>>),
  GroupSize(uint, uint)
}
