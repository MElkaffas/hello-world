//! User-adjustable conversion settings. All fields are optional; engines fall back to sensible
//! defaults. Values are validated and clamped before they reach any command line.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Options {
    // Images
    /// 1-100
    pub quality: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// 1-1000 percent
    pub scale_pct: Option<u32>,
    /// 90, 180 or 270
    pub rotate: Option<u32>,
    pub flip_h: bool,
    pub flip_v: bool,
    pub crop: Option<Crop>,
    pub strip_metadata: bool,
    /// Rendering resolution for PDF pages / SVG, 36-1200
    pub dpi: Option<u32>,
    // PDF
    /// qpdf page range syntax, e.g. "1-3,5,z" or "3,1,2". Validated.
    pub pages: Option<String>,
    pub password: Option<String>,
    /// Tesseract language codes, e.g. "eng" or "eng+deu". Validated.
    pub ocr_lang: Option<String>,
    // Audio
    pub audio_bitrate_kbps: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    // Video
    /// "h264", "h265", "vp9", "av1", "mpeg4", "copy"
    pub video_codec: Option<String>,
    /// Output height in pixels, e.g. 720
    pub resolution: Option<u32>,
    pub fps: Option<f64>,
    /// Quality 0-51 (lower is better) for x264/x265; mapped for other codecs.
    pub crf: Option<u32>,
    pub gif_width: Option<u32>,
    // Trimming (seconds)
    pub start: Option<f64>,
    pub end: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(q) = self.quality {
            if !(1..=100).contains(&q) {
                return Err("Quality must be between 1 and 100".into());
            }
        }
        if let Some(s) = self.scale_pct {
            if !(1..=1000).contains(&s) {
                return Err("Scale must be between 1% and 1000%".into());
            }
        }
        for (n, v) in [("Width", self.width), ("Height", self.height), ("Resolution", self.resolution), ("GIF width", self.gif_width)] {
            if let Some(v) = v {
                if v == 0 || v > 16384 {
                    return Err(format!("{n} must be between 1 and 16384 pixels"));
                }
            }
        }
        if let Some(r) = self.rotate {
            if ![0, 90, 180, 270].contains(&r) {
                return Err("Rotation must be 90, 180 or 270 degrees".into());
            }
        }
        if let Some(c) = self.crop {
            if c.width == 0 || c.height == 0 {
                return Err("Crop width and height must be positive".into());
            }
        }
        if let Some(d) = self.dpi {
            if !(36..=1200).contains(&d) {
                return Err("DPI must be between 36 and 1200".into());
            }
        }
        if let Some(p) = &self.pages {
            validate_page_spec(p)?;
        }
        if let Some(l) = &self.ocr_lang {
            if l.is_empty() || l.len() > 64 || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '_') {
                return Err("OCR language must look like 'eng' or 'eng+deu'".into());
            }
        }
        if let Some(pw) = &self.password {
            if pw.is_empty() || pw.len() > 127 || pw.chars().any(|c| c.is_control()) {
                return Err("Password must be 1-127 printable characters".into());
            }
        }
        if let Some(b) = self.audio_bitrate_kbps {
            if !(8..=512).contains(&b) {
                return Err("Audio bitrate must be between 8 and 512 kbps".into());
            }
        }
        if let Some(sr) = self.sample_rate {
            if ![8000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000].contains(&sr) {
                return Err("Unsupported sample rate".into());
            }
        }
        if let Some(c) = self.channels {
            if !(1..=8).contains(&c) {
                return Err("Channels must be between 1 and 8".into());
            }
        }
        if let Some(c) = &self.video_codec {
            if !["h264", "h265", "vp9", "av1", "mpeg4", "copy"].contains(&c.as_str()) {
                return Err(format!("Unknown video codec '{c}'"));
            }
        }
        if let Some(f) = self.fps {
            if !(0.01..=240.0).contains(&f) {
                return Err("FPS must be between 0.01 and 240".into());
            }
        }
        if let Some(c) = self.crf {
            if c > 51 {
                return Err("Quality (CRF) must be between 0 and 51".into());
            }
        }
        match (self.start, self.end) {
            (Some(s), _) if s < 0.0 || !s.is_finite() => return Err("Start time must be positive".into()),
            (_, Some(e)) if e <= 0.0 || !e.is_finite() => return Err("End time must be positive".into()),
            (Some(s), Some(e)) if e <= s => return Err("End time must be after start time".into()),
            _ => {}
        }
        Ok(())
    }
}

/// Accepts qpdf-style page lists: numbers, ranges (a-b), "z" for last page, "r<n>" counted from
/// the end, separated by commas. Rejects anything else so it can be passed to qpdf safely.
pub fn validate_page_spec(spec: &str) -> Result<(), String> {
    let spec = spec.trim();
    if spec.is_empty() || spec.len() > 1000 {
        return Err("Page list is empty".into());
    }
    let page = |t: &str| -> bool {
        let t = t.trim();
        t == "z" || (t.starts_with('r') && t[1..].parse::<u32>().map_or(false, |n| n > 0)) || t.parse::<u32>().map_or(false, |n| n > 0)
    };
    for part in spec.split(',') {
        let part = part.trim();
        let ok = match part.split_once('-') {
            Some((a, b)) => page(a) && page(b),
            None => page(part),
        };
        if !ok {
            return Err(format!("Invalid page entry '{part}'. Use e.g. 1-3,5,z"));
        }
    }
    Ok(())
}

/// Expand a page spec into concrete 1-based page numbers given the page count.
pub fn expand_pages(spec: &str, count: u32) -> Result<Vec<u32>, String> {
    validate_page_spec(spec)?;
    let resolve = |t: &str| -> Result<u32, String> {
        let t = t.trim();
        let n = if t == "z" {
            count
        } else if let Some(r) = t.strip_prefix('r') {
            let r: u32 = r.parse().unwrap();
            count.checked_sub(r - 1).filter(|n| *n >= 1).ok_or_else(|| format!("Page {t} is out of range"))?
        } else {
            t.parse().unwrap()
        };
        if n == 0 || n > count {
            return Err(format!("Page {n} is out of range (document has {count} pages)"));
        }
        Ok(n)
    };
    let mut out = Vec::new();
    for part in spec.split(',') {
        match part.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (resolve(a)?, resolve(b)?);
                if a <= b { out.extend(a..=b) } else { out.extend((b..=a).rev()) }
            }
            None => out.push(resolve(part)?),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_specs() {
        assert!(validate_page_spec("1-3,5,z").is_ok());
        assert!(validate_page_spec("r1").is_ok());
        assert!(validate_page_spec("1;rm -rf").is_err());
        assert!(validate_page_spec("0").is_err());
        assert!(validate_page_spec("--foo").is_err());
        assert_eq!(expand_pages("3,1-2,z", 4).unwrap(), vec![3, 1, 2, 4]);
        assert_eq!(expand_pages("3-1", 4).unwrap(), vec![3, 2, 1]);
        assert!(expand_pages("9", 4).is_err());
    }

    #[test]
    fn option_validation() {
        assert!(Options { quality: Some(0), ..Default::default() }.validate().is_err());
        assert!(Options { start: Some(5.0), end: Some(2.0), ..Default::default() }.validate().is_err());
        assert!(Options { video_codec: Some("evil;".into()), ..Default::default() }.validate().is_err());
        assert!(Options { ocr_lang: Some("eng+deu".into()), ..Default::default() }.validate().is_ok());
        assert!(Options { ocr_lang: Some("../x".into()), ..Default::default() }.validate().is_err());
        assert!(Options::default().validate().is_ok());
    }
}
