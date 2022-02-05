use std::{
    ops::Add,
    time::{Duration, Instant},
};

pub struct Timer {
    reference: Instant,
    last_update: Instant,
    current_durations: Vec<u128>,
    average_durations: Vec<u128>,
    is_stopped: bool,
}

impl Timer {
    const BATCH_DURATION_SECS: usize = 3;

    pub fn new() -> Self {
        Self {
            reference: Instant::now(),
            last_update: Instant::now(),
            current_durations: vec![],
            average_durations: vec![],
            is_stopped: false,
        }
    }

    #[inline]
    pub fn start_frame(&mut self) {
        let now = Instant::now();
        if self.is_stopped {
            self.reset();
            return;
        }
        let needs_new_batch = need_new_batch(&now, &self.reference, Self::BATCH_DURATION_SECS);

        if needs_new_batch {
            self.finish_batch();
        }

        self.current_durations
            .push((now - self.last_update).as_millis());
        self.last_update = now;
    }

    #[inline]
    fn reset(&mut self) {
        let now = Instant::now();
        self.reference = now;
        self.last_update = now;
        self.is_stopped = false;
    }

    #[inline]
    pub fn pause(&mut self) {
        self.is_stopped = true;
    }

    #[inline]
    fn finish_batch(&mut self) {
        let avg = Self::durations_avg(&self.current_durations);
        self.current_durations.clear();
        self.average_durations.push(avg);
    }

    #[inline]
    fn durations_avg(durations: &[u128]) -> u128 {
        let mut total = 0;
        let mut iters = 0;
        for duration in durations {
            total += duration;
            iters += 1;
        }
        return (total as f64 / iters as f64) as u128;
    }

    /// This will only print when a batch has finished.
    #[inline]
    pub fn display(&self) {
        #[cfg(debug_assertions)]
        if self.current_durations.len() == 1 {
            let batch = if self.average_durations.len() == 0 {
                &0
            } else {
                self.average_durations.last().unwrap()
            };
            let average = Self::durations_avg(&self.average_durations);
            let batch_fps = 1000 as f32 / *batch.max(&1) as f32;
            let average_fps = 1000 as f32 / average.max(1) as f32;
            if batch_fps >= 500.0 || average_fps >= 500.0 {
                return;
            }
            println!(
                "[TIMER]: {:.2}fps (latest) | {:.2}fps (average)",
                batch_fps, average_fps
            );
        }
    }
}

fn need_new_batch(now: &Instant, batch_start: &Instant, length_of_batch_sec: usize) -> bool {
    let target_moment = batch_start.add(Duration::from_secs(length_of_batch_sec as u64));
    if target_moment <= *now {
        return true;
    }
    false
}
