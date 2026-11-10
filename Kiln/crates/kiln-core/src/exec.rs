//! Job execution. Every action from the catalog is implemented here by invoking an approved
//! engine with an argument vector (never a shell). Outputs are produced in a private temp
//! folder, validated, and only then moved to their final, collision-safe location.

use crate::catalog::{actions_for_selection, doc_route, magick_coder, Action, ActionInfo, DocRoute, Input};
use crate::detect::detect_path;
use crate::engines::{EngineId, Engines};
use crate::formats::{Category, Format};
use crate::options::{expand_pages, Options};
use crate::output::{self, Conflict, OutputSettings};
use crate::process::{self, os, CancelToken, RunError};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub inputs: Vec<PathBuf>,
    pub action: Action,
    #[serde(default)]
    pub options: Options,
    #[serde(default)]
    pub output: OutputSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outcome {
    pub outputs: Vec<PathBuf>,
    /// True when nothing was written on purpose (e.g. compression would not reduce size).
    pub skipped: bool,
    pub note: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("Cancelled")]
    Cancelled,
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Engine(String),
    #[error("Output validation failed: {0}")]
    Validation(String),
    #[error("{0}")]
    Io(String),
}

impl From<RunError> for JobError {
    fn from(e: RunError) -> Self {
        match e {
            RunError::Cancelled => JobError::Cancelled,
            other => JobError::Engine(other.to_string()),
        }
    }
}

impl From<std::io::Error> for JobError {
    fn from(e: std::io::Error) -> Self {
        if e.raw_os_error() == Some(112) || e.raw_os_error() == Some(39) {
            return JobError::Io("Not enough disk space".into());
        }
        JobError::Io(e.to_string())
    }
}

type R<T> = Result<T, JobError>;

/// Office automation is serialized: PowerPoint is a single shared instance and running several
/// Office instances in parallel is slow and fragile.
static OFFICE_LOCK: Mutex<()> = Mutex::new(());

pub trait ProgressSink {
    fn progress(&mut self, fraction: Option<f64>, stage: &str);
}

impl<F: FnMut(Option<f64>, &str)> ProgressSink for F {
    fn progress(&mut self, fraction: Option<f64>, stage: &str) {
        self(fraction, stage)
    }
}

struct Ctx<'a> {
    e: &'a Engines,
    cancel: &'a CancelToken,
    sink: &'a mut dyn ProgressSink,
    tmp: PathBuf,
    opts: &'a Options,
}

impl Ctx<'_> {
    fn run(&mut self, id: EngineId, args: Vec<OsString>) -> R<String> {
        Ok(process::run(self.e, id, &args, Some(&self.tmp), self.cancel, |_| {})?)
    }
    fn stage(&mut self, f: Option<f64>, s: &str) {
        self.sink.progress(f, s);
    }
    fn check_cancel(&self) -> R<()> {
        if self.cancel.is_cancelled() { Err(JobError::Cancelled) } else { Ok(()) }
    }
}

/// Temp folder that removes itself.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> std::io::Result<TempDir> {
        let p = temp_root().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&p)?;
        Ok(TempDir(p))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A just-killed engine can keep the folder (its working directory) or a file open for a
        // moment; retry briefly instead of leaving it behind.
        for _ in 0..30 {
            match fs::remove_dir_all(&self.0) {
                Ok(()) => return,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
    }
}

pub fn temp_root() -> PathBuf {
    std::env::temp_dir().join("Kiln")
}

/// Remove leftovers of interrupted jobs (older than one hour).
pub fn cleanup_stale_temp() -> usize {
    let mut n = 0;
    if let Ok(rd) = fs::read_dir(temp_root()) {
        for e in rd.flatten() {
            let old = e.metadata().and_then(|m| m.modified()).map(|t| t.elapsed().map_or(false, |d| d.as_secs() > 3600)).unwrap_or(false);
            if old && fs::remove_dir_all(e.path()).is_ok() {
                n += 1;
            }
        }
    }
    n
}

/// What the temp stage produced, before finalization.
enum Produced {
    /// One file of the given format.
    File(PathBuf, Format),
    /// Many files of the given format in a folder (pages, frames).
    Many(PathBuf, Format),
    /// An arbitrary folder tree (archive extraction).
    Tree(PathBuf),
    Skipped(String),
}

pub fn execute(spec: &JobSpec, e: &Engines, cancel: &CancelToken, sink: &mut dyn ProgressSink) -> R<Outcome> {
    spec.options.validate().map_err(JobError::Invalid)?;
    if spec.inputs.is_empty() {
        return Err(JobError::Invalid("No input files".into()));
    }
    let mut inputs = Vec::new();
    let mut kinds = Vec::new();
    let mut formats = Vec::new();
    for p in &spec.inputs {
        let canon = output::validate_input(p).map_err(JobError::Invalid)?;
        if canon.is_dir() {
            kinds.push(Input::Folder);
            formats.push(None);
        } else {
            let det = detect_path(&canon).map_err(|err| JobError::Invalid(format!("Cannot read {}: {err}", canon.display())))?;
            let f = det.format.ok_or_else(|| JobError::Invalid(format!("{} is not a supported or recognizable file", file_name(&canon))))?;
            kinds.push(Input::File(f));
            formats.push(Some(f));
        }
        inputs.push(canon);
    }
    // Only actions the catalog offers for these exact inputs may run.
    let offered = refine_for_files(actions_for_selection(&kinds, e), &inputs, &formats, e);
    let info = offered.iter().find(|a| a.action == spec.action).ok_or_else(|| {
        JobError::Invalid(format!("{} is not available for {}", action_name(spec.action), describe_kinds(&kinds)))
    })?;
    if !info.combines_inputs && inputs.len() != 1 {
        return Err(JobError::Invalid("This action processes one file per job".into()));
    }

    let out_dir = match &spec.output.directory {
        Some(d) => output::validate_output_dir(d).map_err(JobError::Invalid)?,
        None => inputs[0].parent().map(Path::to_path_buf).ok_or_else(|| JobError::Invalid("Input has no parent folder".into()))?,
    };

    // Disk space: need roughly the input size again on both the temp and output volumes.
    let need: u64 = inputs.iter().map(|p| output::dir_size(p)).sum::<u64>() + 50 * 1024 * 1024;
    let tmp = TempDir::new()?;
    for vol in [&out_dir, &tmp.0] {
        if let Some(free) = output::free_space(vol) {
            if free < need {
                return Err(JobError::Io(format!("Not enough disk space on {}: about {} needed, {} free", vol.display(), output::human_size(need), output::human_size(free))));
            }
        }
    }

    let mut ctx = Ctx { e, cancel, sink, tmp: tmp.0.clone(), opts: &spec.options };
    ctx.stage(None, "Starting");
    let src = formats[0];
    let produced = produce(&mut ctx, spec.action, &inputs, src)?;
    ctx.check_cancel()?;
    ctx.stage(None, "Validating output");
    let outcome = finalize(&mut ctx, produced, spec, &inputs, &out_dir)?;
    ctx.stage(Some(1.0), "Done");
    Ok(outcome)
}

/// Actions for concrete files: starts from the format catalog, then inspects the files
/// themselves (media streams, PDF text layer) and removes options that cannot work.
pub fn actions_for_paths(paths: &[PathBuf], e: &Engines) -> Result<Vec<ActionInfo>, String> {
    let mut kinds = vec![];
    let mut formats = vec![];
    let mut canon = vec![];
    for p in paths {
        let c = output::validate_input(p)?;
        if c.is_dir() {
            kinds.push(Input::Folder);
            formats.push(None);
        } else {
            let f = detect_path(&c).map_err(|err| err.to_string())?.format.ok_or_else(|| format!("{} is not a supported file", file_name(&c)))?;
            kinds.push(Input::File(f));
            formats.push(Some(f));
        }
        canon.push(c);
    }
    Ok(refine_for_files(actions_for_selection(&kinds, e), &canon, &formats, e))
}

