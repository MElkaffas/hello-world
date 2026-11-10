//! Content-based file type detection. The file extension is only used to break ties between
//! formats that share a container (ASF audio vs video, plain text flavours).

use crate::formats::Format;
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub format: Option<Format>,
    /// How the format was established: "content", "content+extension", "extension", "unknown".
    pub method: &'static str,
    /// Set when the extension disagrees with the content (e.g. a PNG named .jpg).
    pub extension_mismatch: bool,
}

const HEAD: usize = 8192;

pub fn detect_path(path: &Path) -> std::io::Result<Detection> {
    let mut f = File::open(path)?;
    let mut buf = vec![0u8; HEAD];
    let n = read_up_to(&mut f, &mut buf)?;
    buf.truncate(n);
    let by_ext = path.file_name().and_then(|n| n.to_str()).and_then(Format::from_filename);
    let by_content = sniff(&buf, by_ext, Some(&mut f));
    Ok(finish(by_content, by_ext))
}

pub fn detect_bytes(buf: &[u8], name: Option<&str>) -> Detection {
    let by_ext = name.and_then(Format::from_filename);
    let by_content = sniff::<File>(&buf[..buf.len().min(HEAD)], by_ext, None);
    finish(by_content, by_ext)
}

fn finish(content: Sniff, by_ext: Option<Format>) -> Detection {
    match content {
        Sniff::Exact(f) => Detection {
            format: Some(f),
            method: "content",
            extension_mismatch: by_ext.map_or(false, |e| e != f && !equivalent(e, f)),
        },
        Sniff::Ambiguous(f) => Detection { format: Some(f), method: "content+extension", extension_mismatch: false },
        Sniff::Text => match by_ext {
            Some(e) if is_text_format(e) => Detection { format: Some(e), method: "content+extension", extension_mismatch: false },
            _ => Detection { format: Some(Format::Txt), method: "content", extension_mismatch: by_ext.is_some() },
        },
        Sniff::Unknown => Detection { format: None, method: "unknown", extension_mismatch: false },
    }
}

fn equivalent(a: Format, b: Format) -> bool {
    use Format::*;
    matches!((a, b), (Heic, Heif) | (Heif, Heic) | (Mp4, M4a) | (M4a, Mp4) | (Ogg, Opus) | (Mkv, Webm) | (Gz, TarGz) | (TarGz, Gz) | (Txt, Csv) | (Txt, Md))
}

fn is_text_format(f: Format) -> bool {
    matches!(f, Format::Txt | Format::Csv | Format::Md | Format::Html | Format::Svg | Format::Rtf)
}

enum Sniff {
    Exact(Format),
    Ambiguous(Format),
    Text,
    Unknown,
}

fn read_up_to<R: Read>(r: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        let n = r.read(&mut buf[total..])?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
}

fn starts(b: &[u8], sig: &[u8]) -> bool {
    b.len() >= sig.len() && &b[..sig.len()] == sig
}

fn at(b: &[u8], off: usize, sig: &[u8]) -> bool {
    b.len() >= off + sig.len() && &b[off..off + sig.len()] == sig
}

