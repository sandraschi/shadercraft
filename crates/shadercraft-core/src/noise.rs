//! Deterministic CPU noise primitives for ShaderCraft.
//!
//! Every function here is a pure function of its inputs: the same seed and
//! coordinates always produce the same bytes on every machine, so presets
//! render identically in the studio, the CLI, and agent automation. There is
//! no GPU dependency and no global RNG state.

/// Large odd constant used to decorrelate per-octave and per-channel hashes.
const GOLDEN: u64 = 0x9E3779B97F4A7C15;

/// Mix integer lattice coordinates and a seed into a 64-bit hash (splitmix64).
pub fn hash2(ix: i32, iy: i32, seed: u64) -> u64 {
    let mut z = (ix as u64)
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((iy as u64).wrapping_mul(0xBF58476D1CE4E5B9))
        .wrapping_add(seed.wrapping_mul(0x94D049BB133111EB));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Deterministic lattice value in `[0, 1]` from integer coordinates and seed.
pub fn lattice(ix: i32, iy: i32, seed: u64) -> f32 {
    ((hash2(ix, iy, seed) >> 40) as f32) * (1.0 / 16_777_216.0)
}

fn wrap_coord(c: i32, period: u32) -> i32 {
    let p = period.max(1) as i32;
    ((c % p) + p) % p
}

fn smoother(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in `[0, 1]`. Coordinates are in cell units. When `period` is
/// set, lattice coordinates wrap modulo `period`, which makes the noise tile
/// seamlessly over `period` cells.
pub fn value_noise(x: f32, y: f32, seed: u64, period: Option<u32>) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;
    let wrap = |c: i32| match period {
        Some(p) => wrap_coord(c, p),
        None => c,
    };
    let a = lattice(wrap(xi), wrap(yi), seed);
    let b = lattice(wrap(xi + 1), wrap(yi), seed);
    let c = lattice(wrap(xi), wrap(yi + 1), seed);
    let d = lattice(wrap(xi + 1), wrap(yi + 1), seed);
    let u = smoother(xf);
    let v = smoother(yf);
    (a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v).clamp(0.0, 1.0)
}

fn grad_vec(h: u64) -> (f32, f32) {
    match (h >> 32) % 8 {
        0 => (1.0, 0.0),
        1 => (-1.0, 0.0),
        2 => (0.0, 1.0),
        3 => (0.0, -1.0),
        4 => (1.0, 1.0),
        5 => (-1.0, 1.0),
        6 => (1.0, -1.0),
        _ => (-1.0, -1.0),
    }
}

/// Perlin-style gradient noise in `[0, 1]`. Same tiling contract as
/// [`value_noise`].
pub fn gradient_noise(x: f32, y: f32, seed: u64, period: Option<u32>) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;
    let wrap = |c: i32| match period {
        Some(p) => wrap_coord(c, p),
        None => c,
    };
    let dot = |cx: i32, cy: i32, ox: f32, oy: f32| {
        let (gx, gy) = grad_vec(hash2(wrap(cx), wrap(cy), seed));
        gx * ox + gy * oy
    };
    let a = dot(xi, yi, xf, yf);
    let b = dot(xi + 1, yi, xf - 1.0, yf);
    let c = dot(xi, yi + 1, xf, yf - 1.0);
    let d = dot(xi + 1, yi + 1, xf - 1.0, yf - 1.0);
    let u = fade(xf);
    let v = fade(yf);
    let n = a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
    (n * 0.5 + 0.5).clamp(0.0, 1.0)
}

/// Voronoi cell noise. Returns `(f1, f2)`: distance to the nearest and
/// second-nearest feature points. Deltas wrap around `period` when set, so
/// the cells tile seamlessly.
pub fn voronoi(x: f32, y: f32, seed: u64, jitter: f32, period: Option<u32>) -> (f32, f32) {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    for j in -1..=1 {
        for i in -1..=1 {
            let nx = xi + i;
            let ny = yi + j;
            let (wx, wy) = match period {
                Some(p) => (wrap_coord(nx, p), wrap_coord(ny, p)),
                None => (nx, ny),
            };
            let rx = lattice(wx, wy, seed);
            let ry = lattice(wx, wy, seed ^ GOLDEN);
            // Keep the whole-cell offset separate from the fractional jitter:
            // (cell - sample) is exact for tile-sized coordinates, so probing
            // a period boundary is bit-identical to probing the origin.
            let ox = (rx - 0.5) * 2.0 * jitter;
            let oy = (ry - 0.5) * 2.0 * jitter;
            let mut dx = (nx as f32 - x) + ox;
            let mut dy = (ny as f32 - y) + oy;
            if let Some(p) = period {
                if p > 0 {
                    let pf = p as f32;
                    dx -= (dx / pf).round() * pf;
                    dy -= (dy / pf).round() * pf;
                }
            }
            let d = dx.hypot(dy);
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
}

/// Fractal Brownian motion over value noise in `[0, 1]`. Each octave gets a
/// decorrelated seed and, when `period` is set, a matching wrap period so the
/// sum still tiles seamlessly.
pub fn fbm(
    x: f32,
    y: f32,
    seed: u64,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
    period: Option<u32>,
) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut norm = 0.0;
    let mut n = 0;
    while n < octaves.max(1) {
        let p = period.map(|bp| {
            let v = (bp as f32 * freq).round() as u32;
            v.max(1)
        });
        sum += amp * value_noise(x * freq, y * freq, seed.wrapping_add(n as u64), p);
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
        n += 1;
    }
    if norm <= 0.0 {
        0.5
    } else {
        (sum / norm).clamp(0.0, 1.0)
    }
}