fn refine_for_files(mut acts: Vec<ActionInfo>, paths: &[PathBuf], formats: &[Option<Format>], e: &Engines) -> Vec<ActionInfo> {
    // Probing is only worth it for small selections; large batches keep the format-level list
    // and individual jobs report a clear error instead.
    if paths.len() > 20 {
        return acts;
    }
    for (p, f) in paths.iter().zip(formats) {
        let Some(f) = *f else { continue };
        if matches!(f.category(), Category::Video | Category::Audio) && e.has(EngineId::Ffprobe) {
            let pr = probe_streams(e, p);
            if !pr.has_audio {
                acts.retain(|a| a.output_category != Some(Category::Audio) && !matches!(a.action, Action::Normalize | Action::AudioCompress));
            }
            if !pr.has_video {
                acts.retain(|a| !matches!(a.output_category, Some(Category::Video | Category::Image)));
            }
        }
        if f == Format::Pdf && e.has(EngineId::Pdftotext) && p.to_string_lossy().is_ascii() {
            let out = process::run(e, EngineId::Pdftotext, &[os("-l"), os("3"), os("-enc"), os("UTF-8"), p.into(), os("-")], None, &CancelToken::default(), |_| {});
            if matches!(&out, Ok(t) if t.trim().is_empty()) {
                // Scanned PDF without a text layer: text extraction would be empty; OCR is the answer.
                acts.retain(|a| !matches!(a.action, Action::Convert(Format::Txt | Format::Html)));
                for a in &mut acts {
                    if a.action == Action::PdfOcr {
                        a.recommended = true;
                    }
                }
            }
        }
    }
    acts
}

fn describe_kinds(k: &[Input]) -> String {
    let names: Vec<String> = k.iter().map(|i| match i { Input::Folder => "folder".to_string(), Input::File(f) => f.label().to_string() }).collect();
    let mut uniq = names.clone();
    uniq.dedup();
    uniq.join(" + ")
}

pub fn action_name(a: Action) -> String {
    match a {
        Action::Convert(f) => format!("Convert to {}", f.label()),
        Action::ImageOcr(f) => format!("OCR to {}", f.label()),
        Action::SlidesToImages(f) => format!("Slides to {}", f.label()),
        Action::ArchiveCreate(f) => format!("Compress to {}", f.label()),
        other => {
            let s = format!("{other:?}");
            let mut out = String::new();
            for (i, ch) in s.chars().enumerate() {
                if ch.is_uppercase() && i > 0 {
                    out.push(' ');
                    out.extend(ch.to_lowercase());
                } else {
                    out.push(ch);
                }
            }
            out
        }
    }
}

