//! The capability catalog: which actions are valid for a given set of inputs with the engines
//! that are actually installed. The executor (`exec.rs`) implements exactly these actions; the
//! QA harness enumerates this catalog against real fixtures so every listed option is exercised.

use crate::engines::{EngineId, Engines};
use crate::formats::{Category, Format};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "to", rename_all = "snake_case")]
pub enum Action {
    Convert(Format),
    // Images
    ImageCompress,
    ImageResize,
    ImageCrop,
    ImageRotate,
    StripMetadata,
    ImagesToPdf,
    GifFrames,
    ImageOcr(Format),
    // PDF
    PdfMerge,
    PdfSplit,
    PdfPages,
    PdfRotate,
    PdfCompress,
    PdfProtect,
    PdfUnlock,
    PdfOcr,
    // Audio / video
    Trim,
    MediaMerge,
    Normalize,
    VideoCompress,
    AudioCompress,
    VideoCrop,
    VideoResize,
    ExtractFrames,
    // Presentations
    SlidesToImages(Format),
    // Archives
    ArchiveCreate(Format),
    ArchiveExtract,
}

/// Which settings panel(s) the UI should show for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Param {
    ImageQuality,
    Resize,
    Crop,
    Rotate,
    Dpi,
    PageRange,
    PageOrder,
    PdfRotate,
    Password,
    OcrLanguage,
    Audio,
    Video,
    TrimRange,
    Fps,
    GifWidth,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionInfo {
    pub action: Action,
    pub label: String,
    /// "convert" for format conversions shown on the format grid, "tool" for the Tools section.
    pub group: &'static str,
    /// Category of the produced output, used to group formats in the picker.
    pub output_category: Option<Category>,
    pub recommended: bool,
    /// True when all inputs combine into one output (merge, images to PDF, create archive).
    pub combines_inputs: bool,
    /// Output is a folder of files (frames, split pages, extracted archive).
    pub produces_folder: bool,
    pub engines: Vec<EngineId>,
    pub params: Vec<Param>,
}

/// An input item as the catalog sees it.
#[derive(Debug, Clone, Copy)]
pub enum Input {
    File(Format),
    Folder,
}

const RASTER_IN: &[Format] = &[Format::Jpg, Format::Png, Format::Webp, Format::Heic, Format::Heif, Format::Tiff, Format::Bmp, Format::Gif, Format::Avif, Format::Ico, Format::Svg];
const IMAGE_OUT: &[Format] = &[Format::Jpg, Format::Png, Format::Webp, Format::Avif, Format::Heic, Format::Tiff, Format::Bmp, Format::Gif, Format::Ico, Format::Pdf];
const AUDIO: &[Format] = &[Format::Mp3, Format::Wav, Format::Aac, Format::M4a, Format::Flac, Format::Ogg, Format::Opus, Format::Aiff, Format::Wma];
const VIDEO: &[Format] = &[Format::Mp4, Format::Mov, Format::Mkv, Format::Avi, Format::Webm, Format::Wmv, Format::Mpeg];
const WORD_IN: &[Format] = &[Format::Doc, Format::Docx, Format::Odt, Format::Rtf, Format::Txt, Format::Html];
const WORD_OUT: &[Format] = &[Format::Pdf, Format::Docx, Format::Doc, Format::Odt, Format::Rtf, Format::Txt, Format::Html];
const PANDOC_IN: &[Format] = &[Format::Docx, Format::Odt, Format::Html, Format::Md, Format::Epub, Format::Rtf];
const PANDOC_OUT: &[Format] = &[Format::Docx, Format::Odt, Format::Html, Format::Md, Format::Epub, Format::Rtf, Format::Txt];
const EXCEL_IN: &[Format] = &[Format::Xls, Format::Xlsx, Format::Ods, Format::Csv];
const EXCEL_OUT: &[Format] = &[Format::Pdf, Format::Xlsx, Format::Xls, Format::Ods, Format::Csv];
const PPT_IN: &[Format] = &[Format::Ppt, Format::Pptx, Format::Odp];
const PPT_OUT: &[Format] = &[Format::Pdf, Format::Pptx, Format::Ppt, Format::Odp];
const ARCHIVE_IN: &[Format] = &[Format::Zip, Format::Rar, Format::SevenZ, Format::Tar, Format::TarGz, Format::Gz];
pub const ARCHIVE_OUT: &[Format] = &[Format::Zip, Format::SevenZ, Format::Tar, Format::TarGz];

