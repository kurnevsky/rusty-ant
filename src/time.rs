extern crate time;

pub fn get_time() -> u64 {
  let ts = time::get_time();
  (ts.sec as u64) * 1000 + (ts.nsec as u64) / 1_000_000
}

pub fn elapsed_time(start_time: u64) -> u32 {
  (get_time() - start_time) as u32
}