/// Name suffix for tool outputs so they never look like a plain conversion.
fn suffix(a: Action) -> &'static str {
    match a {
        Action::ImageCompress | Action::AudioCompress | Action::VideoCompress | Action::PdfCompress => "compressed",
        Action::ImageResize | Action::VideoResize => "resized",
        Action::ImageCrop | Action::VideoCrop => "cropped",
        Action::ImageRotate | Action::PdfRotate => "rotated",
        Action::StripMetadata => "no metadata",
        Action::GifFrames | Action::ExtractFrames => "frames",
        Action::ImageOcr(_) | Action::PdfOcr => "OCR",
        Action::PdfMerge | Action::MediaMerge => "merged",
        Action::PdfSplit => "pages",
        Action::PdfPages => "pages",
        Action::PdfProtect => "protected",
        Action::PdfUnlock => "unlocked",
        Action::Trim => "trimmed",
        Action::Normalize => "normalized",
        Action::SlidesToImages(_) => "slides",
        _ => "",
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

/// Engines with narrow (ANSI) command lines can't open non-ASCII paths: give them an ASCII
/// alias inside the temp folder (hard link when possible, otherwise a copy).
fn ascii_alias(ctx: &Ctx, p: &Path, name: &str) -> R<PathBuf> {
    if p.to_string_lossy().is_ascii() {
        return Ok(p.to_path_buf());
    }
    let alias = ctx.tmp.join(name);
    if fs::hard_link(p, &alias).is_err() {
        fs::copy(p, &alias)?;
    }
    Ok(alias)
}

fn produce(ctx: &mut Ctx, action: Action, inputs: &[PathBuf], src: Option<Format>) -> R<Produced> {
    let input = &inputs[0];
    match action {
        Action::Convert(to) => {
            let src = src.ok_or_else(|| JobError::Invalid("Folders can only be archived".into()))?;
            convert(ctx, input, src, to)
        }
        Action::ImageCompress => {
            let f = src.unwrap();
            let to = tool_target(ctx.e, f);
            let mut o = ctx.opts.clone();
            o.quality.get_or_insert(if to == Format::Png { 95 } else { 72 });
            o.strip_metadata = true;
            let out = magick_one(ctx, input, f, to, &o)?;
            not_larger(out, input, to)
        }
        Action::ImageResize | Action::ImageCrop | Action::ImageRotate | Action::StripMetadata => {
            let f = src.unwrap();
            let o = ctx.opts;
            match action {
                Action::ImageResize if o.width.is_none() && o.height.is_none() && o.scale_pct.is_none() => return Err(JobError::Invalid("Enter a width, height or scale".into())),
                Action::ImageCrop if o.crop.is_none() => return Err(JobError::Invalid("Choose a crop area".into())),
                Action::ImageRotate if o.rotate.unwrap_or(0) == 0 && !o.flip_h && !o.flip_v => return Err(JobError::Invalid("Choose a rotation or flip".into())),
                _ => {}
            }
            let mut o = o.clone();
            if action == Action::StripMetadata {
                o = Options { strip_metadata: true, quality: Some(95), ..Default::default() };
            }
            let to = tool_target(ctx.e, f);
            Ok(Produced::File(magick_one(ctx, input, f, to, &o)?, to))
        }
        Action::ImagesToPdf => {
            let out = ctx.tmp.join("out.pdf");
            let mut args = vec![];
            for (i, p) in inputs.iter().enumerate() {
                let f = detect_path(p).ok().and_then(|d| d.format);
                if f == Some(Format::Svg) {
                    args.extend([os("-density"), os("150")]);
                }
                let _ = i;
                args.push(frame0(p, f));
            }
            args.extend([os("-auto-orient"), os("-background"), os("white"), os("-alpha"), os("remove")]);
            args.push(coder_path("PDF", &out));
            ctx.stage(None, "Building PDF");
            ctx.run(EngineId::Magick, args)?;
            Ok(Produced::File(out, Format::Pdf))
        }
        Action::GifFrames => {
            let dir = ctx.tmp.join("frames");
            fs::create_dir(&dir)?;
            ctx.run(EngineId::Magick, vec![input.into(), os("-coalesce"), coder_path("PNG", &dir.join("frame_%04d.png"))])?;
            Ok(Produced::Many(dir, Format::Png))
        }
        Action::ImageOcr(to) => {
            let png = ctx.tmp.join("ocr-src.png");
            let src_f = src.unwrap();
            ctx.stage(None, "Preparing image");
            ctx.run(EngineId::Magick, vec![frame0(input, Some(src_f)), os("-auto-orient"), os("-background"), os("white"), os("-alpha"), os("remove"), coder_path("PNG", &png)])?;
            let lang = ocr_lang(ctx)?;
            ctx.stage(None, "Recognizing text");
            let base = ctx.tmp.join("ocr-out");
            let mode = if to == Format::Pdf { "pdf" } else { "txt" };
            ctx.run(EngineId::Tesseract, vec![png.into(), base.clone().into(), os("-l"), os(lang), os(mode)])?;
            let out = base.with_extension(mode);
            if to == Format::Txt && fs::read_to_string(&out).map(|s| s.trim().is_empty()).unwrap_or(false) {
                return Err(JobError::Validation("No text was recognized in this image".into()));
            }
            Ok(Produced::File(out, to))
        }
        Action::PdfMerge => {
            let out = ctx.tmp.join("out.pdf");
            let mut args = vec![os("--empty"), os("--pages")];
            args.extend(inputs.iter().map(|p| p.into()));
            args.extend([os("--"), out.clone().into()]);
            ctx.stage(None, "Merging");
            ctx.run(EngineId::Qpdf, args)?;
            Ok(Produced::File(out, Format::Pdf))
        }
        Action::PdfSplit => {
            let dir = ctx.tmp.join("pages");
            fs::create_dir(&dir)?;
            ctx.stage(None, "Splitting");
            ctx.run(EngineId::Qpdf, vec![input.into(), os("--split-pages"), dir.join("page.pdf").into()])?;
            Ok(Produced::Many(dir, Format::Pdf))
        }
        Action::PdfPages => {
            let spec = ctx.opts.pages.clone().ok_or_else(|| JobError::Invalid("Enter the pages to keep, in order (e.g. 3,1-2)".into()))?;
            let out = ctx.tmp.join("out.pdf");
            ctx.run(EngineId::Qpdf, vec![os("--empty"), os("--pages"), input.into(), os(spec), os("--"), out.clone().into()])?;
            Ok(Produced::File(out, Format::Pdf))
        }
        Action::PdfRotate => {
            let angle = ctx.opts.rotate.filter(|a| *a != 0).ok_or_else(|| JobError::Invalid("Choose 90, 180 or 270 degrees".into()))?;
            let pages = ctx.opts.pages.clone().unwrap_or_else(|| "1-z".into());
            let out = ctx.tmp.join("out.pdf");
            ctx.run(EngineId::Qpdf, vec![input.into(), out.clone().into(), os(format!("--rotate=+{angle}:{pages}"))])?;
            Ok(Produced::File(out, Format::Pdf))
        }
        Action::PdfCompress => {
            let out = ctx.tmp.join("out.pdf");
            ctx.stage(None, "Optimizing");
            ctx.run(EngineId::Qpdf, vec![
                os("--object-streams=generate"), os("--compress-streams=y"), os("--recompress-flate"),
                os("--compression-level=9"), os("--remove-unreferenced-resources=yes"), input.into(), out.clone().into(),
            ])?;
            not_larger(out, input, Format::Pdf)
        }
        Action::PdfProtect | Action::PdfUnlock => {
            let pw = ctx.opts.password.clone().ok_or_else(|| JobError::Invalid("Enter a password".into()))?;
            let out = ctx.tmp.join("out.pdf");
            // Arguments go through an argument file so the password is not visible in the
            // process list.
            let argfile = ctx.tmp.join("args.txt");
            let lines = if action == Action::PdfProtect {
                vec![input.to_string_lossy().to_string(), out.to_string_lossy().to_string(), "--encrypt".into(), format!("--user-password={pw}"), format!("--owner-password={pw}"), "--bits=256".into(), "--".into()]
            } else {
                vec![format!("--password={pw}"), "--decrypt".into(), input.to_string_lossy().to_string(), out.to_string_lossy().to_string()]
            };
            fs::write(&argfile, lines.join("\n"))?;
            let r = ctx.run(EngineId::Qpdf, vec![OsString::from(format!("@{}", argfile.display()))]);
            let _ = fs::remove_file(&argfile);
            r?;
            Ok(Produced::File(out, Format::Pdf))
        }
        Action::PdfOcr => {
            let lang = ocr_lang(ctx)?;
            let pdf = ascii_alias(ctx, input, "src.pdf")?;
            let n = pdf_pages(ctx, &pdf)?;
            let dir = ctx.tmp.join("ocr");
            fs::create_dir(&dir)?;
            let mut list = String::new();
            for p in 1..=n {
                ctx.check_cancel()?;
                ctx.stage(Some((p - 1) as f64 / n as f64 * 0.5), &format!("Rendering page {p} of {n}"));
                let base = dir.join(format!("p{p:05}"));
                ctx.run(EngineId::Pdftoppm, vec![os("-r"), os("300"), os("-gray"), os("-png"), os("-singlefile"), os("-f"), os(p.to_string()), os("-l"), os(p.to_string()), pdf.clone().into(), base.clone().into()])?;
                list.push_str(&format!("{}\n", base.with_extension("png").display()));
            }
            let listf = ctx.tmp.join("pages.txt");
            fs::write(&listf, list)?;
            ctx.stage(Some(0.5), "Recognizing text");
            let outbase = ctx.tmp.join("ocr-out");
            ctx.run(EngineId::Tesseract, vec![listf.into(), outbase.clone().into(), os("-l"), os(lang), os("pdf")])?;
            Ok(Produced::File(outbase.with_extension("pdf"), Format::Pdf))
        }
        Action::Trim => {
            if ctx.opts.start.is_none() && ctx.opts.end.is_none() {
                return Err(JobError::Invalid("Enter a start and/or end time".into()));
            }
            let f = src.unwrap();
            ffmpeg_convert(ctx, input, f, f, &ctx.opts.clone(), Variant::Plain)
        }
        Action::AudioCompress => {
            let f = src.unwrap();
            let mut o = ctx.opts.clone();
            o.audio_bitrate_kbps.get_or_insert(96);
            let p = ffmpeg_convert(ctx, input, f, f, &o, Variant::Plain)?;
            match p { Produced::File(out, f) => not_larger(out, input, f), other => Ok(other) }
        }
        Action::VideoCompress => {
            let f = src.unwrap();
            let mut o = ctx.opts.clone();
            o.crf.get_or_insert(28);
            o.audio_bitrate_kbps.get_or_insert(128);
            let p = ffmpeg_convert(ctx, input, f, f, &o, Variant::Plain)?;
            match p { Produced::File(out, f) => not_larger(out, input, f), other => Ok(other) }
        }
        Action::VideoCrop => {
            if ctx.opts.crop.is_none() {
                return Err(JobError::Invalid("Choose a crop area".into()));
            }
            let f = src.unwrap();
            ffmpeg_convert(ctx, input, f, f, &ctx.opts.clone(), Variant::Plain)
        }
        Action::VideoResize => {
            if ctx.opts.resolution.is_none() && ctx.opts.fps.is_none() {
                return Err(JobError::Invalid("Choose a resolution or frame rate".into()));
            }
            let f = src.unwrap();
            ffmpeg_convert(ctx, input, f, f, &ctx.opts.clone(), Variant::Plain)
        }
        Action::Normalize => {
            let f = src.unwrap();
            ffmpeg_convert(ctx, input, f, f, &ctx.opts.clone(), Variant::Normalize)
        }
        Action::ExtractFrames => {
            let dir = ctx.tmp.join("frames");
            fs::create_dir(&dir)?;
            let fps = ctx.opts.fps.unwrap_or(1.0);
            let dur = media_duration(ctx, input);
            let args = vec![os("-hide_banner"), os("-nostdin"), os("-y"), os("-progress"), os("pipe:1"), os("-nostats"), os("-i"), input.into(), os("-vf"), os(format!("fps={fps}")), dir.join("frame_%05d.png").into()];
            run_ffmpeg(ctx, args, dur, "Extracting frames")?;
            Ok(Produced::Many(dir, Format::Png))
        }
        Action::MediaMerge => media_merge(ctx, inputs, src.unwrap()),
        Action::SlidesToImages(fmt) => {
            let dir = ctx.tmp.join("slides");
            office(ctx, EngineId::Powerpoint, input, &dir, fmt)?;
            // PowerPoint writes the images into the folder it is given.
            if !dir.is_dir() {
                return Err(JobError::Validation("PowerPoint did not export any slides".into()));
            }
            Ok(Produced::Many(dir, fmt))
        }
        Action::ArchiveCreate(fmt) => {
            let pw = ctx.opts.password.clone();
            archive_create(ctx, inputs, fmt, pw)
        }
        Action::ArchiveExtract => {
            let dir = ctx.tmp.join("extracted");
            fs::create_dir(&dir)?;
            extract_archive(ctx, input, src.unwrap(), &dir)?;
            Ok(Produced::Tree(dir))
        }
    }
}

/// Format an image tool writes: the source format when ImageMagick can encode it, otherwise JPEG
/// (e.g. HEIC, which is decode-only in the standard Windows ImageMagick build). SVG becomes PNG.
pub fn tool_target(e: &Engines, f: Format) -> Format {
    match f {
        Format::Svg => Format::Png,
        Format::Heif | Format::Heic if e.magick_formats.can_write("HEIC") => Format::Heic,
        other if e.magick_formats.can_write(magick_coder(other)) => other,
        _ => Format::Jpg,
    }
}

fn not_larger(out: PathBuf, input: &Path, f: Format) -> R<Produced> {
    let (a, b) = (fs::metadata(&out)?.len(), fs::metadata(input)?.len());
    if a >= b {
        return Ok(Produced::Skipped(format!("Already optimized: the result ({}) was not smaller than the original ({})", output::human_size(a), output::human_size(b))));
    }
    Ok(Produced::File(out, f))
}

fn ocr_lang(ctx: &mut Ctx) -> R<String> {
    let lang = ctx.opts.ocr_lang.clone().unwrap_or_else(|| "eng".into());
    let installed = tesseract_languages(ctx.e);
    for l in lang.split('+') {
        if !installed.iter().any(|x| x == l) {
            return Err(JobError::Invalid(format!("OCR language '{l}' is not installed. Installed: {}", installed.join(", "))));
        }
    }
    Ok(lang)
}

pub fn tesseract_languages(e: &Engines) -> Vec<String> {
    let Ok(out) = process::run(e, EngineId::Tesseract, &[os("--list-langs")], None, &CancelToken::default(), |_| {}) else { return vec![] };
    out.lines().skip_while(|l| !l.contains("List of available languages")).skip(1).map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "osd").collect()
}

