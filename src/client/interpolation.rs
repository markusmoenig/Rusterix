use instant::Instant;
use std::collections::VecDeque;
use std::time::Duration;
use vek::Vec2;

#[derive(Clone, Copy, Debug)]
struct TimedPosition {
    time: Instant,
    pos: Vec2<f32>,
}

const INTERPOLATION_DELAY: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct InterpolationBuffer {
    buffer: VecDeque<TimedPosition>,
}

impl InterpolationBuffer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn add_position(&mut self, pos: Vec2<f32>) {
        let now = Instant::now();

        self.buffer.push_back(TimedPosition { time: now, pos });

        // Remove old positions (older than 500ms)
        while self.buffer.len() > 2 && self.buffer[1].time < now - Duration::from_millis(500) {
            self.buffer.pop_front();
        }
    }

    pub fn get_interpolated(&self) -> Vec2<f32> {
        let target_time = Instant::now() - INTERPOLATION_DELAY;

        for i in 0..self.buffer.len().saturating_sub(1) {
            let a = self.buffer[i];
            let b = self.buffer[i + 1];

            if target_time >= a.time && target_time <= b.time {
                let total = b.time.duration_since(a.time).as_secs_f32();
                let alpha = (target_time - a.time).as_secs_f32() / total;
                return Vec2::lerp(a.pos, b.pos, alpha);
            }
        }

        // Fallback: return latest known position
        self.buffer.back().map(|tp| tp.pos).unwrap_or(Vec2::zero())
    }
}
