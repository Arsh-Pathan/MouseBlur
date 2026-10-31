use std::collections::VecDeque;
use std::time::Instant;
use windows_numerics::Vector2;

#[derive(Debug, Clone, Copy)]
pub struct TrailPoint {
    pub x: f32,
    pub y: f32,
    pub time: Instant,
    pub velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[inline(always)]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline(always)]
    pub fn length_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_sq().sqrt()
    }

    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0001 {
            let inv_len = 1.0 / len;
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
            }
        } else {
            Self { x: 0.0, y: 0.0 }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub decay: f32,
    pub size: f32,
}

pub struct MouseTracker {
    history: VecDeque<TrailPoint>,
    particles: Vec<Particle>,
    current_velocity: f32,
    last_record_time: Instant,
    last_pos: (f32, f32),
    rng_state: u64,
    // Pre-allocated scratch buffers to achieve 0 heap allocations during rendering
    spline_buf: Vec<(Vec2, f32, f32)>,
    left_strip_buf: Vec<Vec2>,
    right_strip_buf: Vec<Vec2>,
    polygon_buf: Vec<Vec2>,
    speed_lines_buf: Vec<(Vec2, Vec2, f32)>,
    plasma_arcs_buf: Vec<(Vec2, Vec2, f32)>,
    afterimages_buf: Vec<(Vec2, f32)>,
    d2d_ribbon_buf: Vec<Vector2>,
    motion_samples_buf: Vec<(Vec2, Vec2, f32)>,
}