fn coder_path(coder: &str, p: &Path) -> OsString {
    let mut s = OsString::from(format!("{coder}:"));
    s.push(p.as_os_str());
    s
}

fn frame0(p: &Path, f: Option<Format>) -> OsString {
    let mut s = p.as_os_str().to_os_string();
    if matches!(f, Some(Format::Gif | Format::Tiff | Format::Ico | Format::Heic | Format::Heif | Format::Webp | Format::Avif)) {
        s.push("[0]");
    }
    s
}

fn largest_frame(ctx: &mut Ctx, p: &Path) -> usize {
    let out = ctx.run(EngineId::Magick, vec![os("identify"), os("-format"), os("%w %h\n"), p.into()]).unwrap_or_default();
    out.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let mut it = l.split_whitespace().map(|v| v.parse::<u64>().ok());
            Some((i, it.next()?? * it.next()??))
        })
        .max_by_key(|(_, a)| *a)
        .map_or(0, |(i, _)| i)
}

/// One ImageMagick conversion with the image options applied.
fn magick_one(ctx: &mut Ctx, input: &Path, src: Format, to: Format, o: &Options) -> R<PathBuf> {
    let out = ctx.tmp.join(format!("out.{}", to.primary_ext()));
    let mut args: Vec<OsString> = vec![];
    if src == Format::Svg {
        args.extend([os("-density"), os(o.dpi.unwrap_or(96).to_string()), os("-background"), os("none")]);
    }
    let keep_frames = matches!((src, to), (Format::Gif, Format::Gif) | (Format::Tiff, Format::Tiff | Format::Pdf) | (Format::Gif, Format::Webp));
    if keep_frames {
        args.push(input.into());
        if src == Format::Gif {
            args.push(os("-coalesce"));
        }
    } else if src == Format::Ico {
        // Use the largest size stored in the icon.
        let idx = largest_frame(ctx, input);
        let mut s = input.as_os_str().to_os_string();
        s.push(format!("[{idx}]"));
        args.push(s);
    } else {
        args.push(frame0(input, Some(src)));
    }
    args.push(os("-auto-orient"));
    if let Some(c) = o.crop {
        args.extend([os("-crop"), os(format!("{}x{}+{}+{}", c.width, c.height, c.x, c.y)), os("+repage")]);
    }
    if let Some(p) = o.scale_pct {
        args.extend([os("-resize"), os(format!("{p}%"))]);
    } else if o.width.is_some() || o.height.is_some() {
        let g = match (o.width, o.height) {
            (Some(w), Some(h)) => format!("{w}x{h}"),
            (Some(w), None) => format!("{w}x"),
            (None, Some(h)) => format!("x{h}"),
            _ => unreachable!(),
        };
        args.extend([os("-resize"), os(g)]);
    }
    if let Some(r) = o.rotate.filter(|r| *r != 0) {
        args.extend([os("-rotate"), os(r.to_string())]);
    }
    if o.flip_h {
        args.push(os("-flop"));
    }
    if o.flip_v {
        args.push(os("-flip"));
    }
    if o.strip_metadata {
        args.push(os("-strip"));
    }
    if !to.supports_alpha() {
        args.extend([os("-background"), os("white"), os("-alpha"), os("remove"), os("-alpha"), os("off")]);
    }
    if let Some(q) = o.quality {
        args.extend([os("-quality"), os(q.to_string())]);
    }
    match to {
        Format::Ico => {
            args.extend([os("-resize"), os("256x256>"), os("-define"), os("icon:auto-resize=256,128,64,48,32,16")]);
        }
        Format::Png => args.extend([os("-define"), os("png:compression-level=9")]),
        _ => {}
    }
    if keep_frames && to == Format::Gif {
        args.extend([os("-layers"), os("optimize")]);
    }
    args.push(coder_path(magick_coder(to), &out));
    ctx.stage(None, &format!("Converting to {}", to.label()));
    ctx.run(EngineId::Magick, args)?;
    Ok(out)
}

fn convert(ctx: &mut Ctx, input: &Path, src: Format, to: Format) -> R<Produced> {
    use Category::*;
    // Archive re-packing
    if src.category() == Archive {
        let dir = ctx.tmp.join("x");
        fs::create_dir(&dir)?;
        extract_archive(ctx, input, src, &dir)?;
        return archive_from_dir(ctx, &dir, to);
    }
    if src == Format::Pdf {
        return pdf_convert(ctx, input, to);
    }
    if matches!(src.category(), Document | Spreadsheet | Presentation) {
        let route = doc_route(src, to, ctx.e).ok_or_else(|| JobError::Invalid(format!("No installed engine converts {} to {}", src.label(), to.label())))?;
        let out = ctx.tmp.join(format!("out.{}", to.primary_ext()));
        match route {
            DocRoute::Word => office(ctx, EngineId::Word, input, &out, to)?,
            DocRoute::Excel => office(ctx, EngineId::Excel, input, &out, to)?,
            DocRoute::PowerPoint => office(ctx, EngineId::Powerpoint, input, &out, to)?,
            DocRoute::Pandoc => pandoc(ctx, input, src, &out, to)?,
            DocRoute::PandocThenWord => {
                let mid = ctx.tmp.join("stage.docx");
                pandoc(ctx, input, src, &mid, Format::Docx)?;
                office(ctx, EngineId::Word, &mid, &out, to)?;
            }
        }
        return Ok(Produced::File(out, to));
    }
    if src.is_media() && (to.category() == Audio || to.category() == Video || (to == Format::Gif && src != Format::Gif)) {
        return ffmpeg_convert(ctx, input, src, to, &ctx.opts.clone(), Variant::Plain);
    }
    if src.category() == Image {
        let out = magick_one(ctx, input, src, to, &ctx.opts.clone())?;
        return Ok(Produced::File(out, to));
    }
    Err(JobError::Invalid(format!("Converting {} to {} is not supported", src.label(), to.label())))
}

fn pdf_pages(ctx: &mut Ctx, pdf: &Path) -> R<u32> {
    if ctx.e.has(EngineId::Qpdf) {
        let out = ctx.run(EngineId::Qpdf, vec![os("--show-npages"), pdf.into()])?;
        if let Ok(n) = out.trim().parse() {
            return Ok(n);
        }
    }
    let out = ctx.run(EngineId::Pdfinfo, vec![pdf.into()])?;
    out.lines()
        .find_map(|l| l.strip_prefix("Pages:").and_then(|v| v.trim().parse().ok()))
        .ok_or_else(|| JobError::Engine("Could not read the page count".into()))
}