/// ImageMagick's coder name for a format.
pub fn magick_coder(f: Format) -> &'static str {
    match f {
        Format::Jpg => "JPEG",
        Format::Png => "PNG",
        Format::Webp => "WEBP",
        Format::Heic | Format::Heif => "HEIC",
        Format::Tiff => "TIFF",
        Format::Bmp => "BMP",
        Format::Gif => "GIF",
        Format::Avif => "AVIF",
        Format::Svg => "SVG",
        Format::Ico => "ICO",
        Format::Pdf => "PDF",
        _ => "",
    }
}

/// How a document conversion is carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocRoute {
    Word,
    Excel,
    PowerPoint,
    Pandoc,
    /// Pandoc to DOCX, then Word for the final format (e.g. Markdown to PDF).
    PandocThenWord,
}

pub fn doc_route(src: Format, dst: Format, e: &Engines) -> Option<DocRoute> {
    if src == dst {
        return None;
    }
    if EXCEL_IN.contains(&src) {
        return (e.has(EngineId::Excel) && EXCEL_OUT.contains(&dst)).then_some(DocRoute::Excel);
    }
    if PPT_IN.contains(&src) {
        return (e.has(EngineId::Powerpoint) && PPT_OUT.contains(&dst)).then_some(DocRoute::PowerPoint);
    }
    // Pandoc keeps Markdown/EPUB structure better; prefer it when it can do the job alone
    // and the output is a structural format.
    let pandoc_ok = e.has(EngineId::Pandoc) && PANDOC_IN.contains(&src) && PANDOC_OUT.contains(&dst);
    let word_ok = e.has(EngineId::Word) && WORD_IN.contains(&src) && WORD_OUT.contains(&dst);
    if matches!(src, Format::Md | Format::Epub) || matches!(dst, Format::Md | Format::Epub) {
        if pandoc_ok {
            return Some(DocRoute::Pandoc);
        }
    }
    if word_ok {
        return Some(DocRoute::Word);
    }
    if pandoc_ok {
        return Some(DocRoute::Pandoc);
    }
    if e.has(EngineId::Pandoc) && e.has(EngineId::Word) && PANDOC_IN.contains(&src) && WORD_OUT.contains(&dst) {
        return Some(DocRoute::PandocThenWord);
    }
    None
}

fn info(action: Action, label: impl Into<String>, group: &'static str, out: Option<Category>, engines: &[EngineId], params: &[Param]) -> ActionInfo {
    let (combines_inputs, produces_folder) = match action {
        Action::ImagesToPdf | Action::PdfMerge | Action::MediaMerge | Action::ArchiveCreate(_) => (true, false),
        Action::GifFrames | Action::PdfSplit | Action::ExtractFrames | Action::ArchiveExtract | Action::SlidesToImages(_) => (false, true),
        _ => (false, false),
    };
    ActionInfo { action, label: label.into(), group, output_category: out, recommended: false, combines_inputs, produces_folder, engines: engines.to_vec(), params: params.to_vec() }
}

fn image_convert_params(to: Format) -> Vec<Param> {
    let mut p = vec![Param::Resize, Param::Rotate];
    if matches!(to, Format::Jpg | Format::Webp | Format::Avif | Format::Heic) {
        p.insert(0, Param::ImageQuality);
    }
    p
}

