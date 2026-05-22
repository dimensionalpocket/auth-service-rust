use std::time::{SystemTime, UNIX_EPOCH};

pub struct GetServerTimestampService;

impl GetServerTimestampService {
  pub fn run() -> u64 {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Time went backwards")
      .as_millis() as u64
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;
  use std::time::Duration;

  #[test]
  fn test_get_server_timestamp_returns_positive_value() {
    let timestamp = GetServerTimestampService::run();
    assert!(timestamp > 0);
  }

  #[test]
  fn test_get_server_timestamp_increases_over_time() {
    let timestamp1 = GetServerTimestampService::run();
    thread::sleep(Duration::from_millis(1));
    let timestamp2 = GetServerTimestampService::run();
    assert!(timestamp2 > timestamp1);
  }

  #[test]
  fn test_get_server_timestamp_reasonable_range() {
    let timestamp = GetServerTimestampService::run();
    // Should be after 2020-01-01 and before 2030-01-01
    let min_timestamp = 1577836800000u64; // 2020-01-01 in ms
    let max_timestamp = 1893456000000u64; // 2030-01-01 in ms
    assert!(timestamp >= min_timestamp);
    assert!(timestamp <= max_timestamp);
  }
}
