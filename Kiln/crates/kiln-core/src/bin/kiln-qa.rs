//! Kiln QA harness.
//!
//! 1. Generates a representative test dataset with the installed engines.
//! 2. Runs every action the catalog offers for every fixture through the real executor.
//! 3. Independently re-checks each output (exists, non-empty, detected format).
//! 4. Runs scenario tests: batch, cancellation, collisions, corrupt input, wrong password,
//!    unicode names, temp cleanup, interrupted-history recovery.
//! 5. Writes QA_REPORT.md and qa-results.json.
//!
//! Usage: kiln-qa <work-dir> <report.md> [--filter <substring>]

use kiln_core::catalog::{actions_for_selection, Action, ActionInfo, Input};
use kiln_core::exec::actions_for_paths;
use kiln_core::detect::detect_path;
use kiln_core::engines::{EngineId, Engines};
use kiln_core::exec::{action_name, execute, JobSpec};
use kiln_core::formats::{Category, Format};
use kiln_core::history::History;
use kiln_core::jobs::{JobManager, JobStatus};
use kiln_core::options::{Crop, Options};
use kiln_core::output::{Conflict, OutputSettings};
use kiln_core::process::{os, run, CancelToken};
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Serialize, Clone)]
struct Row {
    feature: String,
    input: String,
    output: String,
    expected: String,
    actual: String,
    status: &'static str,
    error: String,
    seconds: f64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: kiln-qa <work-dir> <report.md> [--filter s]");
        std::process::exit(2);
    }
    let work = PathBuf::from(&args[1]);
    let report = PathBuf::from(&args[2]);
    let filter = args.iter().position(|a| a == "--filter").and_then(|i| args.get(i + 1)).cloned();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/office.ps1");
    // KILN_ENGINES_DIR points at a bundled engines folder (as shipped in the installer);
    // with KILN_BUNDLED_ONLY=1 the system-wide installs are ignored.
    let bundled = std::env::var_os("KILN_ENGINES_DIR").map(PathBuf::from);
    let e = Engines::discover_with(&HashMap::new(), Some(kiln_core::output::strip_verbatim(&fs::canonicalize(&script).expect("office.ps1"))), bundled.as_deref());
    let fixtures = work.join("fixtures");
    let out_root = work.join("out");
    let _ = fs::remove_dir_all(&out_root);
    fs::create_dir_all(&fixtures).unwrap();
    fs::create_dir_all(&out_root).unwrap();

    println!("== Engines");
    for s in e.statuses() {
        println!("  {:<22} {:<5} {}", s.label, if s.available { "yes" } else { "no" }, s.version.clone().or(s.problem.clone()).unwrap_or_default());
    }

    println!("== Generating fixtures");
    let mut rows: Vec<Row> = vec![];
    let fx = generate_fixtures(&e, &fixtures, &mut rows);
    println!("  {} fixtures", fx.len());

    println!("== Conversion matrix");
    let started = Instant::now();
    let scenarios_only = args.iter().any(|a| a == "--scenarios-only");
    if !scenarios_only {
        matrix(&e, &fx, &out_root, &filter, &mut rows);
    }
    println!("== Scenarios");
    if filter.is_none() {
        scenarios(&e, &fx, &work, &mut rows);
    }
    let total = started.elapsed().as_secs_f64();
    write_report(&e, &rows, &report, total);
    let pass = rows.iter().filter(|r| r.status == "PASS").count();
    let fail = rows.iter().filter(|r| r.status == "FAIL").count();
    println!("== {pass} passed, {fail} failed, {} total", rows.len());
    if fail > 0 {
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------- fixtures

struct Fixture {
    path: PathBuf,
    format: Format,
}

fn gen(e: &Engines, id: EngineId, args: Vec<OsString>, cwd: &Path) -> Result<(), String> {
    run(e, id, &args, Some(cwd), &CancelToken::default(), |_| {}).map(|_| ()).map_err(|e| e.to_string())
}

fn generate_fixtures(e: &Engines, dir: &Path, rows: &mut Vec<Row>) -> Vec<Fixture> {
    let mut v = vec![];
    let mut add = |name: &str, res: Result<(), String>, rows: &mut Vec<Row>| {
        let p = dir.join(name);
        let ok = res.is_ok() && p.exists() && fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false);
        if !ok {
            rows.push(Row { feature: "Fixture generation".into(), input: "-".into(), output: name.into(), expected: "fixture created".into(), actual: "not created".into(), status: "FAIL", error: res.err().unwrap_or_default(), seconds: 0.0 });
            return;
        }
        let det = detect_path(&p).ok().and_then(|d| d.format);
        match det {
            Some(f) => v.push(Fixture { path: p, format: f }),
            None => rows.push(Row { feature: "Fixture generation".into(), input: "-".into(), output: name.into(), expected: "detectable".into(), actual: "undetected".into(), status: "FAIL", error: String::new(), seconds: 0.0 }),
        }
    };
    let p = |n: &str| -> OsString { dir.join(n).into() };
    use EngineId::*;

    // Images
    let base = gen(e, Magick, vec![os("-size"), os("800x480"), os("gradient:#ff8a00-#3355ff"), os("-fill"), os("white"), os("-draw"), os("rectangle 40,300 760,440"), os("-fill"), os("#ffffff80"), os("-draw"), os("circle 400,150 400,40"), os("-fill"), os("black"), os("-font"), os("Arial"), os("-pointsize"), os("64"), os("-annotate"), os("+70+395"), os("Kiln QA sample"), p("sample.png")], dir);
    add("sample.png", base, rows);
    // HEIC cannot be encoded with the installed ImageMagick (read-only HEIC support), so a
    // sample HEIC (e.g. libheif's examples/example.heic) can be supplied via KILN_QA_HEIC.
    if let Some(h) = std::env::var_os("KILN_QA_HEIC").map(PathBuf::from).filter(|p| p.is_file()) {
        let _ = fs::copy(&h, dir.join("photo.heic"));
        add("photo.heic", Ok(()), rows);
    }
    add("transparent.png", gen(e, Magick, vec![os("-size"), os("300x300"), os("xc:none"), os("-fill"), os("#ff000080"), os("-draw"), os("circle 150,150 150,40"), p("transparent.png")], dir), rows);
    for (name, coder) in [("sample.jpg", "JPEG"), ("sample.webp", "WEBP"), ("sample.heic", "HEIC"), ("sample.avif", "AVIF"), ("sample.tiff", "TIFF"), ("sample.bmp", "BMP"), ("sample.ico", "ICO")] {
        if !e.magick_formats.can_write(coder) {
            continue; // e.g. HEIC: read-only in this ImageMagick build
        }
        let mut out = OsString::from(format!("{coder}:"));
        out.push(dir.join(name));
        let mut args = vec![p("sample.png")];
        if coder == "ICO" {
            args.extend([os("-resize"), os("256x256")]);
        }
        args.push(out);
        add(name, gen(e, Magick, args, dir), rows);
    }
    let heic_src = if dir.join("sample.heic").exists() { dir.join("sample.heic") } else { dir.join("photo.heic") };
    if heic_src.exists() {
        let _ = fs::copy(&heic_src, dir.join("photo.heif"));
        add("photo.heif", Ok(()), rows);
    }
    add("animated.gif", gen(e, Magick, vec![os("-delay"), os("25"), os("-size"), os("160x120"), os("xc:#e74c3c"), os("xc:#2ecc71"), os("xc:#3498db"), os("xc:#f1c40f"), os("-loop"), os("0"), p("animated.gif")], dir), rows);
    fs::write(dir.join("vector.svg"), r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120" viewBox="0 0 200 120"><rect width="200" height="120" rx="16" fill="#ff8a00"/><circle cx="60" cy="60" r="36" fill="#fff"/><text x="110" y="70" font-family="Arial" font-size="28" fill="#222">SVG</text></svg>"##).unwrap();
    add("vector.svg", Ok(()), rows);
    let _ = fs::copy(dir.join("sample.png"), dir.join("Ünïcødé 文件 (1).png"));
    add("Ünïcødé 文件 (1).png", Ok(()), rows);

    // Audio
    let tone = |n: &str, extra: &[&str]| -> Result<(), String> {
        let mut a = vec![os("-hide_banner"), os("-y"), os("-f"), os("lavfi"), os("-i"), os("sine=frequency=440:duration=3"), os("-ac"), os("2")];
        a.extend(extra.iter().map(|s| os(*s)));
        a.push(dir.join(n).into());
        gen(e, Ffmpeg, a, dir)
    };
    add("tone.wav", tone("tone.wav", &[]), rows);
    add("tone.mp3", tone("tone.mp3", &["-c:a", "libmp3lame", "-b:a", "192k"]), rows);
    add("tone.aac", tone("tone.aac", &["-c:a", "aac"]), rows);
    add("tone.m4a", tone("tone.m4a", &["-c:a", "aac"]), rows);
    add("tone.flac", tone("tone.flac", &[]), rows);
    add("tone.ogg", tone("tone.ogg", &["-c:a", "libvorbis"]), rows);
    add("tone.opus", tone("tone.opus", &["-c:a", "libopus", "-ar", "48000"]), rows);
    add("tone.aiff", tone("tone.aiff", &[]), rows);
    add("tone.wma", tone("tone.wma", &["-c:a", "wmav2"]), rows);
    add("spaces & symbols #1.wav", tone("spaces & symbols #1.wav", &["-ar", "22050"]), rows);

    // Video
    let clip = |n: &str, extra: &[&str], audio: bool| -> Result<(), String> {
        let mut a = vec![os("-hide_banner"), os("-y"), os("-f"), os("lavfi"), os("-i"), os("testsrc2=size=320x240:rate=25:duration=3")];
        if audio {
            a.extend([os("-f"), os("lavfi"), os("-i"), os("sine=frequency=660:duration=3")]);
        }
        a.extend(extra.iter().map(|s| os(*s)));
        a.push(os("-shortest"));
        a.push(dir.join(n).into());
        gen(e, Ffmpeg, a, dir)
    };
    add("clip.mp4", clip("clip.mp4", &["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"], true), rows);
    add("clip.mov", clip("clip.mov", &["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"], true), rows);
    add("clip.mkv", clip("clip.mkv", &["-c:v", "libx264", "-c:a", "aac"], true), rows);
    add("clip.avi", clip("clip.avi", &["-c:v", "mpeg4", "-c:a", "libmp3lame"], true), rows);
    add("clip.webm", clip("clip.webm", &["-c:v", "libvpx-vp9", "-b:v", "300k", "-c:a", "libopus"], true), rows);
    add("clip.wmv", clip("clip.wmv", &["-c:v", "wmv2", "-c:a", "wmav2"], true), rows);
    add("clip.mpeg", clip("clip.mpeg", &["-c:v", "mpeg2video", "-c:a", "mp2", "-f", "mpeg"], true), rows);
    add("silent.mp4", clip("silent.mp4", &["-c:v", "libx264", "-pix_fmt", "yuv420p"], false), rows);

    // Documents
    let md = "# Kiln QA document\n\nThis is a **test document** with *formatting*, a list and a table.\n\n- Alpha\n- Beta — ünïcødé ✓\n\n| Col A | Col B |\n|---|---|\n| 1 | 2 |\n\n```{=openxml}\n<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>\n```\n\n## Second page\n\nThe quick brown fox jumps over the lazy dog.\n";
    fs::write(dir.join("doc.md"), md).unwrap();
    add("doc.md", Ok(()), rows);
    fs::write(dir.join("page.html"), "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>QA</title></head><body><h1>Kiln QA</h1><p>Hello <b>world</b> — ünïcødé.</p></body></html>").unwrap();
    add("page.html", Ok(()), rows);
    fs::write(dir.join("notes.txt"), "Kiln QA plain text\nLine two — ünïcødé ✓\n").unwrap();
    add("notes.txt", Ok(()), rows);
    fs::write(dir.join("data.csv"), "name,qty,price\nApple,3,1.20\nPear,5,0.95\nKiwi,12,0.30\n").unwrap();
    add("data.csv", Ok(()), rows);
    for (name, to) in [("doc.docx", "docx"), ("doc.odt", "odt"), ("doc.rtf", "rtf"), ("book.epub", "epub"), ("slides.pptx", "pptx")] {
        let mut a = vec![os("--from"), os("markdown"), os("--to"), os(to), os("--standalone"), os("--metadata"), os("title=Kiln QA"), os("--output"), p(name), p("doc.md")];
        if to == "pptx" {
            a = vec![os("--from"), os("markdown"), os("--to"), os("pptx"), os("--output"), p(name), p("doc.md")];
        }
        add(name, gen(e, Pandoc, a, dir), rows);
    }
    let office = |app: &str, input: &str, out: &str, fmt: &str| -> Result<(), String> {
        let script = e.office_script.clone().unwrap();
        let id = match app { "word" => Word, "excel" => Excel, _ => Powerpoint };
        gen(e, id, vec![os("-NoProfile"), os("-NonInteractive"), os("-ExecutionPolicy"), os("Bypass"), os("-File"), script.into(), os("-App"), os(app), os("-InFile"), p(input), os("-OutFile"), p(out), os("-Format"), os(fmt)], dir)
    };
    if e.has(Word) {
        add("doc.doc", office("word", "doc.docx", "doc.doc", "doc"), rows);
        add("document.pdf", office("word", "doc.docx", "document.pdf", "pdf"), rows);
    }
    if e.has(Excel) {
        add("sheet.xlsx", office("excel", "data.csv", "sheet.xlsx", "xlsx"), rows);
        add("sheet.xls", office("excel", "data.csv", "sheet.xls", "xls"), rows);
        add("sheet.ods", office("excel", "data.csv", "sheet.ods", "ods"), rows);
    }
    if e.has(Powerpoint) {
        add("slides.ppt", office("powerpoint", "slides.pptx", "slides.ppt", "ppt"), rows);
        add("slides.odp", office("powerpoint", "slides.pptx", "slides.odp", "odp"), rows);
    }
    // Scanned PDF (image only) for OCR
    add("scan.pdf", gen(e, Magick, vec![os("-size"), os("1700x600"), os("xc:white"), os("-font"), os("Arial"), os("-pointsize"), os("90"), os("-fill"), os("black"), os("-annotate"), os("+80+250"), os("Scanned page text"), os("-annotate"), os("+80+420"), os("OCR works offline"), os("-density"), os("200"), os("-units"), os("PixelsPerInch"), p("scan.pdf")], dir), rows);
    if e.has(Qpdf) && dir.join("document.pdf").exists() {
        add("locked.pdf", gen(e, Qpdf, vec![os("--encrypt"), os("--user-password=secret"), os("--owner-password=secret"), os("--bits=256"), os("--"), p("document.pdf"), p("locked.pdf")], dir), rows);
    }

    // Archives
    let seven = |args: Vec<OsString>| gen(e, Sevenzip, args, dir);
    add("bundle.zip", seven(vec![os("a"), os("-tzip"), p("bundle.zip"), p("notes.txt"), p("sample.png")]), rows);
    add("bundle.7z", seven(vec![os("a"), os("-t7z"), p("bundle.7z"), p("notes.txt"), p("sample.png")]), rows);
    add("bundle.tar", seven(vec![os("a"), os("-ttar"), p("bundle.tar"), p("notes.txt"), p("sample.png")]), rows);
    add("bundle.tar.gz", seven(vec![os("a"), os("-tgzip"), p("bundle.tar.gz"), p("bundle.tar")]), rows);
    add("notes.txt.gz", seven(vec![os("a"), os("-tgzip"), p("notes.txt.gz"), p("notes.txt")]), rows);
    add("secret.zip", seven(vec![os("a"), os("-tzip"), os("-psecret"), os("-mem=AES256"), p("secret.zip"), p("notes.txt")]), rows);
    let rar = PathBuf::from(r"C:\Program Files\WinRAR\Rar.exe");
    if rar.exists() {
        let st = std::process::Command::new(&rar).args(["a", "-ep", "-idq"]).arg(dir.join("bundle.rar")).arg(dir.join("notes.txt")).status();
        add("bundle.rar", st.map(|_| ()).map_err(|e| e.to_string()), rows);
    }

    // Damaged inputs (used by negative scenarios, not by the matrix)
    let mut junk = b"\x00\x00\x00\x18ftypisom\x00\x00\x00\x00".to_vec();
    junk.extend((0..4000u32).map(|i| (i * 31 % 251) as u8));
    fs::write(dir.join("broken.mp4"), junk).unwrap();
    fs::write(dir.join("broken.png"), b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDRgarbage").unwrap();
    fs::write(dir.join("broken.pdf"), b"%PDF-1.7\nthis is not really a pdf\n").unwrap();
    v
}

// ---------------------------------------------------------------- matrix

fn needs_opts(a: Action, f: Format) -> Options {
    let mut o = Options::default();
    match a {
        Action::ImageResize => o.width = Some(200),
        Action::ImageCrop => o.crop = Some(Crop { x: 10, y: 10, width: 100, height: 80 }),
        Action::VideoCrop => o.crop = Some(Crop { x: 0, y: 0, width: 160, height: 120 }),
        Action::ImageRotate => o.rotate = Some(90),
        Action::PdfRotate => o.rotate = Some(90),
        Action::PdfPages => o.pages = Some("z,1".into()),
        Action::PdfProtect => o.password = Some("secret".into()),
        Action::PdfUnlock => o.password = Some("secret".into()),
        Action::Trim => {
            o.start = Some(0.5);
            o.end = Some(2.0);
        }
        Action::VideoResize => {
            o.resolution = Some(120);
            o.fps = Some(15.0);
        }
        Action::ExtractFrames => o.fps = Some(2.0),
        Action::ArchiveExtract if f == Format::Zip => {}
        _ => {}
    }
    o
}

fn expected_output(a: &ActionInfo, src: Format) -> String {
    match a.action {
        Action::Convert(f) | Action::ImageOcr(f) => {
            if a.produces_folder || (src == Format::Pdf && f.category() == Category::Image) { format!("{} (file or folder of pages)", f.label()) } else { f.label().to_string() }
        }
        Action::ArchiveCreate(f) => f.label().into(),
        Action::SlidesToImages(f) => format!("folder of {}", f.label()),
        Action::GifFrames | Action::ExtractFrames => "folder of PNG".into(),
        Action::PdfSplit => "folder of PDF".into(),
        Action::ArchiveExtract => "folder".into(),
        Action::PdfMerge | Action::PdfPages | Action::PdfRotate | Action::PdfCompress | Action::PdfProtect | Action::PdfUnlock | Action::PdfOcr | Action::ImagesToPdf => "PDF".into(),
        _ => src.label().into(),
    }
}

fn matrix(e: &Engines, fx: &[Fixture], out_root: &Path, filter: &Option<String>, rows: &mut Vec<Row>) {
    let skip_names = ["Ünïcødé", "spaces &"];
    for f in fx {
        let name = f.path.file_name().unwrap().to_string_lossy().to_string();
        let is_variant = skip_names.iter().any(|s| name.starts_with(s));
        // The same file-aware list the UI shows (streams / text layer inspected).
        let actions = match actions_for_paths(&[f.path.clone()], e) {
            Ok(a) => a,
            Err(err) => {
                rows.push(Row { feature: "Action listing".into(), input: name.clone(), output: "-".into(), expected: "actions listed".into(), actual: "error".into(), status: "FAIL", error: err, seconds: 0.0 });
                continue;
            }
        };
        let all = actions.clone();
        for a in actions {
            if a.group == "tool" && all.iter().any(|b| b.group == "convert" && b.action == a.action) {
                continue; // "Extract text"/"Export to PDF" duplicates of a conversion
            }
            // Archive creation is format-independent: test it on a few inputs only.
            if matches!(a.action, Action::ArchiveCreate(_)) && !matches!(f.format, Format::Txt | Format::Png | Format::Mp4) {
                continue;
            }
            if a.action == Action::PdfUnlock && name != "locked.pdf" {
                continue;
            }
            if name == "locked.pdf" && a.action != Action::PdfUnlock {
                continue;
            }
            if is_variant && !matches!(a.action, Action::Convert(Format::Jpg | Format::Mp3 | Format::Webp)) {
                continue;
            }
            let label = format!("{} [{}]", a.label, action_name(a.action));
            if let Some(flt) = filter {
                let hay = format!("{name} {label}").to_lowercase();
                if !hay.contains(&flt.to_lowercase()) {
                    continue;
                }
            }
            let mut inputs = vec![f.path.clone()];
            if a.combines_inputs {
                // Use a second file of the same format.
                let copy = out_root.join(format!("second-{name}"));
                let _ = fs::copy(&f.path, &copy);
                inputs.push(copy);
            }
            let mut o = needs_opts(a.action, f.format);
            if name == "secret.zip" {
                o.password = Some("secret".into());
            }
            let dir = out_root.join(name.replace(['.', ' '], "_"));
            fs::create_dir_all(&dir).unwrap();
            let spec = JobSpec { inputs, action: a.action, options: o, output: OutputSettings { directory: Some(dir.clone()), template: "{name}".into(), conflict: Conflict::Rename } };
            rows.push(run_case(e, &spec, &a, f.format, &format!("{}: {}", category_name(f.format), a.label)));
        }
    }
}

fn category_name(f: Format) -> &'static str {
    f.category().label()
}

fn run_case(e: &Engines, spec: &JobSpec, a: &ActionInfo, src: Format, feature: &str) -> Row {
    let t = Instant::now();
    let cancel = CancelToken::default();
    let mut sink = |_: Option<f64>, _: &str| {};
    let before: Vec<u64> = spec.inputs.iter().map(|p| hash_file(p)).collect();
    let res = execute(spec, e, &cancel, &mut sink);
    let secs = t.elapsed().as_secs_f64();
    let after: Vec<u64> = spec.inputs.iter().map(|p| hash_file(p)).collect();
    let expected = expected_output(a, src);
    let input_name = spec.inputs[0].file_name().unwrap().to_string_lossy().to_string();
    let mut row = Row { feature: feature.into(), input: format!("{} ({})", src.label(), input_name), output: expected.clone(), expected: format!("valid {expected}"), actual: String::new(), status: "PASS", error: String::new(), seconds: (secs * 10.0).round() / 10.0 };
    if before != after {
        row.status = "FAIL";
        row.actual = "SOURCE FILE WAS MODIFIED".into();
        return row;
    }
    match res {
        Ok(o) if o.skipped => {
            row.actual = format!("skipped: {}", o.note.unwrap_or_default());
            row.status = "PASS";
            row.expected = format!("valid {expected}, or skipped when not smaller");
        }
        Ok(o) => {
            // Independent re-verification of what the executor reported.
            let mut problems = vec![];
            let mut desc = vec![];
            for p in &o.outputs {
                if p.is_dir() {
                    let files = walk(p);
                    if files.is_empty() {
                        problems.push(format!("{} is an empty folder", p.display()));
                    }
                    let mut kinds: Vec<String> = files.iter().filter_map(|f| detect_path(f).ok().and_then(|d| d.format).map(|x| x.label().to_string())).collect();
                    kinds.sort();
                    kinds.dedup();
                    if files.iter().any(|f| fs::metadata(f).map(|m| m.len() == 0).unwrap_or(true)) {
                        problems.push("empty file in output folder".into());
                    }
                    desc.push(format!("folder with {} files ({})", files.len(), kinds.join(", ")));
                } else {
                    let size = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
                    let det = detect_path(p).ok().and_then(|d| d.format);
                    if size == 0 {
                        problems.push("empty output".into());
                    }
                    desc.push(format!("{} file, {} bytes", det.map_or("unrecognized", |f| f.label()), size));
                    if let Action::Convert(to) = a.action {
                        let ok = det == Some(to) || matches!((to, det), (Format::Txt | Format::Md | Format::Html | Format::Csv, Some(Format::Txt | Format::Html | Format::Csv | Format::Md)) | (Format::M4a, Some(Format::Mp4)) | (Format::TarGz, Some(Format::Gz)) | (Format::Heic, Some(Format::Heif)))
                            || (src == Format::Pdf && to.category() == Category::Image && det == Some(to));
                        if !ok {
                            problems.push(format!("expected {} got {:?}", to.label(), det));
                        }
                    }
                }
            }
            if o.outputs.is_empty() {
                problems.push("no outputs reported".into());
            }
            row.actual = desc.join("; ");
            if !problems.is_empty() {
                row.status = "FAIL";
                row.error = problems.join("; ");
            }
        }
        Err(err) if matches!(a.action, Action::ImageOcr(Format::Txt)) && err.to_string().contains("No text was recognized") && !input_name.starts_with("sample.") => {
            // Images without legible text (shapes, tiny icons): the correct result is a clear message.
            row.expected = "recognized text, or a 'no text' message for images without text".into();
            row.actual = err.to_string();
        }
        Err(err) => {
            row.status = "FAIL";
            row.actual = "error".into();
            row.error = err.to_string();
        }
    }
    println!("  [{}] {:<60} {:>5.1}s {}", row.status, format!("{} <- {}", row.feature, row.input), secs, if row.error.is_empty() { "" } else { &row.error });
    row
}

fn walk(p: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(rd) = fs::read_dir(p) {
        for e in rd.flatten() {
            if e.path().is_dir() { v.extend(walk(&e.path())) } else { v.push(e.path()) }
        }
    }
    v
}

fn hash_file(p: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    fs::read(p).unwrap_or_default().hash(&mut h);
    h.finish()
}

// ---------------------------------------------------------------- scenarios

fn fixture<'a>(fx: &'a [Fixture], name: &str) -> Option<&'a Fixture> {
    fx.iter().find(|f| f.path.file_name().map_or(false, |n| n == name))
}