/// Every action available for a single input.
pub fn actions_for(input: Input, e: &Engines) -> Vec<ActionInfo> {
    let mut v = Vec::new();
    let f = match input {
        Input::Folder => {
            if e.has(EngineId::Sevenzip) {
                for &to in ARCHIVE_OUT {
                    v.push(info(Action::ArchiveCreate(to), format!("Compress to {}", to.label()), "tool", Some(Category::Archive), &[EngineId::Sevenzip], &[Param::Password]));
                }
            }
            return finalize(v, None);
        }
        Input::File(f) => f,
    };
    use EngineId::*;

    // ---- Images ----
    if RASTER_IN.contains(&f) && e.has(Magick) && e.magick_formats.can_read(magick_coder(f)) {
        for &to in IMAGE_OUT {
            if to == f || (f == Format::Heif && to == Format::Heic) || !e.magick_formats.can_write(magick_coder(to)) {
                continue;
            }
            let cat = if to == Format::Pdf { Category::Pdf } else { Category::Image };
            let mut a = info(Action::Convert(to), to.label(), "convert", Some(cat), &[Magick], &image_convert_params(to));
            if to == Format::Pdf {
                a.params = vec![];
            }
            v.push(a);
        }
        let heic_ro = matches!(f, Format::Heic | Format::Heif) && !e.magick_formats.can_write("HEIC");
        if matches!(f, Format::Jpg | Format::Png | Format::Webp | Format::Avif | Format::Heic | Format::Heif | Format::Tiff | Format::Gif) {
            v.push(info(Action::ImageCompress, if heic_ro { "Compress (saves as JPEG)" } else { "Compress" }, "tool", Some(Category::Image), &[Magick], &[Param::ImageQuality]));
        }
        if f != Format::Svg {
            v.push(info(Action::ImageResize, "Resize", "tool", Some(Category::Image), &[Magick], &[Param::Resize]));
            v.push(info(Action::ImageCrop, "Crop", "tool", Some(Category::Image), &[Magick], &[Param::Crop]));
            v.push(info(Action::ImageRotate, "Rotate / flip", "tool", Some(Category::Image), &[Magick], &[Param::Rotate]));
            v.push(info(Action::StripMetadata, "Remove metadata", "tool", Some(Category::Image), &[Magick], &[]));
        }
        if e.magick_formats.can_write("PDF") {
            v.push(info(Action::ImagesToPdf, "Combine into PDF", "tool", Some(Category::Pdf), &[Magick], &[]));
        }
        if f == Format::Gif {
            v.push(info(Action::GifFrames, "Extract frames", "tool", Some(Category::Image), &[Magick], &[]));
        }
        if e.has(Tesseract) && f != Format::Svg {
            v.push(info(Action::ImageOcr(Format::Txt), "OCR to text", "tool", Some(Category::Document), &[Magick, Tesseract], &[Param::OcrLanguage]));
            v.push(info(Action::ImageOcr(Format::Pdf), "OCR to searchable PDF", "tool", Some(Category::Pdf), &[Magick, Tesseract], &[Param::OcrLanguage]));
        }
    }

    // ---- Animated GIF as video source ----
    if f == Format::Gif && e.has(Ffmpeg) {
        for to in [Format::Mp4, Format::Webm] {
            v.push(info(Action::Convert(to), to.label(), "convert", Some(Category::Video), &[Ffmpeg], &[Param::Video]));
        }
    }

    // ---- PDF ----
    if f == Format::Pdf {
        if e.has(Pdftoppm) {
            v.push(info(Action::Convert(Format::Jpg), "JPEG (one per page)", "convert", Some(Category::Image), &[Pdftoppm], &[Param::Dpi, Param::PageRange]));
            v.push(info(Action::Convert(Format::Png), "PNG (one per page)", "convert", Some(Category::Image), &[Pdftoppm], &[Param::Dpi, Param::PageRange]));
            v.push(info(Action::Convert(Format::Tiff), "TIFF (one per page)", "convert", Some(Category::Image), &[Pdftoppm], &[Param::Dpi, Param::PageRange]));
            if e.has(Magick) && e.magick_formats.can_write("WEBP") {
                v.push(info(Action::Convert(Format::Webp), "WebP (one per page)", "convert", Some(Category::Image), &[Pdftoppm, Magick], &[Param::Dpi, Param::PageRange]));
            }
        }
        if e.has(Pdftotext) {
            v.push(info(Action::Convert(Format::Txt), "Plain Text", "convert", Some(Category::Document), &[Pdftotext], &[Param::PageRange]));
        }
        if e.has(Pdftohtml) {
            v.push(info(Action::Convert(Format::Html), "HTML", "convert", Some(Category::Document), &[Pdftohtml], &[]));
        }
        // PDF → DOCX is intentionally not offered: Word's PDF import blocks on a modal dialog
        // under hidden automation (verified), and no other installed engine converts PDF to DOCX.
        if e.has(Qpdf) {
            v.push(info(Action::PdfMerge, "Merge PDFs", "tool", Some(Category::Pdf), &[Qpdf], &[]));
            v.push(info(Action::PdfSplit, "Split into pages", "tool", Some(Category::Pdf), &[Qpdf], &[]));
            v.push(info(Action::PdfPages, "Extract / reorder pages", "tool", Some(Category::Pdf), &[Qpdf], &[Param::PageOrder]));
            v.push(info(Action::PdfRotate, "Rotate pages", "tool", Some(Category::Pdf), &[Qpdf], &[Param::PdfRotate]));
            v.push(info(Action::PdfCompress, "Optimize size (lossless)", "tool", Some(Category::Pdf), &[Qpdf], &[]));
            v.push(info(Action::PdfProtect, "Protect with password", "tool", Some(Category::Pdf), &[Qpdf], &[Param::Password]));
            v.push(info(Action::PdfUnlock, "Remove password", "tool", Some(Category::Pdf), &[Qpdf], &[Param::Password]));
        }
        if e.has(Pdftoppm) && e.has(Tesseract) && e.has(Qpdf) {
            v.push(info(Action::PdfOcr, "OCR to searchable PDF", "tool", Some(Category::Pdf), &[Pdftoppm, Tesseract, Qpdf], &[Param::OcrLanguage]));
        }
    }

    // ---- Audio ----
    if AUDIO.contains(&f) && e.has(Ffmpeg) {
        for &to in AUDIO {
            if to != f {
                v.push(info(Action::Convert(to), to.label(), "convert", Some(Category::Audio), &[Ffmpeg], &[Param::Audio, Param::TrimRange]));
            }
        }
        if matches!(f, Format::Mp3 | Format::Aac | Format::M4a | Format::Ogg | Format::Opus | Format::Wma) {
            v.push(info(Action::AudioCompress, "Compress", "tool", Some(Category::Audio), &[Ffmpeg], &[Param::Audio]));
        }
        v.push(info(Action::Trim, "Trim", "tool", Some(Category::Audio), &[Ffmpeg], &[Param::TrimRange]));
        v.push(info(Action::MediaMerge, "Merge", "tool", Some(Category::Audio), &[Ffmpeg], &[]));
        v.push(info(Action::Normalize, "Normalize loudness", "tool", Some(Category::Audio), &[Ffmpeg], &[]));
    }

    // ---- Video ----
    if VIDEO.contains(&f) && e.has(Ffmpeg) {
        for &to in VIDEO {
            if to != f {
                v.push(info(Action::Convert(to), to.label(), "convert", Some(Category::Video), &[Ffmpeg], &[Param::Video, Param::TrimRange]));
            }
        }
        v.push(info(Action::Convert(Format::Gif), "Animated GIF", "convert", Some(Category::Image), &[Ffmpeg], &[Param::GifWidth, Param::Fps, Param::TrimRange]));
        for &to in &[Format::Mp3, Format::M4a, Format::Wav, Format::Aac, Format::Flac, Format::Ogg, Format::Opus] {
            v.push(info(Action::Convert(to), format!("{} (audio only)", to.label()), "convert", Some(Category::Audio), &[Ffmpeg], &[Param::Audio, Param::TrimRange]));
        }
        v.push(info(Action::VideoCompress, "Compress", "tool", Some(Category::Video), &[Ffmpeg], &[Param::Video]));
        v.push(info(Action::Trim, "Trim", "tool", Some(Category::Video), &[Ffmpeg], &[Param::TrimRange]));
        v.push(info(Action::VideoCrop, "Crop", "tool", Some(Category::Video), &[Ffmpeg], &[Param::Crop]));
        v.push(info(Action::VideoResize, "Change resolution / FPS", "tool", Some(Category::Video), &[Ffmpeg], &[Param::Video, Param::Fps]));
        v.push(info(Action::ExtractFrames, "Extract frames", "tool", Some(Category::Image), &[Ffmpeg], &[Param::Fps]));
        v.push(info(Action::MediaMerge, "Merge", "tool", Some(Category::Video), &[Ffmpeg], &[]));
    }

    // ---- Documents / spreadsheets / presentations ----
    if matches!(f.category(), Category::Document | Category::Spreadsheet | Category::Presentation) {
        for &to in Format::ALL {
            if let Some(route) = doc_route(f, to, e) {
                let engines: Vec<EngineId> = match route {
                    DocRoute::Word => vec![Word],
                    DocRoute::Excel => vec![Excel],
                    DocRoute::PowerPoint => vec![Powerpoint],
                    DocRoute::Pandoc => vec![Pandoc],
                    DocRoute::PandocThenWord => vec![Pandoc, Word],
                };
                v.push(info(Action::Convert(to), to.label(), "convert", Some(to.category()), &engines, &[]));
            }
        }
        if PPT_IN.contains(&f) && e.has(Powerpoint) {
            v.push(info(Action::SlidesToImages(Format::Png), "Slides to PNG", "convert", Some(Category::Image), &[Powerpoint], &[]));
            v.push(info(Action::SlidesToImages(Format::Jpg), "Slides to JPEG", "convert", Some(Category::Image), &[Powerpoint], &[]));
        }
        // Text extraction for documents is the TXT conversion above; surface it as a tool as well.
        if let Some(a) = v.iter().find(|a| a.action == Action::Convert(Format::Txt)).cloned() {
            v.push(ActionInfo { group: "tool", label: "Extract text".into(), ..a });
        }
        if let Some(a) = v.iter().find(|a| a.action == Action::Convert(Format::Pdf)).cloned() {
            v.push(ActionInfo { group: "tool", label: "Export to PDF".into(), ..a });
        }
    }

    // ---- Archives ----
    if ARCHIVE_IN.contains(&f) && e.has(Sevenzip) {
        v.push(info(Action::ArchiveExtract, "Extract", "tool", None, &[Sevenzip], &[Param::Password]));
        if f != Format::Gz {
            for &to in ARCHIVE_OUT {
                if to != f {
                    v.push(info(Action::Convert(to), to.label(), "convert", Some(Category::Archive), &[Sevenzip], &[]));
                }
            }
        }
    }
    // Any file can be added to a new archive.
    if e.has(Sevenzip) {
        for &to in ARCHIVE_OUT {
            v.push(info(Action::ArchiveCreate(to), format!("Compress to {}", to.label()), "tool", Some(Category::Archive), &[Sevenzip], &[Param::Password]));
        }
    }

    finalize(v, Some(f))
}

