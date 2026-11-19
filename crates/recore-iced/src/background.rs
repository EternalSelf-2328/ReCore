//! The grade ReCore puts over a background picture, GIF or video: the
//! filter presets from Appearance and the brightness slider. The grade is
//! applied to the first frame too, so a still background and the first
//! frame of an animation look alike.

/// A background filter, as Appearance offers it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    None,
    Warm,
    Cool,
    Mono,
    Sepia,
    Scanlines,
}

impl Filter {
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Warm,
        Self::Cool,
        Self::Mono,
        Self::Sepia,
        Self::Scanlines,
    ];

    pub fn from_name(name: &str) -> Self {
        match name {
            "warm" => Self::Warm,
            "cool" => Self::Cool,
            "mono" => Self::Mono,
            "sepia" => Self::Sepia,
            "scanlines" => Self::Scanlines,
            _ => Self::None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Warm => "warm",
            Self::Cool => "cool",
            Self::Mono => "mono",
            Self::Sepia => "sepia",
            Self::Scanlines => "scanlines",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Warm => "Warm",
            Self::Cool => "Cool",
            Self::Mono => "Mono",
            Self::Sepia => "Sepia",
            Self::Scanlines => "Scanlines",
        }
    }
}

/// How a picture's channels are changed before it is drawn. The default is
/// the identity, so animated pictures elsewhere in the window are untouched.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    sepia: f32,
    red: f32,
    green: f32,
    blue: f32,
    scanline: f32,
}

impl Default for Grade {
    fn default() -> Self {
        Self {
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            sepia: 0.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            scanline: 0.0,
        }
    }
}

impl Grade {
    pub fn new(filter: Filter, brightness: f32) -> Self {
        let mut grade = Self {
            brightness: brightness.clamp(0.25, 2.0),
            ..Self::default()
        };
        match filter {
            Filter::None => {}
            Filter::Warm => {
                grade.saturation = 0.9;
                grade.sepia = 0.18;
                grade.red = 1.10;
                grade.blue = 0.90;
            }
            Filter::Cool => {
                grade.saturation = 0.9;
                grade.red = 0.88;
                grade.blue = 1.12;
            }
            Filter::Mono => {
                grade.saturation = 0.0;
                grade.contrast = 1.08;
            }
            Filter::Sepia => {
                grade.saturation = 0.55;
                grade.sepia = 0.82;
                grade.contrast = 1.04;
            }
            Filter::Scanlines => {
                grade.scanline = 0.18;
                grade.contrast = 1.03;
            }
        }
        grade
    }

    pub fn is_identity(self) -> bool {
        self == Self::default()
    }
}