fn pdf_convert(ctx: &mut Ctx, input: &Path, to: Format) -> R<Produced> {
    let pdf = ascii_alias(ctx, input, "src.pdf")?;
    match to {
        Format::Jpg | Format::Png | Format::Tiff | Format::Webp => {
            let n = pdf_pages(ctx, &pdf)?;
            let pages: Vec<u32> = match &ctx.opts.pages {
                Some(s) => expand_pages(s, n).map_err(JobError::Invalid)?,
                None => (1..=n).collect(),
            };
            let dpi = ctx.opts.dpi.unwrap_or(150).to_string();
            let dir = ctx.tmp.join("pages");
            fs::create_dir(&dir)?;
            let flag = match to { Format::Jpg => "-jpeg", Format::Tiff => "-tiff", _ => "-png" };
            let total = pages.len();
            for (i, p) in pages.iter().enumerate() {
                ctx.check_cancel()?;
                ctx.stage(Some(i as f64 / total as f64), &format!("Rendering page {p} of {n}"));
                let base = dir.join(format!("page-{p:03}"));
                let mut args = vec![os("-r"), os(dpi.clone()), os(flag), os("-singlefile"), os("-f"), os(p.to_string()), os("-l"), os(p.to_string())];
                if to == Format::Jpg {
                    args.extend([os("-jpegopt"), os(format!("quality={}", ctx.opts.quality.unwrap_or(90)))]);
                }
                args.extend([pdf.clone().into(), base.clone().into()]);
                ctx.run(EngineId::Pdftoppm, args)?;
                if to == Format::Webp {
                    let png = base.with_extension("png");
                    let webp = base.with_extension("webp");
                    ctx.run(EngineId::Magick, vec![png.clone().into(), os("-quality"), os(ctx.opts.quality.unwrap_or(85).to_string()), coder_path("WEBP", &webp)])?;
                    fs::remove_file(png)?;
                }
            }
            Ok(Produced::Many(dir, to))
        }
        Format::Txt => {
            let out = ctx.tmp.join("out.txt");
            match ctx.opts.pages.clone() {
                None => {
                    ctx.run(EngineId::Pdftotext, vec![os("-enc"), os("UTF-8"), os("-layout"), pdf.into(), out.clone().into()])?;
                }
                Some(spec) => {
                    let n = pdf_pages(ctx, &pdf)?;
                    let mut text = String::new();
                    for p in expand_pages(&spec, n).map_err(JobError::Invalid)? {
                        let part = ctx.tmp.join(format!("p{p}.txt"));
                        ctx.run(EngineId::Pdftotext, vec![os("-enc"), os("UTF-8"), os("-layout"), os("-f"), os(p.to_string()), os("-l"), os(p.to_string()), pdf.clone().into(), part.clone().into()])?;
                        text.push_str(&fs::read_to_string(&part).unwrap_or_default());
                    }
                    fs::write(&out, text)?;
                }
            }
            if fs::metadata(&out).map(|m| m.len()).unwrap_or(0) == 0 || fs::read_to_string(&out).map(|s| s.trim().is_empty()).unwrap_or(true) {
                return Err(JobError::Validation("The PDF contains no extractable text. It may be a scan; use OCR instead.".into()));
            }
            Ok(Produced::File(out, Format::Txt))
        }
        Format::Html => {
            let dir = ctx.tmp.join("html");
            fs::create_dir(&dir)?;
            ctx.run(EngineId::Pdftohtml, vec![os("-s"), os("-noframes"), os("-q"), os("-enc"), os("UTF-8"), pdf.into(), dir.join("out").into()])?;
            let files: Vec<PathBuf> = fs::read_dir(&dir)?.flatten().map(|e| e.path()).collect();
            if files.len() == 1 {
                let html = files.into_iter().next().unwrap();
                let out = ctx.tmp.join("out.html");
                fs::rename(&html, &out)?;
                return Ok(Produced::File(out, Format::Html));
            }
            Ok(Produced::Tree(dir))
        }
        _ => Err(JobError::Invalid(format!("PDF to {} is not supported", to.label()))),
    }
}

fn office(ctx: &mut Ctx, app: EngineId, input: &Path, out: &Path, to: Format) -> R<()> {
    let script = ctx.e.office_script.clone().ok_or_else(|| JobError::Engine("Office automation script missing".into()))?;
    let app_name = match app { EngineId::Word => "word", EngineId::Excel => "excel", _ => "powerpoint" };
    ctx.stage(None, &format!("Converting with {}", app.label()));
    let _guard = OFFICE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    ctx.check_cancel()?;
    let fmt = to.id();
    let args = vec![
        os("-NoProfile"), os("-NonInteractive"), os("-ExecutionPolicy"), os("Bypass"), os("-File"), script.into(),
        os("-App"), os(app_name), os("-InFile"), input.into(), os("-OutFile"), out.into(), os("-Format"), os(fmt),
    ];
    // Watchdog: Office can block on hidden dialogs. The run is cancelled on user cancel or after
    // OFFICE_TIMEOUT, and the Office instance this job started (PID reported by the script) is
    // stopped. Office windows the user opened are never touched.
    let inner = CancelToken::default();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let timed_out = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let (outer, inner, done, timed_out) = (ctx.cancel.clone(), inner.clone(), done.clone(), timed_out.clone());
        std::thread::spawn(move || {
            let start = std::time::Instant::now();
            while !done.load(std::sync::atomic::Ordering::SeqCst) {
                if outer.is_cancelled() {
                    inner.cancel();
                    return;
                }
                if start.elapsed() > OFFICE_TIMEOUT {
                    timed_out.store(true, std::sync::atomic::Ordering::SeqCst);
                    inner.cancel();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        });
    }
    let mut pid: Option<u32> = None;
    let r = process::run(ctx.e, app, &args, Some(&ctx.tmp), &inner, |line| {
        if let Some(p) = line.strip_prefix("KILN_PID=") {
            pid = p.trim().parse().ok();
        }
    });
    done.store(true, std::sync::atomic::Ordering::SeqCst);
    if r.is_err() {
        if let Some(p) = pid {
            kill_pid(p);
        }
    }
    match r {
        Err(RunError::Cancelled) if timed_out.load(std::sync::atomic::Ordering::SeqCst) => {
            Err(JobError::Engine(format!("{} did not finish within {} minutes and was stopped. The document may show a dialog that needs Office itself.", app.label(), OFFICE_TIMEOUT.as_secs() / 60)))
        }
        other => Ok(other.map(|_| ())?),
    }
}

const OFFICE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

fn kill_pid(pid: u32) {
    let taskkill = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows")).join(r"System32\taskkill.exe");
    let _ = crate::engines::hidden_command(&taskkill).args(["/PID", &pid.to_string(), "/F"]).output();
}

fn pandoc(ctx: &mut Ctx, input: &Path, src: Format, out: &Path, to: Format) -> R<()> {
    let from = match src {
        Format::Docx => "docx", Format::Odt => "odt", Format::Html => "html", Format::Md => "markdown",
        Format::Epub => "epub", Format::Rtf => "rtf", _ => return Err(JobError::Invalid("Pandoc cannot read this format".into())),
    };
    let target = match to {
        Format::Docx => "docx", Format::Odt => "odt", Format::Html => "html", Format::Md => "gfm",
        Format::Epub => "epub", Format::Rtf => "rtf", Format::Txt => "plain", _ => return Err(JobError::Invalid("Pandoc cannot write this format".into())),
    };
    let title = output::file_stem(input);
    let mut args = vec![os("--from"), os(from), os("--to"), os(target), os("--standalone"), os("--metadata"), os(format!("title={title}"))];
    if to == Format::Html {
        args.push(os("--embed-resources"));
    }
    if matches!(to, Format::Md | Format::Html | Format::Txt) && matches!(src, Format::Docx | Format::Odt | Format::Epub) {
        // Keep extracted images next to the temp output so --embed-resources can inline them.
        args.extend([os("--extract-media"), ctx.tmp.join("media").into()]);
    }
    args.extend([os("--output"), out.into(), input.into()]);
    ctx.stage(None, "Converting with Pandoc");
    // Pandoc resolves relative resources (images) from the document's folder.
    let cwd = input.parent().map(Path::to_path_buf);
    process::run(ctx.e, EngineId::Pandoc, &args, cwd.as_deref(), ctx.cancel, |_| {})?;
    Ok(())
}

// ---------------- FFmpeg ----------------

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Plain,
    Normalize,
}

#[derive(Default)]
struct Probe {
    duration: Option<f64>,
    has_video: bool,
    has_audio: bool,
    width: Option<u32>,
    height: Option<u32>,
}

fn probe(ctx: &Ctx, input: &Path) -> Probe {
    probe_streams(ctx.e, input)
}

fn probe_streams(e: &Engines, input: &Path) -> Probe {
    let mut p = Probe::default();
    if !e.has(EngineId::Ffprobe) {
        p.has_audio = true;
        p.has_video = true;
        return p;
    }
    let args = [os("-v"), os("error"), os("-show_entries"), os("format=duration:stream=codec_type,width,height"), os("-of"), os("default=nw=1"), input.into()];
    if let Ok(out) = process::run(e, EngineId::Ffprobe, &args, None, &CancelToken::default(), |_| {}) {
        for l in out.lines() {
            match l.split_once('=') {
                Some(("duration", v)) => p.duration = v.trim().parse().ok().filter(|d: &f64| *d > 0.0),
                Some(("codec_type", "video")) => p.has_video = true,
                Some(("codec_type", "audio")) => p.has_audio = true,
                Some(("width", v)) if p.width.is_none() => p.width = v.trim().parse().ok(),
                Some(("height", v)) if p.height.is_none() => p.height = v.trim().parse().ok(),
                _ => {}
            }
        }
    }
    p
}