fn finalize(mut v: Vec<ActionInfo>, src: Option<Format>) -> Vec<ActionInfo> {
    let rec = src.map(recommended_targets).unwrap_or_default();
    for a in &mut v {
        if let Action::Convert(to) = a.action {
            a.recommended = rec.contains(&to);
        }
    }
    // Remove duplicates (same action + group) keeping the first.
    let mut seen = std::collections::HashSet::new();
    v.retain(|a| seen.insert((a.action, a.group)));
    v
}

/// Smart recommendations, based on the detected source format.
pub fn recommended_targets(src: Format) -> Vec<Format> {
    use Format::*;
    match src {
        Heic | Heif | Avif => vec![Jpg, Png],
        Png => vec![Webp, Jpg],
        Jpg => vec![Png, Webp, Pdf],
        Webp => vec![Jpg, Png],
        Svg => vec![Png],
        Tiff | Bmp => vec![Jpg, Png],
        Gif => vec![Mp4, Png],
        Ico => vec![Png],
        Pdf => vec![Jpg, Txt],
        Mp4 | Mov | Mkv | Avi | Webm | Wmv | Mpeg => vec![Mp4, Mp3, Gif],
        Wav | Aiff | Flac => vec![Mp3, M4a],
        Mp3 => vec![Wav, M4a],
        M4a | Aac | Ogg | Opus | Wma => vec![Mp3],
        Doc | Docx | Odt | Rtf => vec![Pdf, Docx],
        Xls | Xlsx | Ods => vec![Pdf, Csv, Xlsx],
        Csv => vec![Xlsx, Pdf],
        Ppt | Pptx | Odp => vec![Pdf, Pptx],
        Md => vec![Pdf, Html, Docx],
        Html => vec![Pdf, Md],
        Txt => vec![Pdf],
        Epub => vec![Pdf, Docx],
        Rar | SevenZ | Tar | TarGz => vec![Zip],
        Zip => vec![SevenZ],
        Gz => vec![],
    }
}

