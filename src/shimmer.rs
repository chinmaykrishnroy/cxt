pub fn apply_shimmer(text: &str, tick: u64) -> String {
    let wave_width = 15.0;
    let speed = 0.5;
    let base_r = 85.0;
    let base_g = 85.0;
    let base_b = 85.0;

    let peak_r = 255.0;
    let peak_g = 255.0;
    let peak_b = 255.0;

    let text_len = text.chars().count() as f64;

    let cycle_length = text_len + wave_width * 2.0;
    let wave_center = ((tick as f64 * speed) % cycle_length) - wave_width;

    let mut result = String::new();

    for (i, c) in text.chars().enumerate() {
        let i_f64 = i as f64;

        let distance = (i_f64 - wave_center).abs();

        let intensity = if distance < wave_width {
            let angle = (distance / wave_width) * std::f64::consts::PI;
            (angle.cos() + 1.0) / 2.0
        } else {
            0.0
        };

        let r = (base_r + (peak_r - base_r) * intensity) as u8;
        let g = (base_g + (peak_g - base_g) * intensity) as u8;
        let b = (base_b + (peak_b - base_b) * intensity) as u8;

        result.push_str(&format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, c));
    }

    result
}