fn sniff<R: Read + Seek>(b: &[u8], by_ext: Option<Format>, file: Option<&mut R>) -> Sniff {
    use Format::*;
    if b.is_empty() {
        return Sniff::Unknown;
    }
    if starts(b, b"\xFF\xD8\xFF") { return Sniff::Exact(Jpg); }
    if starts(b, b"\x89PNG\r\n\x1a\n") { return Sniff::Exact(Png); }
    if starts(b, b"GIF87a") || starts(b, b"GIF89a") { return Sniff::Exact(Gif); }
    if starts(b, b"BM") && b.len() > 14 && at(b, 6, &[0, 0, 0, 0]) { return Sniff::Exact(Bmp); }
    if starts(b, b"II*\0") || starts(b, b"MM\0*") { return Sniff::Exact(Tiff); }
    if starts(b, &[0, 0, 1, 0]) && b.len() > 6 && b[4] > 0 { return Sniff::Exact(Ico); }
    if starts(b, b"%PDF-") { return Sniff::Exact(Pdf); }
    if starts(b, b"RIFF") && b.len() >= 12 {
        match &b[8..12] {
            b"WEBP" => return Sniff::Exact(Webp),
            b"WAVE" => return Sniff::Exact(Wav),
            b"AVI " => return Sniff::Exact(Avi),
            _ => {}
        }
    }
    if starts(b, b"FORM") && (at(b, 8, b"AIFF") || at(b, 8, b"AIFC")) { return Sniff::Exact(Aiff); }
    if starts(b, b"fLaC") { return Sniff::Exact(Flac); }
    if starts(b, b"OggS") {
        return if find(&b[..b.len().min(256)], b"OpusHead").is_some() { Sniff::Exact(Opus) } else { Sniff::Exact(Ogg) };
    }
    if at(b, 4, b"ftyp") && b.len() >= 12 {
        let brand = &b[8..12];
        let compat = &b[8..b.len().min(64)];
        return match brand {
            b"heic" | b"heix" | b"heim" | b"heis" | b"hevc" | b"hevx" => Sniff::Exact(if by_ext == Some(Heif) { Heif } else { Heic }),
            b"avif" | b"avis" => Sniff::Exact(Avif),
            b"mif1" | b"msf1" => {
                if find(compat, b"avif").is_some() { Sniff::Exact(Avif) } else { Sniff::Exact(if by_ext == Some(Heic) { Heic } else { Heif }) }
            }
            b"qt  " => Sniff::Exact(Mov),
            b"M4A " | b"M4B " => Sniff::Exact(M4a),
            _ => {
                if by_ext == Some(M4a) { Sniff::Ambiguous(M4a) } else if by_ext == Some(Mov) { Sniff::Ambiguous(Mov) } else { Sniff::Exact(Mp4) }
            }
        };
    }
    if starts(b, &[0x1A, 0x45, 0xDF, 0xA3]) {
        return if find(&b[..b.len().min(64)], b"webm").is_some() { Sniff::Exact(Webm) } else { Sniff::Exact(Mkv) };
    }
    if starts(b, &[0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11]) {
        // ASF container: look for a video stream type GUID (BC19EFC0-5B4D-11CF-A8FD-00805F5C442B).
        let video_guid = [0xC0, 0xEF, 0x19, 0xBC, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B];
        return if find(b, &video_guid).is_some() { Sniff::Exact(Wmv) } else { Sniff::Exact(Wma) };
    }
    if starts(b, &[0, 0, 1, 0xBA]) || starts(b, &[0, 0, 1, 0xB3]) { return Sniff::Exact(Mpeg); }
    if starts(b, b"ID3") {
        // ID3 tags can precede MP3 or AAC; trust .aac when given.
        return if by_ext == Some(Aac) { Sniff::Ambiguous(Aac) } else { Sniff::Exact(Mp3) };
    }
    if b.len() >= 2 && b[0] == 0xFF && (b[1] & 0xF6) == 0xF0 { return Sniff::Exact(Aac); }
    if b.len() >= 2 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0 { return Sniff::Exact(Mp3); }
    if starts(b, b"Rar!\x1A\x07") { return Sniff::Exact(Rar); }
    if starts(b, b"7z\xBC\xAF\x27\x1C") { return Sniff::Exact(SevenZ); }
    if starts(b, &[0x1F, 0x8B]) {
        return if by_ext == Some(TarGz) { Sniff::Exact(TarGz) } else { Sniff::Exact(Gz) };
    }
    if at(b, 257, b"ustar") { return Sniff::Exact(Tar); }
    if starts(b, b"PK\x03\x04") || starts(b, b"PK\x05\x06") {
        if let Some(f) = file {
            if let Some(kind) = zip_kind(f) {
                return Sniff::Exact(kind);
            }
        } else if let Some(kind) = zip_kind_from_head(b) {
            return Sniff::Exact(kind);
        }
        return Sniff::Exact(Zip);
    }
    if starts(b, &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        if let Some(f) = file {
            if let Some(kind) = ole_kind(f) {
                return Sniff::Exact(kind);
            }
        }
        return match by_ext {
            Some(e @ (Doc | Xls | Ppt)) => Sniff::Ambiguous(e),
            _ => Sniff::Unknown,
        };
    }
    if starts(b, b"{\\rtf") { return Sniff::Exact(Rtf); }

    // Text-based formats
    let text = strip_bom(b);
    if looks_like_text(text) {
        let head = String::from_utf8_lossy(&text[..text.len().min(2048)]).to_lowercase();
        let trimmed = head.trim_start();
        if trimmed.starts_with("<svg") || (trimmed.starts_with("<?xml") && head.contains("<svg")) || (trimmed.starts_with("<!doctype svg")) {
            return Sniff::Exact(Svg);
        }
        if trimmed.starts_with("<!doctype html") || trimmed.starts_with("<html") || (head.contains("<html") && head.contains("<body")) {
            return Sniff::Exact(Html);
        }
        return Sniff::Text;
    }
    Sniff::Unknown
}