/// Actions valid for every input in a selection. Mixed selections only offer what all items
/// support; combining actions need two or more inputs (except archive creation).
pub fn actions_for_selection(inputs: &[Input], e: &Engines) -> Vec<ActionInfo> {
    if inputs.is_empty() {
        return vec![];
    }
    let mut common = actions_for(inputs[0], e);
    for i in &inputs[1..] {
        let other = actions_for(*i, e);
        common.retain(|a| other.iter().any(|b| b.action == a.action && b.group == a.group));
        // Only recommend if recommended for all
        for a in &mut common {
            if let Some(b) = other.iter().find(|b| b.action == a.action) {
                a.recommended &= b.recommended;
            }
        }
    }
    let multi = inputs.len() > 1;
    common.retain(|a| match a.action {
        Action::PdfMerge | Action::MediaMerge => multi,
        _ => true,
    });
    if multi {
        // Merge requires one category: no merging audio with video.
        let cats: std::collections::HashSet<_> = inputs.iter().filter_map(|i| if let Input::File(f) = i { Some(f.category()) } else { None }).collect();
        if cats.len() > 1 {
            common.retain(|a| a.action != Action::MediaMerge);
        }
    }
    common
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_engines() -> Engines {
        Engines::fake(EngineId::ALL)
    }

    #[test]
    fn never_offers_converting_to_the_same_format() {
        let e = all_engines();
        for &f in Format::ALL {
            for a in actions_for(Input::File(f), &e) {
                if a.group == "convert" {
                    assert_ne!(a.action, Action::Convert(f), "{f:?} offered conversion to itself");
                }
            }
        }
    }

    #[test]
    fn missing_engines_hide_options() {
        let none = Engines::fake(&[]);
        for &f in Format::ALL {
            assert!(actions_for(Input::File(f), &none).is_empty(), "{f:?} has actions without engines");
        }
        let only_ffmpeg = Engines::fake(&[EngineId::Ffmpeg]);
        let acts = actions_for(Input::File(Format::Pdf), &only_ffmpeg);
        assert!(acts.is_empty());
        let acts = actions_for(Input::File(Format::Wav), &only_ffmpeg);
        assert!(acts.iter().any(|a| a.action == Action::Convert(Format::Mp3)));
    }

    #[test]
    fn smart_recommendations_match_spec_examples() {
        let e = all_engines();
        let rec = |f, to| actions_for(Input::File(f), &e).iter().any(|a| a.action == Action::Convert(to) && a.recommended);
        assert!(rec(Format::Heic, Format::Jpg));
        assert!(rec(Format::Heic, Format::Png));
        assert!(rec(Format::Png, Format::Webp));
        assert!(rec(Format::Pdf, Format::Jpg));
        // PDF → DOCX is deliberately unavailable (see actions_for).
        assert!(!actions_for(Input::File(Format::Pdf), &e).iter().any(|a| a.action == Action::Convert(Format::Docx)));
        assert!(rec(Format::Mp4, Format::Mp3));
        assert!(rec(Format::Wav, Format::Mp3));
        assert!(rec(Format::Docx, Format::Pdf));
        assert!(rec(Format::Xlsx, Format::Pdf));
    }

    #[test]
    fn mixed_selection_only_shows_common_actions() {
        let e = all_engines();
        let acts = actions_for_selection(&[Input::File(Format::Png), Input::File(Format::Mp3)], &e);
        assert!(acts.iter().all(|a| matches!(a.action, Action::ArchiveCreate(_))));
        let acts = actions_for_selection(&[Input::File(Format::Pdf), Input::File(Format::Pdf)], &e);
        assert!(acts.iter().any(|a| a.action == Action::PdfMerge));
        let acts = actions_for_selection(&[Input::File(Format::Pdf)], &e);
        assert!(!acts.iter().any(|a| a.action == Action::PdfMerge));
    }

    #[test]
    fn markdown_to_pdf_needs_word_or_is_hidden() {
        let pandoc_only = Engines::fake(&[EngineId::Pandoc]);
        assert_eq!(doc_route(Format::Md, Format::Pdf, &pandoc_only), None);
        let both = Engines::fake(&[EngineId::Pandoc, EngineId::Word]);
        assert_eq!(doc_route(Format::Md, Format::Pdf, &both), Some(DocRoute::PandocThenWord));
        assert_eq!(doc_route(Format::Md, Format::Docx, &both), Some(DocRoute::Pandoc));
        assert_eq!(doc_route(Format::Docx, Format::Pdf, &both), Some(DocRoute::Word));
    }
}