#[allow(dead_code)]
impl MouseTracker {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            history: VecDeque::with_capacity(128),
            particles: Vec::with_capacity(256),
            current_velocity: 0.0,
            last_record_time: now,
            last_pos: (0.0, 0.0),
            rng_state: 123456789,
            spline_buf: Vec::with_capacity(256),
            left_strip_buf: Vec::with_capacity(256),
            right_strip_buf: Vec::with_capacity(256),
            polygon_buf: Vec::with_capacity(512),
            speed_lines_buf: Vec::with_capacity(16),
            plasma_arcs_buf: Vec::with_capacity(128),
            afterimages_buf: Vec::with_capacity(16),
            d2d_ribbon_buf: Vec::with_capacity(512),
            motion_samples_buf: Vec::with_capacity(128),
        }
    }

    #[inline(always)]
    fn rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        ((self.rng_state & 0x00FF_FFFF) as f32) * (1.0 / 16777216.0)
    }

    pub fn record_position(&mut self, x: f32, y: f32, max_points: usize, tail_duration_ms: f32) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_record_time).as_secs_f32();

        let dx = x - self.last_pos.0;
        let dy = y - self.last_pos.1;
        let dist_sq = dx * dx + dy * dy;

        let inst_vel = if dt > 0.0001 { dist_sq.sqrt() / dt } else { 0.0 };

        // Exponential smoothing for velocity
        let alpha = 0.45;
        self.current_velocity = alpha * inst_vel + (1.0 - alpha) * self.current_velocity;

        // Sub-pixel threshold: capture fine micro-movements (>0.4px) for buttery continuous curves
        let should_record = dist_sq > 0.16 || (dt > 0.002 && dist_sq > 0.0);

        if should_record {
            self.history.push_back(TrailPoint {
                x,
                y,
                time: now,
                velocity: self.current_velocity,
            });

            // Emit particles when moving rapidly
            if self.current_velocity > 160.0 {
                let tangent = Vec2::new(dx, dy).normalize();
                let normal = Vec2::new(-tangent.y, tangent.x);

                let count = (self.current_velocity * (1.0 / 400.0)).clamp(1.0, 4.0) as usize;
                for _ in 0..count {
                    let side = (self.rand_f32() - 0.5) * 16.0;
                    let speed = (self.rand_f32() * 80.0) + 20.0;
                    let decay = 2.5 + self.rand_f32() * 2.0;
                    let size = 2.0 + self.rand_f32() * 2.5;

                    let p_vel = Vec2::new(
                        -tangent.x * speed + normal.x * side * 4.0,
                        -tangent.y * speed + normal.y * side * 4.0,
                    );
                    self.particles.push(Particle {
                        pos: Vec2::new(x + normal.x * side, y + normal.y * side),
                        vel: p_vel,
                        life: 1.0,
                        decay,
                        size,
                    });
                }
            }

            self.last_pos = (x, y);
            self.last_record_time = now;
        }

        self.update_particles(dt.clamp(0.001, 0.05));
        self.prune(now, tail_duration_ms, max_points);
    }

    pub fn update_particles(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
            p.vel.x *= 0.92;
            p.vel.y *= 0.92;
            p.life -= p.decay * dt;

            if p.life <= 0.0 {
                self.particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    #[inline]
    pub fn prune(&mut self, now: Instant, tail_duration_ms: f32, max_points: usize) {
        let max_age_secs = tail_duration_ms * 0.001;
        while let Some(front) = self.history.front() {
            if now.duration_since(front.time).as_secs_f32() > max_age_secs || self.history.len() > max_points {
                self.history.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.particles.clear();
        self.current_velocity = 0.0;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.history.len()
    }

    #[inline]
    pub fn current_velocity(&self) -> f32 {
        self.current_velocity
    }

    pub fn get_points(&self) -> &VecDeque<TrailPoint> {
        &self.history
    }

    pub fn get_particles(&self) -> &[Particle] {
        &self.particles
    }

    #[inline]
    pub fn is_active(&self, tail_duration_ms: f32) -> bool {
        if !self.particles.is_empty() {
            return true;
        }
        if self.history.len() < 2 {
            return false;
        }
        let now = Instant::now();
        let max_age_secs = tail_duration_ms * 0.001;
        if let Some(back) = self.history.back() {
            now.duration_since(back.time).as_secs_f32() < max_age_secs
        } else {
            false
        }
    }

    /// Zero-allocation ribbon generation reusing internal buffers
    pub fn generate_ribbon(
        &mut self,
        head_width: f32,
        tail_width: f32,
        min_vel: f32,
        max_vel: f32,
        tail_duration_ms: f32,
    ) -> Option<&[Vec2]> {
        self.generate_ribbon_offset(head_width, tail_width, min_vel, max_vel, tail_duration_ms, 0.0)
    }

    /// Zero-allocation ribbon generation with normal offset
    pub fn generate_ribbon_offset(
        &mut self,
        head_width: f32,
        tail_width: f32,
        min_vel: f32,
        max_vel: f32,
        tail_duration_ms: f32,
        normal_offset: f32,
    ) -> Option<&[Vec2]> {
        let n = self.history.len();
        if n < 2 {
            return None;
        }

        let now = Instant::now();
        let duration_secs = (tail_duration_ms * 0.001).max(0.001);
        let inv_duration = 1.0 / duration_secs;

        self.spline_buf.clear();

        for i in 0..n - 1 {
            let p0 = if i > 0 { self.history[i - 1] } else { self.history[i] };
            let p1 = self.history[i];
            let p2 = self.history[i + 1];
            let p3 = if i + 2 < n { self.history[i + 2] } else { self.history[i + 1] };

            let steps = 4;
            let step_inv = 1.0 / steps as f32;
            for step in 0..steps {
                let t = step as f32 * step_inv;
                let pt = catmull_rom(
                    Vec2::new(p0.x, p0.y),
                    Vec2::new(p1.x, p1.y),
                    Vec2::new(p2.x, p2.y),
                    Vec2::new(p3.x, p3.y),
                    t,
                );

                let p1_age = duration_secs - now.duration_since(p1.time).as_secs_f32();
                let p2_age = duration_secs - now.duration_since(p2.time).as_secs_f32();
                let age = (p1_age * (1.0 - t) + p2_age * t).max(0.0);
                let progress = (age * inv_duration).clamp(0.0, 1.0);

                let vel = p1.velocity * (1.0 - t) + p2.velocity * t;
                self.spline_buf.push((pt, progress, vel));
            }
        }

        if let Some(last) = self.history.back() {
            let vel = last.velocity;
            let age = duration_secs - now.duration_since(last.time).as_secs_f32();
            let progress = (age * inv_duration).clamp(0.0, 1.0);
            self.spline_buf.push((Vec2::new(last.x, last.y), progress, vel));
        }

        let m = self.spline_buf.len();
        if m < 3 {
            return None;
        }

        self.left_strip_buf.clear();
        self.right_strip_buf.clear();
        let inv_vel_range = if max_vel > min_vel { 1.0 / (max_vel - min_vel) } else { 1.0 };

        for i in 0..m {
            let (pos, progress, vel) = self.spline_buf[i];

            let tangent = if i == 0 {
                Vec2::new(self.spline_buf[1].0.x - pos.x, self.spline_buf[1].0.y - pos.y).normalize()
            } else if i == m - 1 {
                Vec2::new(pos.x - self.spline_buf[i - 1].0.x, pos.y - self.spline_buf[i - 1].0.y).normalize()
            } else {
                Vec2::new(
                    self.spline_buf[i + 1].0.x - self.spline_buf[i - 1].0.x,
                    self.spline_buf[i + 1].0.y - self.spline_buf[i - 1].0.y,
                )
                .normalize()
            };

            let normal = Vec2::new(-tangent.y, tangent.x);
            let vel_factor = if max_vel > min_vel {
                ((vel - min_vel) * inv_vel_range).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let current_width = (tail_width + (head_width - tail_width) * progress.powf(1.4)) * vel_factor;
            let half_w = current_width * 0.5;

            let center_x = pos.x + normal.x * normal_offset;
            let center_y = pos.y + normal.y * normal_offset;

            self.left_strip_buf.push(Vec2::new(center_x + normal.x * half_w, center_y + normal.y * half_w));
            self.right_strip_buf.push(Vec2::new(center_x - normal.x * half_w, center_y - normal.y * half_w));
        }

        self.polygon_buf.clear();
        self.polygon_buf.extend_from_slice(&self.left_strip_buf);
        for pt in self.right_strip_buf.iter().rev() {
            self.polygon_buf.push(*pt);
        }

        Some(&self.polygon_buf)
    }

    /// Zero-allocation D2D ribbon generation with viewport translation
    pub fn generate_ribbon_d2d(
        &mut self,
        head_width: f32,
        tail_width: f32,
        min_vel: f32,
        max_vel: f32,
        tail_duration_ms: f32,
        normal_offset: f32,
        origin_x: f32,
        origin_y: f32,
    ) -> Option<&[Vector2]> {
        if self
            .generate_ribbon_offset(head_width, tail_width, min_vel, max_vel, tail_duration_ms, normal_offset)
            .is_none()
        {
            return None;
        }

        self.d2d_ribbon_buf.clear();
        self.d2d_ribbon_buf.reserve(self.polygon_buf.len());
        for pt in &self.polygon_buf {
            self.d2d_ribbon_buf.push(Vector2 {
                X: pt.x - origin_x,
                Y: pt.y - origin_y,
            });
        }
        Some(&self.d2d_ribbon_buf)
    }

    /// Zero-allocation speed lines generation
    pub fn generate_speed_lines(&mut self, count: u32) -> &[(Vec2, Vec2, f32)] {
        self.speed_lines_buf.clear();
        if self.history.len() < 3 || count == 0 {
            return &self.speed_lines_buf;
        }

        let n = self.history.len();
        let head = self.history[n - 1];
        let mid = self.history[n - 2];

        let dir = Vec2::new(head.x - mid.x, head.y - mid.y);
        let dist = dir.length();
        if dist < 4.0 {
            return &self.speed_lines_buf;
        }

        let tangent = dir.normalize();
        let normal = Vec2::new(-tangent.y, tangent.x);

        let half_count = (count as f32) * 0.5;
        let spacing = 7.0;
        let inv_half_range = 1.0 / (half_count * spacing);

        for i in 0..count {
            let offset = (i as f32 - half_count + 0.5) * spacing;
            let line_len = (dist * (1.2 + (i as f32 % 3.0) * 0.4)).clamp(15.0, 120.0);

            let p_start = Vec2::new(head.x + normal.x * offset, head.y + normal.y * offset);
            let p_end = Vec2::new(
                p_start.x - tangent.x * line_len,
                p_start.y - tangent.y * line_len,
            );

            let alpha = (1.0 - (offset.abs() * inv_half_range)).clamp(0.2, 1.0);
            self.speed_lines_buf.push((p_start, p_end, alpha));
        }

        &self.speed_lines_buf
    }

    /// Zero-allocation fractal plasma arcs generation
    pub fn generate_plasma_arcs(&mut self, jitter_amount: f32) -> &[(Vec2, Vec2, f32)] {
        self.plasma_arcs_buf.clear();
        let n = self.history.len();
        if n < 2 {
            return &self.plasma_arcs_buf;
        }

        let inv_n = 1.0 / n as f32;
        for i in 0..n - 1 {
            let p1 = self.history[i];
            let p2 = self.history[i + 1];

            let a = Vec2::new(p1.x, p1.y);
            let b = Vec2::new(p2.x, p2.y);
            let dir = Vec2::new(b.x - a.x, b.y - a.y);
            let normal = Vec2::new(-dir.y, dir.x).normalize();

            let mid = Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
            let j = (self.rand_f32() - 0.5) * jitter_amount;
            let displaced_mid = Vec2::new(mid.x + normal.x * j, mid.y + normal.y * j);

            let alpha = ((i + 1) as f32 * inv_n).clamp(0.2, 1.0);
            self.plasma_arcs_buf.push((a, displaced_mid, alpha));
            self.plasma_arcs_buf.push((displaced_mid, b, alpha));
        }

        &self.plasma_arcs_buf
    }

    /// Zero-allocation afterimages sampling
    pub fn get_afterimages(&mut self, count: usize, spacing_ms: f32, tail_duration_ms: f32) -> &[(Vec2, f32)] {
        self.afterimages_buf.clear();
        let n = self.history.len();
        if n < 2 || count == 0 {
            return &self.afterimages_buf;
        }

        let now = Instant::now();
        let spacing_secs = spacing_ms * 0.001;
        let max_age_secs = (tail_duration_ms * 0.001).max(0.001);
        let inv_max_age = 1.0 / max_age_secs;

        let mut target_time_offset = spacing_secs;

        for _ in 0..count {
            if target_time_offset > max_age_secs {
                break;
            }

            let target_age = target_time_offset;
            let mut found = None;

            for i in (0..n - 1).rev() {
                let p1 = &self.history[i + 1];
                let p0 = &self.history[i];
                let age1 = now.duration_since(p1.time).as_secs_f32();
                let age0 = now.duration_since(p0.time).as_secs_f32();

                if age1 <= target_age && target_age <= age0 {
                    let dt = age0 - age1;
                    let factor = if dt > 0.0001 { (target_age - age1) / dt } else { 0.0 };
                    let x = p1.x * (1.0 - factor) + p0.x * factor;
                    let y = p1.y * (1.0 - factor) + p0.y * factor;
                    let alpha = (1.0 - (target_age * inv_max_age)).clamp(0.0, 1.0);
                    found = Some((Vec2::new(x, y), alpha));
                    break;
                }
            }

            if let Some(res) = found {
                self.afterimages_buf.push(res);
            }

            target_time_offset += spacing_secs;
        }

        &self.afterimages_buf
    }

    /// Zero-allocation dense motion blur sampling along the shutter exposure interval
    pub fn get_motion_blur_samples(
        &mut self,
        shutter_duration_ms: f32,
        min_vel: f32,
        max_vel: f32,
    ) -> &[(Vec2, Vec2, f32)] {
        self.motion_samples_buf.clear();

        let n = self.history.len();
        if n < 2 {
            return &self.motion_samples_buf;
        }

        let now = Instant::now();
        let shutter_secs = (shutter_duration_ms * 0.001).clamp(0.005, 0.080);

        // Find how many recent points fall within the shutter exposure window
        let mut start_idx = n - 1;
        while start_idx > 0 {
            let age = now.duration_since(self.history[start_idx - 1].time).as_secs_f32();
            if age > shutter_secs {
                break;
            }
            start_idx -= 1;
        }

        if n - start_idx < 2 {
            if start_idx > 0 {
                start_idx -= 1;
            } else {
                return &self.motion_samples_buf;
            }
        }

        // Calculate total distance traveled during this shutter exposure
        let mut total_dist = 0.0;
        for i in start_idx..n - 1 {
            let p0 = &self.history[i];
            let p1 = &self.history[i + 1];
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            total_dist += (dx * dx + dy * dy).sqrt();
        }

        // Under 1.2 pixels movement -> sharp cursor, no blur
        if total_dist < 1.2 {
            return &self.motion_samples_buf;
        }

        // Velocity scaling: smooth falloff at low speed
        let vel_factor = if max_vel > min_vel {
            ((self.current_velocity - min_vel) / (max_vel - min_vel)).clamp(0.0, 1.0)
        } else {
            1.0
        };

        if vel_factor < 0.02 {
            return &self.motion_samples_buf;
        }

        // Sub-pixel continuous sampling density: ~0.80 pixels between samples yields liquid, velvet blur without gaps
        let sample_count = (total_dist / 0.80).round().clamp(10.0, 96.0) as usize;
        let step_inv = 1.0 / (sample_count - 1) as f32;
        let step_dist = total_dist * step_inv;
        // Physical shutter exposure density normalization:
        // As samples are placed denser, each sample's weight is scaled by step_dist so integrated luminance is invariant
        let density_scale = (step_dist / 14.0).clamp(0.06, 0.40);

        for s in 0..sample_count {
            let t = s as f32 * step_inv; // 0.0 = oldest shutter point, 1.0 = newest current cursor point
            let target_d = t * total_dist;

            let mut accum_d = 0.0;
            let mut seg_idx = start_idx;
            let mut seg_t = 0.0;

            for i in start_idx..n - 1 {
                let p0 = &self.history[i];
                let p1 = &self.history[i + 1];
                let dx = p1.x - p0.x;
                let dy = p1.y - p0.y;
                let d = (dx * dx + dy * dy).sqrt();

                if accum_d + d >= target_d || i == n - 2 {
                    seg_idx = i;
                    seg_t = if d > 0.0001 { (target_d - accum_d) / d } else { 0.0 };
                    break;
                }
                accum_d += d;
            }

            let p0 = if seg_idx > 0 { self.history[seg_idx - 1] } else { self.history[seg_idx] };
            let p1 = self.history[seg_idx];
            let p2 = self.history[seg_idx + 1];
            let p3 = if seg_idx + 2 < n { self.history[seg_idx + 2] } else { self.history[seg_idx + 1] };

            let pos = catmull_rom(
                Vec2::new(p0.x, p0.y),
                Vec2::new(p1.x, p1.y),
                Vec2::new(p2.x, p2.y),
                Vec2::new(p3.x, p3.y),
                seg_t.clamp(0.0, 1.0),
            );

            let dir = Vec2::new(p2.x - p1.x, p2.y - p1.y);
            let tangent = dir.normalize();
            let normal = Vec2::new(-tangent.y, tangent.x);

            // True optical shutter falloff:
            // S(t) = t * (2 - t) smoothly ramps up towards the current cursor tip
            // giving natural shutter dissolution at the tail without abrupt clipping
            let weight = (t * (2.0 - t)).powf(1.1);
            let rel_alpha = (weight * density_scale * vel_factor).clamp(0.0, 1.0);

            self.motion_samples_buf.push((pos, normal, rel_alpha));
        }

        &self.motion_samples_buf
    }
}

/// Inlined SIMD-friendly Catmull-Rom spline interpolation
#[inline(always)]
fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;

    let x = 0.5 * (
        (2.0 * p1.x) +
        (-p0.x + p2.x) * t +
        (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2 +
        (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3
    );

    let y = 0.5 * (
        (2.0 * p1.y) +
        (-p0.y + p2.y) * t +
        (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2 +
        (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3
    );

    Vec2::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_length_and_normalize() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 0.0001);

        let norm = v.normalize();
        assert!((norm.length() - 1.0).abs() < 0.0001);
        assert!((norm.x - 0.6).abs() < 0.0001);
        assert!((norm.y - 0.8).abs() < 0.0001);
    }

    #[test]
    fn test_catmull_rom_endpoints() {
        let p0 = Vec2::new(0.0, 0.0);
        let p1 = Vec2::new(10.0, 0.0);
        let p2 = Vec2::new(20.0, 10.0);
        let p3 = Vec2::new(30.0, 10.0);

        let start = catmull_rom(p0, p1, p2, p3, 0.0);
        assert!((start.x - p1.x).abs() < 0.0001);
        assert!((start.y - p1.y).abs() < 0.0001);

        let end = catmull_rom(p0, p1, p2, p3, 1.0);
        assert!((end.x - p2.x).abs() < 0.0001);
        assert!((end.y - p2.y).abs() < 0.0001);
    }

    #[test]
    fn test_speed_lines_and_plasma_arcs() {
        let mut tracker = MouseTracker::new();
        tracker.record_position(100.0, 100.0, 36, 120.0);
        tracker.record_position(120.0, 105.0, 36, 120.0);
        tracker.record_position(150.0, 115.0, 36, 120.0);
        tracker.record_position(220.0, 130.0, 36, 120.0);

        let lines = tracker.generate_speed_lines(5);
        assert_eq!(lines.len(), 5);

        let arcs = tracker.generate_plasma_arcs(10.0);
        assert!(!arcs.is_empty());
    }

    #[test]
    fn test_particles_emission() {
        let mut tracker = MouseTracker::new();
        tracker.record_position(0.0, 0.0, 36, 120.0);
        tracker.current_velocity = 800.0;
        tracker.record_position(150.0, 150.0, 36, 120.0);

        assert!(!tracker.get_particles().is_empty());
        let initial_count = tracker.get_particles().len();
        tracker.update_particles(0.1);
        assert!(tracker.get_particles().len() <= initial_count);
    }

    #[test]
    fn test_motion_blur_samples() {
        let mut tracker = MouseTracker::new();
        tracker.record_position(100.0, 100.0, 32, 24.0);
        tracker.current_velocity = 600.0;
        tracker.record_position(130.0, 140.0, 32, 24.0);

        let samples = tracker.get_motion_blur_samples(24.0, 40.0, 2000.0);
        assert!(!samples.is_empty());
        assert!(samples.len() >= 6);
    }
}
