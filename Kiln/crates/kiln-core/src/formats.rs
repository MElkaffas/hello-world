//! Format catalog. Every format Kiln knows about, with its category and extensions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Image,
    Pdf,
    Audio,
    Video,
    Document,
    Spreadsheet,
    Presentation,
    Archive,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Image => "Image",
            Category::Pdf => "PDF",
            Category::Audio => "Audio",
            Category::Video => "Video",
            Category::Document => "Document",
            Category::Spreadsheet => "Spreadsheet",
            Category::Presentation => "Presentation",
            Category::Archive => "Archive",
        }
    }
}

macro_rules! formats {
    ($( $v:ident => $id:literal, $label:literal, $cat:ident, [$($ext:literal),+] ;)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum Format { $($v),+ }

        impl Format {
            pub const ALL: &'static [Format] = &[$(Format::$v),+];
            pub fn id(self) -> &'static str { match self { $(Format::$v => $id),+ } }
            pub fn label(self) -> &'static str { match self { $(Format::$v => $label),+ } }
            pub fn category(self) -> Category { match self { $(Format::$v => Category::$cat),+ } }
            /// Extensions, lowercase, without dot. The first one is used for output files.
            pub fn extensions(self) -> &'static [&'static str] { match self { $(Format::$v => &[$($ext),+]),+ } }
            pub fn from_id(s: &str) -> Option<Format> {
                let s = s.to_ascii_lowercase();
                match s.as_str() { $($id => Some(Format::$v),)+ _ => None }
            }
        }
    };
}

formats! {
    Jpg  => "jpg",  "JPEG",  Image, ["jpg", "jpeg", "jpe", "jfif"];
    Png  => "png",  "PNG",   Image, ["png"];
    Webp => "webp", "WebP",  Image, ["webp"];
    Heic => "heic", "HEIC",  Image, ["heic"];
    Heif => "heif", "HEIF",  Image, ["heif"];
    Tiff => "tiff", "TIFF",  Image, ["tiff", "tif"];
    Bmp  => "bmp",  "BMP",   Image, ["bmp", "dib"];
    Gif  => "gif",  "GIF",   Image, ["gif"];
    Avif => "avif", "AVIF",  Image, ["avif"];
    Svg  => "svg",  "SVG",   Image, ["svg"];
    Ico  => "ico",  "ICO",   Image, ["ico"];
    Pdf  => "pdf",  "PDF",   Pdf,   ["pdf"];
    Mp3  => "mp3",  "MP3",   Audio, ["mp3"];
    Wav  => "wav",  "WAV",   Audio, ["wav"];
    Aac  => "aac",  "AAC",   Audio, ["aac"];
    M4a  => "m4a",  "M4A",   Audio, ["m4a"];
    Flac => "flac", "FLAC",  Audio, ["flac"];
    Ogg  => "ogg",  "OGG Vorbis", Audio, ["ogg", "oga"];
    Opus => "opus", "Opus",  Audio, ["opus"];
    Aiff => "aiff", "AIFF",  Audio, ["aiff", "aif"];
    Wma  => "wma",  "WMA",   Audio, ["wma"];
    Mp4  => "mp4",  "MP4",   Video, ["mp4", "m4v"];
    Mov  => "mov",  "MOV",   Video, ["mov", "qt"];
    Mkv  => "mkv",  "MKV",   Video, ["mkv"];
    Avi  => "avi",  "AVI",   Video, ["avi"];
    Webm => "webm", "WebM",  Video, ["webm"];
    Wmv  => "wmv",  "WMV",   Video, ["wmv", "asf"];
    Mpeg => "mpeg", "MPEG",  Video, ["mpg", "mpeg", "mpe"];
    Doc  => "doc",  "Word 97-2003", Document, ["doc"];
    Docx => "docx", "Word",  Document, ["docx"];
    Odt  => "odt",  "OpenDocument Text", Document, ["odt"];
    Rtf  => "rtf",  "Rich Text", Document, ["rtf"];
    Txt  => "txt",  "Plain Text", Document, ["txt", "text", "log"];
    Html => "html", "HTML",  Document, ["html", "htm"];
    Md   => "md",   "Markdown", Document, ["md", "markdown"];
    Epub => "epub", "EPUB",  Document, ["epub"];
    Xls  => "xls",  "Excel 97-2003", Spreadsheet, ["xls"];
    Xlsx => "xlsx", "Excel", Spreadsheet, ["xlsx"];
    Ods  => "ods",  "OpenDocument Sheet", Spreadsheet, ["ods"];
    Csv  => "csv",  "CSV",   Spreadsheet, ["csv"];
    Ppt  => "ppt",  "PowerPoint 97-2003", Presentation, ["ppt"];
    Pptx => "pptx", "PowerPoint", Presentation, ["pptx"];
    Odp  => "odp",  "OpenDocument Slides", Presentation, ["odp"];
    Zip  => "zip",  "ZIP",   Archive, ["zip"];
    Rar  => "rar",  "RAR",   Archive, ["rar"];
    SevenZ => "7z", "7-Zip", Archive, ["7z"];
    Tar  => "tar",  "TAR",   Archive, ["tar"];
    TarGz => "tgz", "TAR.GZ", Archive, ["tar.gz", "tgz"];
    Gz   => "gz",   "GZip",  Archive, ["gz"];
}

impl Format {
    pub fn primary_ext(self) -> &'static str {
        self.extensions()[0]
    }

    /// Match by file name (handles double extensions like .tar.gz).
    pub fn from_filename(name: &str) -> Option<Format> {
        let lower = name.to_lowercase();
        // Longest extension match wins (tar.gz before gz).
        let mut best: Option<(usize, Format)> = None;
        for f in Format::ALL {
            for e in f.extensions() {
                let suffix = format!(".{e}");
                if lower.ends_with(&suffix) && best.map_or(true, |(l, _)| suffix.len() > l) {
                    best = Some((suffix.len(), *f));
                }
            }
        }
        best.map(|(_, f)| f)
    }

    pub fn supports_alpha(self) -> bool {
        matches!(self, Format::Png | Format::Webp | Format::Tiff | Format::Gif | Format::Avif | Format::Ico | Format::Heic | Format::Heif)
    }

    /// True for audio/video containers that ffmpeg decodes.
    pub fn is_media(self) -> bool {
        matches!(self.category(), Category::Audio | Category::Video) || self == Format::Gif
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip_and_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for f in Format::ALL {
            assert!(seen.insert(f.id()), "duplicate id {}", f.id());
            assert_eq!(Format::from_id(f.id()), Some(*f));
        }
    }

    #[test]
    fn filename_matching_prefers_longest_extension() {
        assert_eq!(Format::from_filename("a.TAR.GZ"), Some(Format::TarGz));
        assert_eq!(Format::from_filename("a.gz"), Some(Format::Gz));
        assert_eq!(Format::from_filename("Photo.JPEG"), Some(Format::Jpg));
        assert_eq!(Format::from_filename("noext"), None);
    }
}
