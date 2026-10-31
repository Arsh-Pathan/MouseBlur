use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlurMode {
    #[serde(alias = "Smear", alias = "SmearRibbon", alias = "motionblur", alias = "blur")]
    Optical,
    #[serde(alias = "Directional", alias = "smearframe", alias = "extruded")]
    DirectionalSmear,
    #[serde(alias = "Ghosting", alias = "Ghost", alias = "strobe")]
    GhostShutter,
    #[serde(alias = "Hybrid", alias = "bloom")]
    NeonBloom,
    #[serde(alias = "Sparks")]
    ParticleSparks,
    #[serde(alias = "Chromatic", alias = "RgbSplit", alias = "prismatic")]
    ChromaticAberration,
    #[serde(alias = "Lines", alias = "actionlines")]
    SpeedLines,
    #[serde(alias = "Lightning", alias = "electric")]
    PlasmaArc,
}

impl Default for BlurMode {
    fn default() -> Self {
        BlurMode::Optical
    }
}

impl BlurMode {
    pub fn all() -> &'static [BlurMode] {
        &[
            BlurMode::Optical,
            BlurMode::DirectionalSmear,
            BlurMode::GhostShutter,
            BlurMode::NeonBloom,
            BlurMode::ParticleSparks,
            BlurMode::ChromaticAberration,
            BlurMode::SpeedLines,
            BlurMode::PlasmaArc,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlurMode::Optical => "Optical Motion Blur",
            BlurMode::DirectionalSmear => "Directional Smear",
            BlurMode::GhostShutter => "Ghost Shutter Strobe",
            BlurMode::NeonBloom => "Neon Bloom Laser",
            BlurMode::ParticleSparks => "Particle Sparks",
            BlurMode::ChromaticAberration => "Chromatic Aberration",
            BlurMode::SpeedLines => "Action Speed Lines",
            BlurMode::PlasmaArc => "Plasma Lightning Arc",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BlurMode::Optical => "Authentic optical cursor motion blur with realistic camera shutter exposure",
            BlurMode::DirectionalSmear => "Continuous extruded silhouette smear connecting instantaneous movement",
            BlurMode::GhostShutter => "Temporal strobe exposures simulating a high-speed camera shutter",
            BlurMode::NeonBloom => "High-energy laser cursor motion blur with soft ambient bloom",
            BlurMode::ParticleSparks => "Optical cursor blur accompanied by luminous kinetic flick sparks",
            BlurMode::ChromaticAberration => "Prismatic RGB color dispersion aligned with movement direction",
            BlurMode::SpeedLines => "Optical cursor blur with action speed-lines trailing behind fast flicks",
            BlurMode::PlasmaArc => "Optical cursor blur with electric plasma discharges along movement arc",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().replace([' ', '-', '_'], "").as_str() {
            "optical" | "motionblur" | "blur" | "default" | "smear" | "smearribbon" => Some(BlurMode::Optical),
            "directional" | "directionalsmear" | "smearframe" | "extruded" => Some(BlurMode::DirectionalSmear),
            "ghost" | "ghosting" | "ghostshutter" | "strobe" => Some(BlurMode::GhostShutter),
            "neon" | "neonbloom" | "bloom" | "hybrid" => Some(BlurMode::NeonBloom),
            "particle" | "particles" | "particlesparks" | "sparks" => Some(BlurMode::ParticleSparks),
            "chromatic" | "chromaticaberration" | "rgbsplit" | "rgb" | "prismatic" => Some(BlurMode::ChromaticAberration),
            "speedlines" | "lines" | "actionlines" => Some(BlurMode::SpeedLines),
            "plasma" | "plasmaarc" | "lightning" | "electric" => Some(BlurMode::PlasmaArc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorPreset {
    PureWhite,
    NeonCyan,
    ElectricPurple,
    SunsetOrange,
    MatrixGreen,
    Rainbow,
    Custom,
}

impl Default for ColorPreset {
    fn default() -> Self {
        ColorPreset::PureWhite
    }
}

impl ColorPreset {
    pub fn all() -> &'static [ColorPreset] {
        &[
            ColorPreset::NeonCyan,
            ColorPreset::PureWhite,
            ColorPreset::ElectricPurple,
            ColorPreset::SunsetOrange,
            ColorPreset::MatrixGreen,
            ColorPreset::Rainbow,
            ColorPreset::Custom,
        ]
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            ColorPreset::NeonCyan => "Cyber Neon Cyan",
            ColorPreset::PureWhite => "Pure White",
            ColorPreset::ElectricPurple => "Electric Purple",
            ColorPreset::SunsetOrange => "Sunset Orange",
            ColorPreset::MatrixGreen => "Matrix Green",
            ColorPreset::Rainbow => "Rainbow Wave",
            ColorPreset::Custom => "Custom Color",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub mode: BlurMode,
    pub color_preset: ColorPreset,
    pub custom_color_rgba: [f32; 4],
    pub tail_duration_ms: f32,
    pub max_trail_points: usize,
    pub min_velocity: f32,
    pub max_velocity: f32,
    pub head_width: f32,
    pub tail_width: f32,
    pub opacity: f32,
    pub target_fps: u32,
    pub pause_in_fullscreen: bool,
    pub show_tray_icon: bool,
    pub afterimage_count: usize,
    pub afterimage_spacing_ms: f32,
    // New mod settings
    pub particle_count: u32,
    pub glow_radius: f32,
    pub rgb_split_distance: f32,
    pub speed_lines_count: u32,
    pub plasma_jitter: f32,
    #[serde(default)]
    pub start_on_boot: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: BlurMode::Optical,
            color_preset: ColorPreset::PureWhite,
            custom_color_rgba: [1.0, 1.0, 1.0, 0.85],
            tail_duration_ms: 24.0, // 24ms shutter exposure duration for true optical motion blur
            max_trail_points: 32,
            min_velocity: 50.0,
            max_velocity: 2800.0,
            head_width: 1.0,
            tail_width: 0.2,
            opacity: 0.85,
            target_fps: 0,
            pause_in_fullscreen: false,
            show_tray_icon: true,
            afterimage_count: 5,
            afterimage_spacing_ms: 8.0,
            particle_count: 18,
            glow_radius: 6.0,
            rgb_split_distance: 6.0,
            speed_lines_count: 5,
            plasma_jitter: 8.0,
            start_on_boot: false,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut dir = PathBuf::from(appdata);
            dir.push("MouseBlur");
            if !dir.exists() {
                let _ = fs::create_dir_all(&dir);
            }
            dir.push("config.json");
            return dir;
        }
        PathBuf::from("mouseblur_config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<Config>(&content) {
                    // Normalize old long tail settings to authentic motion blur
                    if cfg.tail_duration_ms > 60.0 {
                        cfg.tail_duration_ms = 24.0;
                    }
                    cfg.start_on_boot = crate::startup::is_startup_enabled();
                    return cfg;
                }
            }
        }
        let mut default_cfg = Self::default();
        default_cfg.start_on_boot = crate::startup::is_startup_enabled();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }

