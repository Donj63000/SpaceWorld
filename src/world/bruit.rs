#[inline]
pub(crate) fn saturate(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[inline]
pub(crate) fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[inline]
pub(crate) fn inverse_lerp(a: f64, b: f64, value: f64) -> f64 {
    if (b - a).abs() <= f64::EPSILON {
        return 0.0;
    }
    saturate((value - a) / (b - a))
}

#[inline]
pub(crate) fn remap_clamped(
    from_min: f64,
    from_max: f64,
    to_min: f64,
    to_max: f64,
    value: f64,
) -> f64 {
    lerp(to_min, to_max, inverse_lerp(from_min, from_max, value))
}

#[inline]
pub(crate) fn smoothstep01(value: f64) -> f64 {
    let x = saturate(value);
    x * x * (3.0 - 2.0 * x)
}

#[inline]
pub(crate) fn smootherstep01(value: f64) -> f64 {
    let x = saturate(value);
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

#[inline]
pub(crate) fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[inline]
pub(crate) fn hash_coords(seed: u64, x: i32, y: i32) -> u64 {
    let a = splitmix64(seed ^ (x as i64 as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    let b = splitmix64((y as i64 as u64) ^ 0xA5A5_A5A5_5A5A_5A5A);
    splitmix64(a ^ b.rotate_left(29))
}

#[inline]
pub(crate) fn unit_f64_from_hash(value: u64) -> f64 {
    ((value >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
}

#[inline]
pub(crate) fn rand01(seed: u64, x: i32, y: i32) -> f32 {
    unit_f64_from_hash(hash_coords(seed, x, y)) as f32
}

#[inline]
pub(crate) fn rand_signed(seed: u64, x: i32, y: i32) -> f64 {
    unit_f64_from_hash(hash_coords(seed, x, y)) * 2.0 - 1.0
}

#[inline]
fn gradient_from_hash(hash: u64) -> (f64, f64) {
    match (hash & 7) as u8 {
        0 => (1.0, 0.0),
        1 => (-1.0, 0.0),
        2 => (0.0, 1.0),
        3 => (0.0, -1.0),
        4 => (0.70710678118, 0.70710678118),
        5 => (-0.70710678118, 0.70710678118),
        6 => (0.70710678118, -0.70710678118),
        _ => (-0.70710678118, -0.70710678118),
    }
}

#[inline]
pub(crate) fn gradient_noise(seed: u64, x: f64, y: f64) -> f64 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let fx = x - x.floor();
    let fy = y - y.floor();
    let u = smootherstep01(fx);
    let v = smootherstep01(fy);

    let (g00x, g00y) = gradient_from_hash(hash_coords(seed, x0, y0));
    let (g10x, g10y) = gradient_from_hash(hash_coords(seed, x1, y0));
    let (g01x, g01y) = gradient_from_hash(hash_coords(seed, x0, y1));
    let (g11x, g11y) = gradient_from_hash(hash_coords(seed, x1, y1));

    let n00 = g00x * fx + g00y * fy;
    let n10 = g10x * (fx - 1.0) + g10y * fy;
    let n01 = g01x * fx + g01y * (fy - 1.0);
    let n11 = g11x * (fx - 1.0) + g11y * (fy - 1.0);

    let nx0 = lerp(n00, n10, u);
    let nx1 = lerp(n01, n11, u);
    lerp(nx0, nx1, v) * 1.41421356237
}

#[inline]
pub(crate) fn fbm(seed: u64, x: f64, y: f64, octaves: usize, lacunarity: f64, gain: f64) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut normalisation = 0.0;

    for octave in 0..octaves {
        total += gradient_noise(
            seed ^ ((octave as u64 + 1) * 0x9E37_79B9),
            x * frequency,
            y * frequency,
        ) * amplitude;
        normalisation += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    if normalisation <= f64::EPSILON {
        0.0
    } else {
        total / normalisation
    }
}

#[inline]
pub(crate) fn billow_fbm(
    seed: u64,
    x: f64,
    y: f64,
    octaves: usize,
    lacunarity: f64,
    gain: f64,
) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut normalisation = 0.0;

    for octave in 0..octaves {
        let n = gradient_noise(
            seed ^ ((octave as u64 + 1) * 0x85EB_CA6B),
            x * frequency,
            y * frequency,
        );
        total += (n.abs() * 2.0 - 1.0) * amplitude;
        normalisation += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    if normalisation <= f64::EPSILON {
        0.0
    } else {
        total / normalisation
    }
}

#[inline]
pub(crate) fn ridged_fbm(
    seed: u64,
    x: f64,
    y: f64,
    octaves: usize,
    lacunarity: f64,
    gain: f64,
) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 0.9;
    let mut frequency = 1.0;
    let mut normalisation = 0.0;
    let mut weight = 1.0;

    for octave in 0..octaves {
        let signal = 1.0
            - gradient_noise(
                seed ^ ((octave as u64 + 1) * 0xC2B2_AE35),
                x * frequency,
                y * frequency,
            )
            .abs();
        let signal = signal * signal;
        let weighted = signal * weight;
        total += weighted * amplitude;
        normalisation += amplitude;
        weight = (weighted * 1.85).clamp(0.0, 1.0);
        amplitude *= gain;
        frequency *= lacunarity;
    }

    if normalisation <= f64::EPSILON {
        0.0
    } else {
        total / normalisation
    }
}

#[inline]
pub(crate) fn domain_warp(
    seed: u64,
    x: f64,
    y: f64,
    noise_scale: f64,
    amplitude: f64,
) -> (f64, f64) {
    let wx = fbm(
        seed ^ 0xA0F1_DA71,
        x * noise_scale + 17.3,
        y * noise_scale - 29.7,
        3,
        2.07,
        0.5,
    );
    let wy = fbm(
        seed ^ 0x5E71_C0DE,
        x * noise_scale - 41.2,
        y * noise_scale + 11.9,
        3,
        2.11,
        0.5,
    );
    (x + wx * amplitude, y + wy * amplitude)
}

#[inline]
pub(crate) fn directional_ripples(
    seed: u64,
    x: f64,
    y: f64,
    direction_x: f64,
    direction_y: f64,
    base_frequency: f64,
    warp_scale: f64,
) -> f64 {
    let along = x * direction_x + y * direction_y;
    let across = -x * direction_y + y * direction_x;
    let warp = fbm(
        seed ^ 0x7A11_4E01,
        along * warp_scale + 9.1,
        across * warp_scale - 4.7,
        3,
        2.0,
        0.55,
    ) * 1.8;

    let phase = along * base_frequency + warp;
    let triangle = 1.0 - ((phase.sin() * 0.5 + 0.5) - 0.5).abs() * 2.0;
    triangle.powf(1.7)
}