/// Applies `grade` in place to straight RGBA pixels. `width` is the
/// picture's width, so the scanline filter can draw one line every third
/// row rather than every third pixel.
///
/// The work is fixed point: 256-entry tone tables and a 3x3 matrix, all
/// scaled by 4096, instead of a float chain for every channel of every
/// pixel. The filter, contrast, saturation, sepia and channel gains stay
/// as they were; only the last bit of rounding can differ.
pub fn apply(rgba: &mut [u8], width: u32, grade: Grade) {
    if grade.is_identity() {
        return;
    }
    const SHIFT: u32 = 12;
    const ONE: i32 = 1 << SHIFT;
    const HALF: i32 = ONE / 2;
    let tone = std::array::from_fn::<i32, 256, _>(|value| {
        let channel = value as f32 / 255.0;
        (((channel - 0.5) * grade.contrast + 0.5) * grade.brightness * ONE as f32).round() as i32
    });
    let matrix = |scan: f32| {
        let (s, p) = (grade.saturation, grade.sepia);
        let luma = [0.299, 0.587, 0.114];
        // Saturation first, then sepia, as one linear map.
        let sat = |row: usize, column: usize| {
            (if row == column { s } else { 0.0 }) + (1.0 - s) * luma[column]
        };
        let sepia = |row: usize, column: usize| {
            let paper = [
                [0.393, 0.769, 0.189],
                [0.349, 0.686, 0.168],
                [0.272, 0.534, 0.131],
            ];
            (if row == column { 1.0 - p } else { 0.0 }) + p * paper[row][column]
        };
        let gain = [grade.red * scan, grade.green * scan, grade.blue * scan];
        std::array::from_fn::<[i32; 3], 3, _>(|row| {
            std::array::from_fn(|column| {
                let mut sum = 0.0;
                for inner in 0..3 {
                    sum += sepia(row, inner) * sat(inner, column);
                }
                (sum * gain[row] * ONE as f32).round() as i32
            })
        })
    };
    let plain = matrix(1.0);
    let scanned = (grade.scanline > 0.0).then(|| matrix(1.0 - grade.scanline));
    let diagonal = grade.saturation == 1.0 && grade.sepia == 0.0;
    let width = width.max(1) as usize;
    let row_width = (width * 4).max(4);
    for (row, line) in rgba.chunks_mut(row_width).enumerate() {
        let m = match &scanned {
            Some(scanned) if row % 3 == 0 => scanned,
            _ => &plain,
        };
        for pixel in line.chunks_exact_mut(4) {
            let (red, green, blue) = (
                tone[pixel[0] as usize],
                tone[pixel[1] as usize],
                tone[pixel[2] as usize],
            );
            let (mut out_red, mut out_green, mut out_blue) = (0, 0, 0);
            if diagonal {
                out_red = (m[0][0] * red + HALF) >> SHIFT;
                out_green = (m[1][1] * green + HALF) >> SHIFT;
                out_blue = (m[2][2] * blue + HALF) >> SHIFT;
            } else {
                for (at, out) in [&mut out_red, &mut out_green, &mut out_blue]
                    .into_iter()
                    .enumerate()
                {
                    *out = (m[at][0] * red + m[at][1] * green + m[at][2] * blue + HALF) >> SHIFT;
                }
            }
            let ink = |value: i32| {
                let value = value.clamp(0, ONE);
                ((value * 255 + HALF) >> SHIFT) as u8
            };
            pixel[0] = ink(out_red);
            pixel[1] = ink(out_green);
            pixel[2] = ink(out_blue);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The float pipeline the fixed point one replaced.
    fn reference(rgba: &mut [u8], width: u32, grade: Grade) {
        let width = width.max(1) as usize;
        for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
            let mut red = f32::from(pixel[0]) / 255.0;
            let mut green = f32::from(pixel[1]) / 255.0;
            let mut blue = f32::from(pixel[2]) / 255.0;
            red = ((red - 0.5) * grade.contrast + 0.5) * grade.brightness;
            green = ((green - 0.5) * grade.contrast + 0.5) * grade.brightness;
            blue = ((blue - 0.5) * grade.contrast + 0.5) * grade.brightness;
            if grade.saturation != 1.0 {
                let luma = red * 0.299 + green * 0.587 + blue * 0.114;
                red = luma + (red - luma) * grade.saturation;
                green = luma + (green - luma) * grade.saturation;
                blue = luma + (blue - luma) * grade.saturation;
            }
            if grade.sepia > 0.0 {
                let (r, g, b) = (red, green, blue);
                red = r * (1.0 - grade.sepia) + (r * 0.393 + g * 0.769 + b * 0.189) * grade.sepia;
                green = g * (1.0 - grade.sepia) + (r * 0.349 + g * 0.686 + b * 0.168) * grade.sepia;
                blue = b * (1.0 - grade.sepia) + (r * 0.272 + g * 0.534 + b * 0.131) * grade.sepia;
            }
            let scan = if grade.scanline > 0.0 && (index / width) % 3 == 0 {
                1.0 - grade.scanline
            } else {
                1.0
            };
            red *= grade.red * scan;
            green *= grade.green * scan;
            blue *= grade.blue * scan;
            pixel[0] = (red.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[1] = (green.clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[2] = (blue.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    #[test]
    fn the_fixed_point_grade_keeps_the_float_ones_look() {
        const WIDTH: u32 = 41;
        let mut pixels = Vec::new();
        for red in (0..=255).step_by(17) {
            for green in (0..=255).step_by(17) {
                for blue in (0..=255).step_by(17) {
                    pixels.extend([red as u8, green as u8, blue as u8, 255]);
                }
            }
        }
        for filter in Filter::ALL {
            for brightness in [25, 50, 100, 150, 200] {
                let grade = Grade::new(filter, f32::from(brightness as u16) / 100.0);
                let (mut fixed, mut exact) = (pixels.clone(), pixels.clone());
                apply(&mut fixed, WIDTH, grade);
                reference(&mut exact, WIDTH, grade);
                let worst = fixed
                    .iter()
                    .zip(&exact)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap_or(0);
                assert!(worst <= 1, "{filter:?} at {brightness}% differs by {worst}");
            }
        }
    }
}