    pub fn get_color_rgba(&self, rainbow_phase: f32) -> [f32; 4] {
        let base_alpha = self.opacity.clamp(0.0, 1.0);
        match self.color_preset {
            ColorPreset::NeonCyan => [0.0, 0.94, 1.0, base_alpha],
            ColorPreset::PureWhite => [1.0, 1.0, 1.0, base_alpha],
            ColorPreset::ElectricPurple => [0.75, 0.15, 1.0, base_alpha],
            ColorPreset::SunsetOrange => [1.0, 0.45, 0.05, base_alpha],
            ColorPreset::MatrixGreen => [0.1, 1.0, 0.25, base_alpha],
            ColorPreset::Rainbow => {
                let (r, g, b) = hsv_to_rgb(rainbow_phase % 360.0, 0.9, 1.0);
                [r, g, b, base_alpha]
            }
            ColorPreset::Custom => [
                self.custom_color_rgba[0].clamp(0.0, 1.0),
                self.custom_color_rgba[1].clamp(0.0, 1.0),
                self.custom_color_rgba[2].clamp(0.0, 1.0),
                (self.custom_color_rgba[3] * base_alpha).clamp(0.0, 1.0),
            ],
        }
    }
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r_prime, g_prime, b_prime) = if (0.0..60.0).contains(&h) {
        (c, x, 0.0)
    } else if (60.0..120.0).contains(&h) {
        (x, c, 0.0)
    } else if (120.0..180.0).contains(&h) {
        (0.0, c, x)
    } else if (180.0..240.0).contains(&h) {
        (0.0, x, c)
    } else if (240.0..300.0).contains(&h) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r_prime + m, g_prime + m, b_prime + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_and_serialization() {
        let default_cfg = Config::default();
        let serialized = serde_json::to_string(&default_cfg).expect("Serialization failed");
        let deserialized: Config = serde_json::from_str(&serialized).expect("Deserialization failed");

        assert_eq!(default_cfg.enabled, deserialized.enabled);
        assert_eq!(default_cfg.mode, deserialized.mode);
        assert_eq!(default_cfg.color_preset, deserialized.color_preset);
        assert_eq!(default_cfg.max_trail_points, deserialized.max_trail_points);
        assert_eq!(default_cfg.particle_count, deserialized.particle_count);
    }

    #[test]
    fn test_blur_mode_parsing() {
        assert_eq!(BlurMode::parse("optical"), Some(BlurMode::Optical));
        assert_eq!(BlurMode::parse("motionblur"), Some(BlurMode::Optical));
        assert_eq!(BlurMode::parse("smear"), Some(BlurMode::Optical));
        assert_eq!(BlurMode::parse("directional"), Some(BlurMode::DirectionalSmear));
        assert_eq!(BlurMode::parse("ghost"), Some(BlurMode::GhostShutter));
        assert_eq!(BlurMode::parse("neon"), Some(BlurMode::NeonBloom));
        assert_eq!(BlurMode::parse("sparks"), Some(BlurMode::ParticleSparks));
        assert_eq!(BlurMode::parse("chromatic"), Some(BlurMode::ChromaticAberration));
        assert_eq!(BlurMode::parse("speedlines"), Some(BlurMode::SpeedLines));
        assert_eq!(BlurMode::parse("plasma"), Some(BlurMode::PlasmaArc));
    }

    #[test]
    fn test_hsv_to_rgb() {
        let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((r - 1.0).abs() < 0.001);
        assert!(g.abs() < 0.001);
        assert!(b.abs() < 0.001);

        let (r, g, b) = hsv_to_rgb(120.0, 1.0, 1.0);
        assert!(r.abs() < 0.001);
        assert!((g - 1.0).abs() < 0.001);
        assert!(b.abs() < 0.001);
    }

    #[test]
    fn test_get_color_rgba() {
        let mut cfg = Config::default();
        cfg.color_preset = ColorPreset::PureWhite;
        cfg.opacity = 0.9;
        let rgba = cfg.get_color_rgba(0.0);
        assert_eq!(rgba, [1.0, 1.0, 1.0, 0.9]);
    }
}