fn media_duration(ctx: &Ctx, input: &Path) -> Option<f64> {
    probe(ctx, input).duration
}

fn run_ffmpeg(ctx: &mut Ctx, args: Vec<OsString>, duration: Option<f64>, label: &str) -> R<()> {
    ctx.stage(if duration.is_some() { Some(0.0) } else { None }, label);
    let sink = &mut *ctx.sink;
    let label = label.to_string();
    process::run(ctx.e, EngineId::Ffmpeg, &args, Some(&ctx.tmp), ctx.cancel, |line| {
        if let Some(v) = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms=")) {
            if let (Ok(us), Some(d)) = (v.trim().parse::<f64>(), duration) {
                sink.progress(Some((us / 1e6 / d).clamp(0.0, 0.99)), &label);
            }
        }
    })?;
    Ok(())
}

fn audio_args(to: Format, o: &Options) -> R<Vec<OsString>> {
    let br = |d: u32| os(format!("{}k", o.audio_bitrate_kbps.unwrap_or(d)));
    let mut a: Vec<OsString> = match to {
        Format::Mp3 => vec![os("-c:a"), os("libmp3lame"), os("-b:a"), br(192)],
        Format::Wav => vec![os("-c:a"), os("pcm_s16le")],
        Format::Aiff => vec![os("-c:a"), os("pcm_s16be")],
        Format::Flac => vec![os("-c:a"), os("flac")],
        Format::Aac | Format::M4a | Format::Mp4 | Format::Mov | Format::Mkv => vec![os("-c:a"), os("aac"), os("-b:a"), br(192)],
        Format::Ogg => vec![os("-c:a"), os("libvorbis"), os("-b:a"), br(192)],
        Format::Opus | Format::Webm => vec![os("-c:a"), os("libopus"), os("-b:a"), br(128)],
        Format::Wma | Format::Wmv => vec![os("-c:a"), os("wmav2"), os("-b:a"), br(192)],
        Format::Avi => vec![os("-c:a"), os("libmp3lame"), os("-b:a"), br(192)],
        Format::Mpeg => vec![os("-c:a"), os("mp2"), os("-b:a"), br(192)],
        _ => return Err(JobError::Invalid(format!("{} has no audio settings", to.label()))),
    };
    if let Some(sr) = o.sample_rate {
        if to == Format::Opus && ![8000, 12000, 16000, 24000, 48000].contains(&sr) {
            return Err(JobError::Invalid("Opus supports 8, 12, 16, 24 or 48 kHz".into()));
        }
        a.extend([os("-ar"), os(sr.to_string())]);
    } else if matches!(to, Format::Opus | Format::Webm) {
        a.extend([os("-ar"), os("48000")]);
    } else if matches!(to, Format::Wma | Format::Wmv | Format::Mpeg) {
        a.extend([os("-ar"), os("44100")]);
    }
    if let Some(c) = o.channels {
        a.extend([os("-ac"), os(c.to_string())]);
    } else if matches!(to, Format::Wma | Format::Wmv | Format::Mpeg | Format::Mp3 | Format::Avi) {
        // These encoders support at most 2 channels.
        a.extend([os("-ac"), os("2")]);
    }
    Ok(a)
}

fn video_args(to: Format, o: &Options) -> R<Vec<OsString>> {
    let codec = o.video_codec.clone().unwrap_or_else(|| match to {
        Format::Webm => "vp9".into(),
        Format::Avi => "mpeg4".into(),
        Format::Wmv => "wmv2".into(),
        Format::Mpeg => "mpeg2".into(),
        _ => "h264".into(),
    });
    let allowed: &[&str] = match to {
        Format::Mp4 => &["h264", "h265", "av1", "vp9", "mpeg4", "copy"],
        Format::Mov => &["h264", "h265", "mpeg4", "copy"],
        Format::Mkv => &["h264", "h265", "av1", "vp9", "mpeg4", "copy"],
        Format::Webm => &["vp9", "av1", "copy"],
        Format::Avi => &["mpeg4", "h264", "copy"],
        Format::Wmv => &["wmv2"],
        Format::Mpeg => &["mpeg2"],
        _ => &[],
    };
    if !allowed.contains(&codec.as_str()) {
        return Err(JobError::Invalid(format!("The {} codec cannot be stored in {}", codec.to_uppercase(), to.label())));
    }
    let crf = |d: u32| os(o.crf.unwrap_or(d).to_string());
    let mut a: Vec<OsString> = match codec.as_str() {
        "h264" => vec![os("-c:v"), os("libx264"), os("-preset"), os("medium"), os("-crf"), crf(23), os("-pix_fmt"), os("yuv420p")],
        "h265" => vec![os("-c:v"), os("libx265"), os("-preset"), os("medium"), os("-crf"), crf(26), os("-pix_fmt"), os("yuv420p"), os("-tag:v"), os("hvc1")],
        "vp9" => vec![os("-c:v"), os("libvpx-vp9"), os("-crf"), crf(32), os("-b:v"), os("0"), os("-row-mt"), os("1"), os("-deadline"), os("good"), os("-cpu-used"), os("4"), os("-pix_fmt"), os("yuv420p")],
        "av1" => vec![os("-c:v"), os("libsvtav1"), os("-crf"), crf(35), os("-preset"), os("8"), os("-pix_fmt"), os("yuv420p")],
        "mpeg4" => vec![os("-c:v"), os("mpeg4"), os("-q:v"), os("4"), os("-pix_fmt"), os("yuv420p")],
        "wmv2" => vec![os("-c:v"), os("wmv2"), os("-q:v"), os("4")],
        "mpeg2" => vec![os("-c:v"), os("mpeg2video"), os("-q:v"), os("4"), os("-pix_fmt"), os("yuv420p")],
        "copy" => vec![os("-c:v"), os("copy")],
        _ => unreachable!(),
    };
    if matches!(to, Format::Mp4 | Format::Mov) {
        a.extend([os("-movflags"), os("+faststart")]);
    }
    Ok(a)
}

fn ffmpeg_convert(ctx: &mut Ctx, input: &Path, src: Format, to: Format, o: &Options, variant: Variant) -> R<Produced> {
    let pr = probe(ctx, input);
    let audio_only = to.category() == Category::Audio;
    if audio_only && !pr.has_audio {
        return Err(JobError::Invalid("This file has no audio track".into()));
    }
    if to.category() == Category::Video && !pr.has_video {
        return Err(JobError::Invalid("This file has no video track".into()));
    }
    let out = ctx.tmp.join(format!("out.{}", to.primary_ext()));
    let mut args = vec![os("-hide_banner"), os("-nostdin"), os("-y"), os("-progress"), os("pipe:1"), os("-nostats")];
    if let Some(s) = o.start {
        args.extend([os("-ss"), os(format!("{s:.3}"))]);
    }
    args.extend([os("-i"), input.into()]);
    if let Some(e) = o.end {
        args.extend([os("-t"), os(format!("{:.3}", e - o.start.unwrap_or(0.0)))]);
    }
    let mut dur = pr.duration.map(|d| o.end.unwrap_or(d).min(d) - o.start.unwrap_or(0.0)).filter(|d| *d > 0.0);
    if o.start.map_or(false, |s| pr.duration.map_or(false, |d| s >= d)) {
        return Err(JobError::Invalid("Start time is beyond the end of the file".into()));
    }
    args.extend([os("-map_metadata"), os("0")]);

    if to == Format::Gif {
        let fps = o.fps.unwrap_or(12.0);
        let w = o.gif_width.unwrap_or(480);
        let mut vf = String::new();
        if let Some(c) = o.crop {
            vf.push_str(&format!("crop={}:{}:{}:{},", c.width, c.height, c.x, c.y));
        }
        vf.push_str(&format!("fps={fps},scale={w}:-1:flags=lanczos,split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=4"));
        args.extend([os("-vf"), os(vf), os("-loop"), os("0"), os("-an")]);
    } else if audio_only {
        args.push(os("-vn"));
        args.extend(audio_args(to, o)?);
        if variant == Variant::Normalize {
            args.extend([os("-af"), os("loudnorm=I=-16:TP=-1.5:LRA=11")]);
            // loudnorm upsamples to 192 kHz internally; bring it back to a normal rate.
            if o.sample_rate.is_none() {
                let sr = if matches!(to, Format::Wma | Format::Mpeg) { "44100" } else { "48000" };
                args.extend([os("-ar"), os(sr)]);
            }
        }
    } else {
        let mut filters = vec![];
        if let Some(c) = o.crop {
            filters.push(format!("crop={}:{}:{}:{}", c.width, c.height, c.x, c.y));
        }
        if let Some(h) = o.resolution {
            filters.push(format!("scale=-2:{h}"));
        }
        if let Some(f) = o.fps {
            filters.push(format!("fps={f}"));
        }
        // yuv420p requires even dimensions.
        filters.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".into());
        let vargs = video_args(to, o)?;
        let copying = vargs.iter().any(|a| a == "copy");
        if copying && (o.crop.is_some() || o.resolution.is_some() || o.fps.is_some()) {
            return Err(JobError::Invalid("Stream copy cannot be combined with crop, resolution or FPS changes".into()));
        }
        if !copying {
            args.extend([os("-vf"), os(filters.join(","))]);
        }
        args.extend(vargs);
        if pr.has_audio {
            args.extend(audio_args(to, o)?);
            if variant == Variant::Normalize {
                args.extend([os("-af"), os("loudnorm=I=-16:TP=-1.5:LRA=11")]);
            }
        } else {
            args.push(os("-an"));
        }
        args.extend([os("-map"), os("0:v:0"), os("-map"), os("0:a:0?")]);
        if src == Format::Gif {
            dur = dur.or(pr.duration);
        }
    }
    args.push(out.clone().into());
    run_ffmpeg(ctx, args, dur, &format!("Converting to {}", to.label()))?;
    Ok(Produced::File(out, to))
}