fn scenario(rows: &mut Vec<Row>, feature: &str, input: &str, expected: &str, f: impl FnOnce() -> Result<String, String>) {
    let t = Instant::now();
    let r = f();
    let secs = t.elapsed().as_secs_f64();
    let row = match r {
        Ok(actual) => Row { feature: feature.into(), input: input.into(), output: "-".into(), expected: expected.into(), actual, status: "PASS", error: String::new(), seconds: (secs * 10.0).round() / 10.0 },
        Err(err) => Row { feature: feature.into(), input: input.into(), output: "-".into(), expected: expected.into(), actual: "unexpected".into(), status: "FAIL", error: err, seconds: (secs * 10.0).round() / 10.0 },
    };
    println!("  [{}] {feature}: {}", row.status, if row.error.is_empty() { &row.actual } else { &row.error });
    rows.push(row);
}

fn scenarios(e: &Engines, fx: &[Fixture], work: &Path, rows: &mut Vec<Row>) {
    let sdir = work.join("scenarios");
    let _ = fs::remove_dir_all(&sdir);
    fs::create_dir_all(&sdir).unwrap();
    let out = |sub: &str| {
        let d = sdir.join(sub);
        fs::create_dir_all(&d).unwrap();
        OutputSettings { directory: Some(d), template: "{name}".into(), conflict: Conflict::Rename }
    };
    let engines = Arc::new(e.clone());

    // Batch processing through the job manager with parallel workers
    if let Some(wav) = fixture(fx, "tone.wav") {
        let o = out("batch");
        scenario(rows, "Batch: 6 WAV → MP3 in parallel (3 workers)", "WAV ×6", "6 jobs done, 6 MP3 outputs", || {
            let mut inputs = vec![];
            for i in 0..6 {
                let p = sdir.join(format!("batch-{i}.wav"));
                fs::copy(&wav.path, &p).map_err(|e| e.to_string())?;
                inputs.push(p);
            }
            let jm = JobManager::new(engines.clone(), 3, Box::new(|_| {}));
            let ids = jm.submit(JobSpec { inputs, action: Action::Convert(Format::Mp3), options: Options::default(), output: o.clone() })?;
            if ids.len() != 6 {
                return Err(format!("expected 6 jobs, got {}", ids.len()));
            }
            if !jm.wait_idle(Duration::from_secs(120)) {
                return Err("timed out".into());
            }
            let snaps = jm.snapshots();
            let done = snaps.iter().filter(|s| s.status == JobStatus::Done).count();
            let mp3s = fs::read_dir(o.directory.as_ref().unwrap()).unwrap().flatten().filter(|e| detect_path(&e.path()).ok().and_then(|d| d.format) == Some(Format::Mp3)).count();
            jm.shutdown();
            if done == 6 && mp3s == 6 { Ok(format!("{done} done, {mp3s} valid MP3 files")) } else { Err(format!("{done} done, {mp3s} mp3 files; errors: {:?}", snaps.iter().filter_map(|s| s.error.clone()).collect::<Vec<_>>())) }
        });
    }

    // Cancellation of a long-running conversion; output must not exist, temp must be removed
    let long = sdir.join("long.mp4");
    let gen_ok = run(e, EngineId::Ffmpeg, &[os("-hide_banner"), os("-y"), os("-f"), os("lavfi"), os("-i"), os("testsrc2=size=1280x720:rate=30:duration=60"), os("-c:v"), os("libx264"), os("-preset"), os("ultrafast"), long.clone().into()], None, &CancelToken::default(), |_| {}).is_ok();
    if gen_ok {
        let o = out("cancel");
        scenario(rows, "Cancellation of running FFmpeg job", "MP4 60 s 720p → WebM", "status cancelled, no output file, temp folder removed", || {
            let temp_before = temp_entries();
            let jm = JobManager::new(engines.clone(), 1, Box::new(|_| {}));
            let ids = jm.submit(JobSpec { inputs: vec![long.clone()], action: Action::Convert(Format::Webm), options: Options::default(), output: o.clone() })?;
            let t = Instant::now();
            while jm.get(ids[0]).map(|s| s.status) != Some(JobStatus::Running) && t.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(50));
            }
            std::thread::sleep(Duration::from_millis(1500));
            let prog = jm.get(ids[0]).and_then(|s| s.progress);
            jm.cancel(ids[0]);
            jm.wait_idle(Duration::from_secs(20));
            let s = jm.get(ids[0]).unwrap();
            jm.shutdown();
            let files = fs::read_dir(o.directory.as_ref().unwrap()).unwrap().count();
            std::thread::sleep(Duration::from_millis(300));
            let temp_after = temp_entries();
            if s.status != JobStatus::Cancelled {
                return Err(format!("status {:?}", s.status));
            }
            if files != 0 {
                return Err(format!("{files} files left in output folder"));
            }
            if temp_after > temp_before {
                return Err("temp folder not cleaned".into());
            }
            Ok(format!("cancelled after progress {:.0}%, no output, temp cleaned", prog.unwrap_or(0.0) * 100.0))
        });
        // Progress reporting: percentage + ETA appear during a long job
        let o = out("progress");
        scenario(rows, "Progress percentage and ETA reported", "MP4 60 s 720p → MP4 720→240p", "progress increases, ETA present", || {
            let jm = JobManager::new(engines.clone(), 1, Box::new(|_| {}));
            let opts = Options { resolution: Some(240), ..Default::default() };
            let ids = jm.submit(JobSpec { inputs: vec![long.clone()], action: Action::VideoResize, options: opts, output: o.clone() })?;
            let mut seen = vec![];
            let mut eta = None;
            let t = Instant::now();
            while t.elapsed() < Duration::from_secs(120) {
                let s = jm.get(ids[0]).unwrap();
                if let Some(p) = s.progress {
                    seen.push(p);
                }
                eta = eta.or(s.eta_secs);
                if s.status.finished() {
                    if s.status != JobStatus::Done {
                        return Err(format!("{:?}: {:?}", s.status, s.error));
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            jm.shutdown();
            let increasing = seen.windows(2).all(|w| w[1] >= w[0]) && seen.len() > 2;
            if increasing && eta.is_some() { Ok(format!("{} progress samples, ETA reported ({}s)", seen.len(), eta.unwrap())) } else { Err(format!("samples {:?}, eta {eta:?}", seen.len())) }
        });
        let _ = fs::remove_file(&long);
    }

    // Filename collisions: converting twice yields "name (2)"; original untouched
    if let Some(png) = fixture(fx, "sample.png") {
        let o = out("collide");
        scenario(rows, "Filename collision handling", "PNG → JPEG twice", "sample.jpg and sample (2).jpg; source unchanged", || {
            let spec = JobSpec { inputs: vec![png.path.clone()], action: Action::Convert(Format::Jpg), options: Options::default(), output: o.clone() };
            let h = hash_file(&png.path);
            let a = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}).map_err(|e| e.to_string())?;
            let b = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}).map_err(|e| e.to_string())?;
            let (na, nb) = (a.outputs[0].file_name().unwrap().to_string_lossy().to_string(), b.outputs[0].file_name().unwrap().to_string_lossy().to_string());
            if na == "sample.jpg" && nb == "sample (2).jpg" && hash_file(&png.path) == h { Ok(format!("{na}, {nb}")) } else { Err(format!("{na}, {nb}")) }
        });
        let o = out("skip");
        scenario(rows, "Conflict policy: skip existing", "PNG → JPEG with existing target", "job skipped, existing file untouched", || {
            let d = o.directory.clone().unwrap();
            fs::write(d.join("sample.jpg"), b"existing").map_err(|e| e.to_string())?;
            let spec = JobSpec { inputs: vec![png.path.clone()], action: Action::Convert(Format::Jpg), options: Options::default(), output: OutputSettings { conflict: Conflict::Skip, ..o.clone() } };
            let r = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}).map_err(|e| e.to_string())?;
            if r.skipped && fs::read(d.join("sample.jpg")).unwrap() == b"existing" { Ok("skipped; existing file intact".into()) } else { Err("existing file changed or not skipped".into()) }
        });
        scenario(rows, "Default output location: beside original", "PNG → WebP, no output folder", "output in source folder", || {
            let src = sdir.join("beside.png");
            fs::copy(&png.path, &src).map_err(|e| e.to_string())?;
            let spec = JobSpec { inputs: vec![src.clone()], action: Action::Convert(Format::Webp), options: Options::default(), output: OutputSettings::default() };
            let r = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}).map_err(|e| e.to_string())?;
            if r.outputs[0] == sdir.join("beside.webp") { Ok(r.outputs[0].display().to_string()) } else { Err(r.outputs[0].display().to_string()) }
        });
        scenario(rows, "Custom naming template", "PNG → JPEG, template \"{name}-web\"", "sample-web.jpg", || {
            let o = OutputSettings { template: "{name}-web".into(), ..out("template") };
            let spec = JobSpec { inputs: vec![png.path.clone()], action: Action::Convert(Format::Jpg), options: Options::default(), output: o };
            let r = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}).map_err(|e| e.to_string())?;
            let n = r.outputs[0].file_name().unwrap().to_string_lossy().to_string();
            if n == "sample-web.jpg" { Ok(n) } else { Err(n) }
        });
        scenario(rows, "Unsupported conversion is rejected", "PNG → MP3", "rejected before any engine runs", || {
            let spec = JobSpec { inputs: vec![png.path.clone()], action: Action::Convert(Format::Mp3), options: Options::default(), output: out("reject") };
            match execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}) {
                Err(err) => Ok(err.to_string()),
                Ok(_) => Err("conversion was accepted".into()),
            }
        });
        scenario(rows, "Invalid option values are rejected", "PNG resize width 0", "validation error", || {
            let spec = JobSpec { inputs: vec![png.path.clone()], action: Action::ImageResize, options: Options { width: Some(0), ..Default::default() }, output: out("reject") };
            match execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}) {
                Err(err) => Ok(err.to_string()),
                Ok(_) => Err("accepted".into()),
            }
        });
    }

    // Damaged inputs produce meaningful errors and no output
    for (name, action) in [("broken.mp4", Action::Convert(Format::Mp3)), ("broken.png", Action::Convert(Format::Jpg)), ("broken.pdf", Action::Convert(Format::Png))] {
        let p = work.join("fixtures").join(name);
        let o = out(&format!("broken-{name}"));
        scenario(rows, "Damaged input handled", name, "clear error, no output written", || {
            let spec = JobSpec { inputs: vec![p.clone()], action, options: Options::default(), output: o.clone() };
            match execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}) {
                Ok(r) => Err(format!("unexpectedly succeeded: {:?}", r.outputs)),
                Err(err) => {
                    let left = fs::read_dir(o.directory.as_ref().unwrap()).unwrap().count();
                    if left > 0 { Err(format!("{left} partial files left; error was {err}")) } else { Ok(err.to_string()) }
                }
            }
        });
    }

    // Wrong password
    if let Some(locked) = fixture(fx, "locked.pdf") {
        scenario(rows, "PDF unlock with wrong password", "locked.pdf", "'password is incorrect' error", || {
            let spec = JobSpec { inputs: vec![locked.path.clone()], action: Action::PdfUnlock, options: Options { password: Some("wrong".into()), ..Default::default() }, output: out("wrongpw") };
            match execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}) {
                Err(err) if err.to_string().to_lowercase().contains("password") => Ok(err.to_string()),
                Err(err) => Err(format!("unexpected error: {err}")),
                Ok(_) => Err("unlocked with the wrong password".into()),
            }
        });
    }
    if let Some(sz) = fixture(fx, "secret.zip") {
        scenario(rows, "Encrypted ZIP without password", "secret.zip", "error, no prompt/hang", || {
            let spec = JobSpec { inputs: vec![sz.path.clone()], action: Action::ArchiveExtract, options: Options::default(), output: out("nopw") };
            match execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}) {
                Err(err) => Ok(err.to_string()),
                Ok(_) => Err("extracted without password".into()),
            }
        });
    }
    // Video with no audio → audio extraction must fail cleanly
    if let Some(silent) = fixture(fx, "silent.mp4") {
        scenario(rows, "Extract audio from video without audio", "silent.mp4 → MP3", "'no audio track' error", || {
            let spec = JobSpec { inputs: vec![silent.path.clone()], action: Action::Convert(Format::Mp3), options: Options::default(), output: out("noaudio") };
            match execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}) {
                Err(err) if err.to_string().contains("no audio") || err.to_string().contains("not available") => Ok(err.to_string()),
                other => Err(format!("{:?}", other.map(|o| o.outputs))),
            }
        });
    }
    // Detection when extension lies
    if let Some(png) = fixture(fx, "sample.png") {
        scenario(rows, "Content detection beats wrong extension", "PNG renamed to .jpg", "detected as PNG, flagged as mismatch", || {
            let p = sdir.join("liar.jpg");
            fs::copy(&png.path, &p).map_err(|e| e.to_string())?;
            let d = detect_path(&p).map_err(|e| e.to_string())?;
            if d.format == Some(Format::Png) && d.extension_mismatch { Ok("PNG, mismatch flagged".into()) } else { Err(format!("{:?}", d.format)) }
        });
    }
    // Mixed selection shows only shared actions
    scenario(rows, "Mixed selection offers only shared actions", "PNG + MP3", "only archive actions", || {
        let acts = actions_for_selection(&[Input::File(Format::Png), Input::File(Format::Mp3)], e);
        if !acts.is_empty() && acts.iter().all(|a| matches!(a.action, Action::ArchiveCreate(_))) { Ok(format!("{} archive actions", acts.len())) } else { Err(format!("{:?}", acts.iter().map(|a| a.action).collect::<Vec<_>>())) }
    });
    // Folder → archive
    scenario(rows, "Compress a folder to ZIP", "folder", "ZIP containing the folder", || {
        let d = sdir.join("Folder ✓");
        fs::create_dir_all(d.join("sub")).map_err(|e| e.to_string())?;
        fs::write(d.join("a.txt"), "a").unwrap();
        fs::write(d.join("sub").join("b.txt"), "b").unwrap();
        let spec = JobSpec { inputs: vec![d.clone()], action: Action::ArchiveCreate(Format::Zip), options: Options::default(), output: out("folderzip") };
        let r = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {}).map_err(|e| e.to_string())?;
        let z = zip_names(&r.outputs[0]);
        if z.iter().any(|n| n.ends_with("sub/b.txt")) { Ok(format!("{} entries", z.len())) } else { Err(format!("{z:?}")) }
    });
    // Path traversal archive
    scenario(rows, "Archive with ../ entries cannot escape", "evil.zip (../escape.txt)", "nothing written outside the output folder", || {
        let evil = sdir.join("evil.zip");
        write_evil_zip(&evil).map_err(|e| e.to_string())?;
        let o = out("evil");
        let spec = JobSpec { inputs: vec![evil.clone()], action: Action::ArchiveExtract, options: Options::default(), output: o.clone() };
        let r = execute(&spec, e, &CancelToken::default(), &mut |_: Option<f64>, _: &str| {});
        let escaped = sdir.join("escape.txt").exists() || o.directory.as_ref().unwrap().join("escape.txt").exists() || sdir.join("evil").join("escape.txt").exists();
        if escaped { Err("file escaped the extraction folder".into()) } else { Ok(format!("{}", match r { Ok(o) => format!("extracted safely into {}", o.outputs[0].display()), Err(err) => format!("rejected: {err}") })) }
    });
    // Interrupted job recovery
    scenario(rows, "Interrupted job recovery", "history with a running entry", "entry reported as interrupted after restart", || {
        let hp = sdir.join("history.json");
        fs::write(&hp, r#"[{"id":"00000000-0000-0000-0000-000000000001","time":"2026-11-06T10:00:00+02:00","label":"Convert to MP4","inputs":["x.mov"],"outputs":[],"status":"running","error":null,"note":null,"elapsed_ms":0,"spec":null}]"#).unwrap();
        let h = History::load(&hp);
        if h.entries[0].status == "interrupted" { Ok("status = interrupted".into()) } else { Err(h.entries[0].status.clone()) }
    });
    // Temp cleanup after everything
    scenario(rows, "Temporary files cleaned up", "%TEMP%\\Kiln", "no leftover job folders", || {
        let n = temp_entries();
        if n == 0 { Ok("empty".into()) } else { Err(format!("{n} leftover folders")) }
    });
}