fn strip_bom(b: &[u8]) -> &[u8] {
    if starts(b, &[0xEF, 0xBB, 0xBF]) { &b[3..] } else { b }
}

fn looks_like_text(b: &[u8]) -> bool {
    if b.is_empty() {
        return false;
    }
    // UTF-16 with BOM counts as text too.
    if starts(b, &[0xFF, 0xFE]) || starts(b, &[0xFE, 0xFF]) {
        return true;
    }
    // Allow a truncated multibyte sequence at the very end of the sample.
    let s = match std::str::from_utf8(b) {
        Ok(s) => s,
        Err(e) if e.valid_up_to() + 4 >= b.len() => std::str::from_utf8(&b[..e.valid_up_to()]).unwrap_or(""),
        Err(_) => {
            // Legacy single-byte encodings (Windows-1252, Latin-1): no NULs, few C0 controls.
            if b.contains(&0) {
                return false;
            }
            let ctrl = b.iter().filter(|&&c| c < 0x20 && !matches!(c, b'\n' | b'\r' | b'\t' | 0x0c)).count();
            return ctrl * 100 <= b.len();
        }
    };
    let ctrl = s.chars().filter(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t' | '\x0c')).count();
    ctrl * 100 <= s.chars().count().max(1)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn zip_kind<R: Read + Seek>(f: &mut R) -> Option<Format> {
    f.rewind().ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    if let Ok(mut m) = z.by_name("mimetype") {
        let mut s = String::new();
        m.read_to_string(&mut s).ok()?;
        return match s.trim() {
            "application/epub+zip" => Some(Format::Epub),
            "application/vnd.oasis.opendocument.text" => Some(Format::Odt),
            "application/vnd.oasis.opendocument.spreadsheet" => Some(Format::Ods),
            "application/vnd.oasis.opendocument.presentation" => Some(Format::Odp),
            _ => None,
        };
    }
    let has_ct = z.by_name("[Content_Types].xml").is_ok();
    if has_ct {
        let names: Vec<String> = z.file_names().map(|s| s.to_string()).collect();
        if names.iter().any(|n| n.starts_with("word/")) { return Some(Format::Docx); }
        if names.iter().any(|n| n.starts_with("xl/")) { return Some(Format::Xlsx); }
        if names.iter().any(|n| n.starts_with("ppt/")) { return Some(Format::Pptx); }
    }
    None
}

fn zip_kind_from_head(b: &[u8]) -> Option<Format> {
    if find(b, b"mimetypeapplication/epub+zip").is_some() { return Some(Format::Epub); }
    if find(b, b"application/vnd.oasis.opendocument.text").is_some() { return Some(Format::Odt); }
    if find(b, b"application/vnd.oasis.opendocument.spreadsheet").is_some() { return Some(Format::Ods); }
    if find(b, b"application/vnd.oasis.opendocument.presentation").is_some() { return Some(Format::Odp); }
    if find(b, b"[Content_Types].xml").is_some() {
        if find(b, b"word/").is_some() { return Some(Format::Docx); }
        if find(b, b"xl/").is_some() { return Some(Format::Xlsx); }
        if find(b, b"ppt/").is_some() { return Some(Format::Pptx); }
    }
    None
}

fn ole_kind<R: Read + Seek>(f: &mut R) -> Option<Format> {
    f.rewind().ok()?;
    let c = cfb::CompoundFile::open(f).ok()?;
    if c.exists("/WordDocument") { return Some(Format::Doc); }
    if c.exists("/Workbook") || c.exists("/Book") { return Some(Format::Xls); }
    if c.exists("/PowerPoint Document") { return Some(Format::Ppt); }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(b: &[u8], n: &str) -> Option<Format> {
        detect_bytes(b, Some(n)).format
    }

    #[test]
    fn magic_numbers() {
        assert_eq!(d(b"\xFF\xD8\xFF\xE0\0\x10JFIF", "x.jpg"), Some(Format::Jpg));
        assert_eq!(d(b"\x89PNG\r\n\x1a\n0000", "x.png"), Some(Format::Png));
        assert_eq!(d(b"%PDF-1.7\n", "x.pdf"), Some(Format::Pdf));
        assert_eq!(d(b"RIFF\0\0\0\0WEBPVP8 ", "x.webp"), Some(Format::Webp));
        assert_eq!(d(b"RIFF\0\0\0\0WAVEfmt ", "x.wav"), Some(Format::Wav));
        assert_eq!(d(b"\0\0\0\x18ftypheic\0\0\0\0mif1heic", "x.heic"), Some(Format::Heic));
        assert_eq!(d(b"\0\0\0\x1cftypavif\0\0\0\0avifmif1", "x.avif"), Some(Format::Avif));
        assert_eq!(d(b"\0\0\0\x14ftypqt  \0\0\0\0", "x.mov"), Some(Format::Mov));
        assert_eq!(d(b"\0\0\0\x18ftypisom\0\0\0\0", "x.mp4"), Some(Format::Mp4));
        assert_eq!(d(b"fLaC\0\0\0\x22", "x.flac"), Some(Format::Flac));
        assert_eq!(d(b"7z\xBC\xAF\x27\x1C\0\x04", "x.7z"), Some(Format::SevenZ));
        assert_eq!(d(b"Rar!\x1A\x07\x01\0", "x.rar"), Some(Format::Rar));
        assert_eq!(d(b"{\\rtf1\\ansi", "x.rtf"), Some(Format::Rtf));
    }

    #[test]
    fn content_wins_over_wrong_extension() {
        let det = detect_bytes(b"\x89PNG\r\n\x1a\n0000", Some("actually.jpg"));
        assert_eq!(det.format, Some(Format::Png));
        assert!(det.extension_mismatch);
    }

    #[test]
    fn text_flavours_use_extension() {
        assert_eq!(d(b"a,b,c\n1,2,3\n", "data.csv"), Some(Format::Csv));
        assert_eq!(d(b"# Title\n\nHello", "readme.md"), Some(Format::Md));
        assert_eq!(d(b"<!DOCTYPE html><html><body>x</body></html>", "p.txt"), Some(Format::Html));
        assert_eq!(d(b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"/>", "a.svg"), Some(Format::Svg));
        assert_eq!(d("Привет мир — ünïcödé".as_bytes(), "notes"), Some(Format::Txt));
        // Windows-1252 HTML (as written by older Word exports)
        assert_eq!(d(b"<html><body>caf\xe9 \x97 na\xefve</body></html>", "p.htm"), Some(Format::Html));
    }

    #[test]
    fn binary_garbage_is_unknown() {
        let junk: Vec<u8> = (0..512u32).map(|i| (i * 7 % 251) as u8).collect();
        assert_eq!(detect_bytes(&junk, Some("x.mp3")).format, None);
        assert_eq!(detect_bytes(b"", Some("x.png")).format, None);
    }
}