fn media_merge(ctx: &mut Ctx, inputs: &[PathBuf], first: Format) -> R<Produced> {
    let probes: Vec<Probe> = inputs.iter().map(|p| probe(ctx, p)).collect();
    let is_video = first.category() == Category::Video;
    let out = ctx.tmp.join(format!("out.{}", first.primary_ext()));
    let mut args = vec![os("-hide_banner"), os("-nostdin"), os("-y"), os("-progress"), os("pipe:1"), os("-nostats")];
    for p in inputs {
        args.extend([os("-i"), p.into()]);
    }
    let n = inputs.len();
    let total: Option<f64> = probes.iter().map(|p| p.duration).sum();
    let mut graph = String::new();
    if is_video {
        let with_audio = probes.iter().filter(|p| p.has_audio).count();
        if with_audio != 0 && with_audio != n {
            return Err(JobError::Invalid("Cannot merge: some videos have audio and others do not".into()));
        }
        let (w, h) = (probes[0].width.unwrap_or(1280) / 2 * 2, probes[0].height.unwrap_or(720) / 2 * 2);
        for i in 0..n {
            graph.push_str(&format!("[{i}:v:0]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v{i}];"));
            if with_audio > 0 {
                graph.push_str(&format!("[{i}:a:0]aresample=48000,aformat=channel_layouts=stereo[a{i}];"));
            }
        }
        for i in 0..n {
            graph.push_str(&format!("[v{i}]"));
            if with_audio > 0 {
                graph.push_str(&format!("[a{i}]"));
            }
        }
        let a = if with_audio > 0 { 1 } else { 0 };
        graph.push_str(&format!("concat=n={n}:v=1:a={a}[v]{}", if a == 1 { "[a]" } else { "" }));
        args.extend([os("-filter_complex"), os(graph), os("-map"), os("[v]")]);
        if a == 1 {
            args.extend([os("-map"), os("[a]")]);
            args.extend(audio_args(first, ctx.opts)?);
        }
        let mut o = ctx.opts.clone();
        o.video_codec = None;
        args.extend(video_args(first, &o)?);
    } else {
        if probes.iter().any(|p| !p.has_audio) {
            return Err(JobError::Invalid("One of the files has no audio track".into()));
        }
        for i in 0..n {
            graph.push_str(&format!("[{i}:a:0]aresample=48000,aformat=channel_layouts=stereo[a{i}];"));
        }
        for i in 0..n {
            graph.push_str(&format!("[a{i}]"));
        }
        graph.push_str(&format!("concat=n={n}:v=0:a=1[a]"));
        args.extend([os("-filter_complex"), os(graph), os("-map"), os("[a]")]);
        args.extend(audio_args(first, ctx.opts)?);
    }
    args.push(out.clone().into());
    run_ffmpeg(ctx, args, total, "Merging")?;
    Ok(Produced::File(out, first))
}

// ---------------- Archives ----------------

fn sevenzip_progress(ctx: &mut Ctx, args: Vec<OsString>, label: &str) -> R<()> {
    let sink = &mut *ctx.sink;
    let label = label.to_string();
    sink.progress(Some(0.0), &label);
    process::run(ctx.e, EngineId::Sevenzip, &args, Some(&ctx.tmp), ctx.cancel, |line| {
        if let Some(pos) = line.find('%') {
            if let Ok(v) = line[..pos].trim().rsplit(' ').next().unwrap_or("").parse::<f64>() {
                sink.progress(Some((v / 100.0).clamp(0.0, 0.99)), &label);
            }
        }
    })?;
    Ok(())
}

fn extract_archive(ctx: &mut Ctx, input: &Path, src: Format, dest: &Path) -> R<()> {
    let mut args = vec![os("x"), os("-y"), os("-bsp1"), os("-bso0"), os("-snld")];
    match &ctx.opts.password {
        Some(pw) => args.push(os(format!("-p{pw}"))),
        // An explicit empty password makes 7-Zip fail on encrypted archives instead of waiting
        // for keyboard input.
        None => args.push(os("-p")),
    }
    let mut o = OsString::from("-o");
    o.push(dest.as_os_str());
    args.push(o);
    args.push(input.into());
    sevenzip_progress(ctx, args, "Extracting")?;
    // .tar.gz (and .gz holding a tar): unpack the inner tar too.
    if matches!(src, Format::TarGz | Format::Gz) {
        let entries: Vec<PathBuf> = fs::read_dir(dest)?.flatten().map(|e| e.path()).collect();
        if entries.len() == 1 && detect_path(&entries[0]).ok().and_then(|d| d.format) == Some(Format::Tar) {
            let tar = entries[0].clone();
            let mut o = OsString::from("-o");
            o.push(dest.as_os_str());
            sevenzip_progress(ctx, vec![os("x"), os("-y"), os("-bsp1"), os("-bso0"), os("-snld"), o, tar.clone().into()], "Extracting")?;
            fs::remove_file(tar)?;
        }
    }
    check_tree(dest, dest)?;
    if fs::read_dir(dest)?.next().is_none() {
        return Err(JobError::Validation("The archive is empty".into()));
    }
    Ok(())
}

/// Defense in depth against path traversal: no reparse points (symlinks/junctions) may come out
/// of an archive, and every entry must be inside the extraction folder.
fn check_tree(root: &Path, p: &Path) -> R<()> {
    for e in fs::read_dir(p)?.flatten() {
        let meta = fs::symlink_metadata(e.path())?;
        if meta.file_type().is_symlink() {
            let _ = fs::remove_file(e.path()).or_else(|_| fs::remove_dir(e.path()));
            return Err(JobError::Invalid("The archive contains links, which are not extracted for safety".into()));
        }
        if !e.path().starts_with(root) {
            return Err(JobError::Invalid("The archive contains paths outside the target folder".into()));
        }
        if meta.is_dir() {
            check_tree(root, &e.path())?;
        }
    }
    Ok(())
}

fn archive_type(f: Format) -> &'static str {
    match f { Format::Zip => "zip", Format::SevenZ => "7z", Format::Tar | Format::TarGz => "tar", _ => "zip" }
}

fn password_args(password: Option<&str>, fmt: Format) -> R<Vec<OsString>> {
    Ok(match (password, fmt) {
        (None, _) => vec![],
        (Some(pw), Format::Zip) => vec![os(format!("-p{pw}")), os("-mem=AES256")],
        (Some(pw), Format::SevenZ) => vec![os(format!("-p{pw}")), os("-mhe=on")],
        (Some(_), _) => return Err(JobError::Invalid(format!("{} archives cannot be password protected; use ZIP or 7-Zip", fmt.label()))),
    })
}