fn temp_entries() -> usize {
    fs::read_dir(kiln_core::exec::temp_root()).map(|r| r.count()).unwrap_or(0)
}

fn zip_names(p: &Path) -> Vec<String> {
    let f = fs::File::open(p).unwrap();
    let z = zip::ZipArchive::new(f).unwrap();
    z.file_names().map(|s| s.replace('\\', "/")).collect()
}

fn write_evil_zip(p: &Path) -> std::io::Result<()> {
    // Hand-built stored ZIP containing "../escape.txt".
    let name = b"../escape.txt";
    let data = b"escaped";
    let crc = crc32(data);
    let mut v = vec![];
    v.extend_from_slice(b"PK\x03\x04");
    v.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    v.extend_from_slice(&crc.to_le_bytes());
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(&(name.len() as u16).to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(name);
    v.extend_from_slice(data);
    let cd_off = v.len() as u32;
    let mut cd = vec![];
    cd.extend_from_slice(b"PK\x01\x02");
    cd.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    cd.extend_from_slice(&crc.to_le_bytes());
    cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
    cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
    cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
    cd.extend_from_slice(&[0u8; 12]);
    cd.extend_from_slice(&0u32.to_le_bytes());
    cd.extend_from_slice(name);
    v.extend_from_slice(&cd);
    v.extend_from_slice(b"PK\x05\x06");
    v.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
    v.extend_from_slice(&(cd.len() as u32).to_le_bytes());
    v.extend_from_slice(&cd_off.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    fs::write(p, v)
}

fn crc32(d: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

// ---------------------------------------------------------------- report

fn write_report(e: &Engines, rows: &[Row], path: &Path, total: f64) {
    let pass = rows.iter().filter(|r| r.status == "PASS").count();
    let fail = rows.iter().filter(|r| r.status == "FAIL").count();
    let mut s = String::new();
    s.push_str("# Kiln QA Report\n\n");
    s.push_str(&format!("Generated by `kiln-qa` on {} — {} checks, **{} passed, {} failed**, {:.0} s total.\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M"), rows.len(), pass, fail, total));
    s.push_str("Every row was executed against the real engines on this machine. For each conversion the harness checks: engine exit code, output exists, output is non-empty, output format detected from content matches the target, and (inside the executor) a decode check: `ffprobe` duration for media, `qpdf --show-npages` for PDFs, `magick identify` for images. The source file's hash is compared before and after every run.\n\n");
    s.push_str("## Environment\n\n| Engine | Available | Version / problem |\n|---|---|---|\n");
    for st in e.statuses() {
        s.push_str(&format!("| {} | {} | {} |\n", st.label, if st.available { "yes" } else { "no" }, st.version.clone().or(st.problem.clone()).unwrap_or_default().replace('|', "/")));
    }
    s.push_str(&format!("\nOS: {}\n\n", os_version()));
    if fail > 0 {
        s.push_str("## Failures\n\n| Feature | Input | Error |\n|---|---|---|\n");
        for r in rows.iter().filter(|r| r.status == "FAIL") {
            s.push_str(&format!("| {} | {} | {} |\n", esc(&r.feature), esc(&r.input), esc(&r.error)));
        }
        s.push('\n');
    }
    s.push_str("## All results\n\n| # | Tested feature | Input | Output format | Expected | Actual | Status | Time (s) | Error |\n|---|---|---|---|---|---|---|---|---|\n");
    for (i, r) in rows.iter().enumerate() {
        s.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n", i + 1, esc(&r.feature), esc(&r.input), esc(&r.output), esc(&r.expected), esc(&r.actual), r.status, r.seconds, esc(&r.error)));
    }
    fs::write(path, s).unwrap();
    fs::write(path.with_file_name("qa-results.json"), serde_json::to_vec_pretty(rows).unwrap()).unwrap();
}

fn esc(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn os_version() -> String {
    std::process::Command::new("cmd").args(["/c", "ver"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}
