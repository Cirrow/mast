use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    window: Duration,
    max: usize,
    times: Mutex<HashMap<String, Vec<Instant>>>,
}
impl RateLimiter {
    pub fn new(window: Duration, max: usize) -> Self {
        Self {
            window,
            max,
            times: Mutex::new(HashMap::new()),
        }
    }

    pub fn hit(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.times.lock().unwrap();
        let hits = map.entry(key.to_string()).or_default();
        hits.retain(|t| now.duration_since(*t) < self.window);
        if hits.len() >= self.max {
            false
        } else {
            hits.push(now);
            self.sweep(&mut map);
            true
        }
    }

    pub fn is_allowed(&self, key: &str) -> bool {
        let now = Instant::now();
        let map = self.times.lock().unwrap();
        match map.get(key) {
            Some(hits) => {
                hits.iter()
                    .filter(|t| now.duration_since(**t) < self.window)
                    .count()
                    < self.max
            }
            None => true,
        }
    }

    pub fn reset(&self, key: &str) {
        self.times.lock().unwrap().remove(key);
    }

    fn sweep(&self, map: &mut HashMap<String, Vec<Instant>>) {
        if map.len() > 10_000 {
            let now = Instant::now();
            map.retain(|_, hits| hits.iter().any(|t| now.duration_since(*t) < self.window));
        }
    }
}