fn archive_create(ctx: &mut Ctx, inputs: &[PathBuf], fmt: Format, password: Option<String>) -> R<Produced> {
    let name = format!("out.{}", fmt.primary_ext());
    let out = ctx.tmp.join(&name);
    let first = if fmt == Format::TarGz { ctx.tmp.join("stage.tar") } else { out.clone() };
    let mut args = vec![os("a"), os(format!("-t{}", archive_type(fmt))), os("-bsp1"), os("-bso0"), os("-y")];
    args.extend(password_args(password.as_deref(), fmt)?);
    args.push(first.clone().into());
    args.extend(inputs.iter().map(|p| p.into()));
    sevenzip_progress(ctx, args, "Compressing")?;
    if fmt == Format::TarGz {
        sevenzip_progress(ctx, vec![os("a"), os("-tgzip"), os("-bsp1"), os("-bso0"), os("-y"), out.clone().into(), first.clone().into()], "Compressing")?;
        fs::remove_file(first)?;
    }
    Ok(Produced::File(out, fmt))
}

fn archive_from_dir(ctx: &mut Ctx, dir: &Path, fmt: Format) -> R<Produced> {
    let entries: Vec<PathBuf> = fs::read_dir(dir)?.flatten().map(|e| e.path()).collect();
    // The password (if any) belongs to the source archive; the new one is not encrypted.
    archive_create(ctx, &entries, fmt, None)
}

// ---------------- Finalization ----------------

fn finalize(ctx: &mut Ctx, produced: Produced, spec: &JobSpec, inputs: &[PathBuf], out_dir: &Path) -> R<Outcome> {
    let action = spec.action;
    let sfx = suffix(action);
    let base_input = if inputs.len() > 1 && matches!(action, Action::ArchiveCreate(_)) {
        PathBuf::from("Archive")
    } else {
        inputs[0].clone()
    };
    let name = output::render_name(&spec.output.template, &base_input, sfx);
    let protected: Vec<PathBuf> = inputs.to_vec();
    match produced {
        Produced::Skipped(note) => Ok(Outcome { outputs: vec![], skipped: true, note: Some(note) }),
        Produced::File(tmp_file, fmt) => {
            validate_output(ctx, &tmp_file, fmt)?;
            let target = output::reserve_path(out_dir, &name, fmt.primary_ext(), spec.output.conflict, &protected)?;
            let Some(target) = target else {
                return Ok(Outcome { outputs: vec![], skipped: true, note: Some("A file with that name already exists (skipped)".into()) });
            };
            move_file(&tmp_file, &target)?;
            Ok(Outcome { outputs: vec![target], skipped: false, note: None })
        }
        Produced::Many(dir, fmt) => {
            let mut files: Vec<PathBuf> = fs::read_dir(&dir)?.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
            files.sort();
            if files.is_empty() {
                return Err(JobError::Validation("No output files were produced".into()));
            }
            for f in &files {
                validate_output(ctx, f, fmt)?;
            }
            if files.len() == 1 {
                let target = output::reserve_path(out_dir, &name, fmt.primary_ext(), spec.output.conflict, &protected)?;
                let Some(target) = target else {
                    return Ok(Outcome { outputs: vec![], skipped: true, note: Some("A file with that name already exists (skipped)".into()) });
                };
                move_file(&files[0], &target)?;
                return Ok(Outcome { outputs: vec![target], skipped: false, note: None });
            }
            let folder_name = if sfx.is_empty() { format!("{name} ({})", fmt.primary_ext()) } else { name };
            let target = output::reserve_dir(out_dir, &folder_name)?;
            for f in &files {
                move_file(f, &target.join(f.file_name().unwrap()))?;
            }
            Ok(Outcome { outputs: vec![target], skipped: false, note: Some(format!("{} files", files.len())) })
        }
        Produced::Tree(dir) => {
            if fs::read_dir(&dir)?.next().is_none() {
                return Err(JobError::Validation("Nothing was produced".into()));
            }
            let target = output::reserve_dir(out_dir, &name)?;
            move_tree(&dir, &target)?;
            let count = count_files(&target);
            Ok(Outcome { outputs: vec![target], skipped: false, note: Some(format!("{count} files")) })
        }
    }
}

fn count_files(p: &Path) -> usize {
    fs::read_dir(p).map(|rd| rd.flatten().map(|e| if e.path().is_dir() { count_files(&e.path()) } else { 1 }).sum()).unwrap_or(0)
}

fn move_file(from: &Path, to: &Path) -> R<()> {
    // The reserved placeholder may exist (Rename) — replace it.
    if fs::rename(from, to).is_err() {
        fs::copy(from, to)?;
        let _ = fs::remove_file(from);
    }
    Ok(())
}

fn move_tree(from: &Path, to: &Path) -> R<()> {
    for e in fs::read_dir(from)?.flatten() {
        let dest = to.join(e.file_name());
        if e.path().is_dir() {
            fs::create_dir_all(&dest)?;
            move_tree(&e.path(), &dest)?;
        } else {
            move_file(&e.path(), &dest)?;
        }
    }
    Ok(())
}

/// Verify that an output exists, is non-empty, has the expected format, and (where an engine
/// can do it) actually decodes.
fn validate_output(ctx: &mut Ctx, p: &Path, expected: Format) -> R<()> {
    let meta = fs::metadata(p).map_err(|_| JobError::Validation(format!("{} was not created", file_name(p))))?;
    if meta.len() == 0 {
        return Err(JobError::Validation(format!("{} is empty", file_name(p))));
    }
    let det = detect_path(p).map_err(|e| JobError::Validation(e.to_string()))?;
    let got = det.format;
    let ok = match (expected, got) {
        (e, Some(g)) if e == g => true,
        (Format::Heic | Format::Heif, Some(Format::Heic | Format::Heif)) => true,
        (Format::M4a, Some(Format::Mp4)) | (Format::Mp4, Some(Format::M4a)) => true,
        (Format::Txt | Format::Md | Format::Csv, Some(Format::Txt | Format::Md | Format::Csv | Format::Html)) => true,
        (Format::Html, Some(Format::Txt | Format::Html)) => true,
        (Format::Rtf, Some(Format::Rtf)) => true,
        (Format::TarGz, Some(Format::Gz)) => true,
        _ => false,
    };
    if !ok {
        return Err(JobError::Validation(format!("expected {} but the output is {}", expected.label(), got.map_or("unrecognized".to_string(), |g| g.label().to_string()))));
    }
    // Deep checks
    let e = ctx.e;
    let quiet = CancelToken::default();
    match expected.category() {
        Category::Audio | Category::Video if e.has(EngineId::Ffprobe) => {
            let out = process::run(e, EngineId::Ffprobe, &[os("-v"), os("error"), os("-show_entries"), os("format=duration"), os("-of"), os("default=nw=1:nk=1"), p.into()], None, &quiet, |_| {})
                .map_err(|err| JobError::Validation(format!("output does not decode: {err}")))?;
            let d: f64 = out.trim().parse().unwrap_or(0.0);
            if d <= 0.0 {
                return Err(JobError::Validation("output has no playable duration".into()));
            }
        }
        Category::Pdf if e.has(EngineId::Qpdf) => {
            // --check fails on encrypted files without password; use --show-npages which works with an empty user password only.
            if spec_needs_password(p) {
                return Ok(());
            }
            let out = process::run(e, EngineId::Qpdf, &[os("--show-npages"), p.into()], None, &quiet, |_| {})
                .map_err(|err| JobError::Validation(format!("output PDF is unreadable: {err}")))?;
            if out.trim().parse::<u32>().unwrap_or(0) == 0 {
                return Err(JobError::Validation("output PDF has no pages".into()));
            }
        }
        Category::Image if e.has(EngineId::Magick) && e.magick_formats.can_read(magick_coder(expected)) && expected != Format::Svg => {
            let mut arg = p.as_os_str().to_os_string();
            arg.push("[0]");
            process::run(e, EngineId::Magick, &[os("identify"), os("-ping"), arg], None, &quiet, |_| {})
                .map_err(|err| JobError::Validation(format!("output image is unreadable: {err}")))?;
        }
        _ => {}
    }
    Ok(())
}

fn spec_needs_password(p: &Path) -> bool {
    // Encrypted PDFs contain an /Encrypt entry in the trailer.
    fs::read(p).map(|b| b.windows(8).any(|w| w == b"/Encrypt")).unwrap_or(false)
}

pub fn conflict_default() -> Conflict {
    Conflict::Rename
}
