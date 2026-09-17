//! Shared Pico-8 golden harness for image and sfx carts.

mod audio_ogg;

use audio_ogg::{load_ogg, load_wav, write_ogg};
use nano9::pico8::audio::{Note, Pico8Note, Sfx};
use nano9::pico8::{Cart, CartLoaderSettings};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Pico-8 tracker tick: 183 samples at 22050 Hz.
const SAMPLES_PER_TICK: usize = 183;
const EXPORT_NOTES: usize = 32;
/// Pico-8 `EXPORT %d.wav` leftover oscillator at slot 0. Not used for playback.
const PICO8_EXPORT_OSC_PHASE: f32 = 0.39;
/// Samples with |s| <= this count as Pico-8's EXPORT lead-in pad.
const EXPORT_PAD_THRESHOLD: i16 = 64;

pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

pub fn sfx_dir() -> PathBuf {
    golden_dir().join("sfx")
}

pub fn synth_dir() -> PathBuf {
    golden_dir().join("synth")
}

#[allow(dead_code)]
pub fn run_image_suite() {
    let mut rows = Vec::new();
    let mut failed = Vec::new();
    collect_suite(
        &golden_dir(),
        Kind::Image,
        "tests/golden/*.p8",
        &mut rows,
        &mut failed,
    );
    finish_suite(rows, failed);
}

#[allow(dead_code)]
pub fn run_sfx_suite() {
    let mut rows = Vec::new();
    let mut failed = Vec::new();
    collect_suite(
        &sfx_dir(),
        Kind::Audio,
        "tests/golden/sfx/*.p8",
        &mut rows,
        &mut failed,
    );
    collect_export_cart(&synth_dir(), "synth.p8", "synth", &mut rows, &mut failed);
    collect_export_cart(&synth_dir(), "oscillator.p8", "osc", &mut rows, &mut failed);
    check_pico8_export_phase_walk();
    finish_suite(rows, failed);
}

#[allow(dead_code)]
enum Kind {
    Image,
    Audio,
}

enum CartResult {
    Match,
    Differ { detail: String, compare: PathBuf },
}

fn collect_suite(
    dir: &Path,
    kind: Kind,
    glob: &str,
    rows: &mut Vec<(String, String)>,
    failed: &mut Vec<String>,
) {
    let all = cart_names(dir);
    assert!(!all.is_empty(), "no {glob} carts found");
    let names: Vec<String> = match filter_arg() {
        Some(filter) => all
            .into_iter()
            .filter(|name| name.contains(&filter))
            .collect(),
        None => all,
    };
    for name in &names {
        let result = match kind {
            Kind::Image => check_image_cart(dir, name),
            Kind::Audio => check_audio_cart(dir, name),
        };
        push_result(rows, failed, name, result);
    }
}

fn finish_suite(rows: Vec<(String, String)>, failed: Vec<String>) {
    if rows.is_empty() {
        println!("0 golden carts matched");
        return;
    }
    let width = rows.iter().map(|(n, _)| n.len()).max().unwrap_or(8);
    println!("\n=== golden summary ===");
    for (name, status) in &rows {
        println!("  {name:<width$}  {status}");
    }
    let _ = std::io::stdout().flush();

    if !failed.is_empty() {
        panic!(
            "{} / {} carts differ: {}",
            failed.len(),
            rows.len(),
            failed.join(", ")
        );
    }
}

fn push_result(
    rows: &mut Vec<(String, String)>,
    failed: &mut Vec<String>,
    name: &str,
    result: Result<CartResult, String>,
) {
    match result {
        Ok(CartResult::Match) => rows.push((name.to_string(), "match".into())),
        Ok(CartResult::Differ { detail, compare }) => {
            rows.push((name.to_string(), format!("{detail}  {}", compare.display())));
            failed.push(name.to_string());
        }
        Err(err) => {
            rows.push((name.to_string(), format!("error: {err}")));
            failed.push(name.to_string());
        }
    }
}

fn cart_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "p8") {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

fn check_image_cart(dir: &Path, name: &str) -> Result<CartResult, String> {
    let cart = dir.join(format!("{name}.p8"));
    let expected = dir.join(format!("{name}-expected.png"));
    let written = dir.join(format!("{name}.png"));
    let actual = dir.join(format!("{name}-actual.png"));

    if !cart.exists() {
        return Err(format!("missing cart {}", cart.display()));
    }
    if !expected.exists() {
        return Err(format!(
            "missing golden {} (run bin/golden-pico8)",
            expected.display()
        ));
    }

    let _ = fs::remove_file(&written);
    let _ = fs::remove_file(&actual);
    run_n9(&cart, dir)?;
    if written.exists() {
        fs::rename(&written, &actual).map_err(|e| e.to_string())?;
    }
    if !actual.exists() {
        return Err(format!("n9 did not write {}", actual.display()));
    }

    let (ew, eh, ebytes) = load_rgb_png(&expected);
    let (aw, ah, abytes) = load_rgb_png(&actual);
    if (ew, eh) != (aw, ah) {
        println!("\n=== {name}: size Pico-8 {ew}x{eh} vs Nano-9 {aw}x{ah} ===");
        show_image("Pico-8", &expected);
        show_image("Nano-9", &actual);
        return Err(format!("size {ew}x{eh} vs {aw}x{ah}"));
    }
    if ebytes == abytes {
        return Ok(CartResult::Match);
    }

    let (diff, changed) = make_diff(&ebytes, &abytes);
    let diff_path = dir.join(format!("{name}-diff.png"));
    write_rgb_png(&diff_path, ew, eh, &diff);
    let (cw, ch, compare) = compose_compare(&ebytes, &abytes, &diff, ew, eh);
    let compare_path = dir.join(format!("{name}-compare.png"));
    write_rgb_png(&compare_path, cw, ch, &compare);
    let total = (ew * eh) as usize;
    let pct = (changed as f64) * 100.0 / total as f64;
    println!("\n=== {name}: {changed}/{total} pixels differ ({pct:.2}%) ===");
    println!("left = Pico-8 (green bar), middle = Nano-9 (orange bar), right = magenta mismatches");
    println!("Pico-8:  {}", expected.display());
    println!("Nano-9:  {}", actual.display());
    println!("diff:    {}", diff_path.display());
    println!("compare: {}", compare_path.display());
    let _ = std::io::stdout().flush();
    show_image(&format!("{name}: Pico-8 | Nano-9 | diff"), &compare_path);
    Ok(CartResult::Differ {
        detail: format!("{changed}/{total} differ ({pct:.2}%)"),
        compare: compare_path,
    })
}

fn check_audio_cart(dir: &Path, name: &str) -> Result<CartResult, String> {
    let cart = dir.join(format!("{name}.p8"));
    let expected = dir.join(format!("{name}-expected.ogg"));
    let written = dir.join(format!("{name}.wav"));
    let actual_wav = dir.join(format!("{name}-actual.wav"));
    let actual = dir.join(format!("{name}-actual.ogg"));

    if !cart.exists() {
        return Err(format!("missing cart {}", cart.display()));
    }
    if !expected.exists() {
        return Err(format!(
            "missing golden {} (run make golden)",
            expected.display()
        ));
    }

    let _ = fs::remove_file(&written);
    let _ = fs::remove_file(&actual_wav);
    let _ = fs::remove_file(&actual);
    run_n9(&cart, dir)?;
    let wav = if written.exists() {
        written.clone()
    } else {
        actual_wav.clone()
    };
    if !wav.exists() {
        return Err(format!("n9 did not write {}", written.display()));
    }

    let pcm = load_wav(&wav);
    write_ogg(&actual, &pcm)?;
    let _ = fs::remove_file(&written);
    let _ = fs::remove_file(&actual_wav);

    let expected_pcm = load_ogg(&expected);
    let actual_pcm = load_ogg(&actual);
    let note_len = SAMPLES_PER_TICK * 8; // playback carts use speed 8 unless noted
    Ok(compare_wav(
        name,
        dir,
        &expected,
        &actual,
        &expected_pcm,
        &actual_pcm,
        note_len,
        true,
        true,
        None,
    ))
}

fn collect_export_cart(
    dir: &Path,
    cart_name: &str,
    prefix: &str,
    rows: &mut Vec<(String, String)>,
    failed: &mut Vec<String>,
) {
    let cart_path = dir.join(cart_name);
    if !cart_path.exists() {
        return;
    }
    let source = match fs::read_to_string(&cart_path) {
        Ok(s) => s,
        Err(err) => {
            push_result(rows, failed, prefix, Err(err.to_string()));
            return;
        }
    };
    let cart = match Cart::from_str(&source, &CartLoaderSettings::default()) {
        Ok(cart) => cart,
        Err(err) => {
            push_result(rows, failed, prefix, Err(err.to_string()));
            return;
        }
    };
    let names = parse_synth_names(&cart.lua);
    let filter = filter_arg();
    let phases = pico8_export_phases(&cart.sfx, PICO8_EXPORT_OSC_PHASE);
    let mut any = false;
    for (index, sfx) in cart.sfx.iter().enumerate() {
        let expected = dir.join(format!("{prefix}-{index:02}-expected.ogg"));
        let name = names
            .get(&index)
            .cloned()
            .unwrap_or_else(|| "slot".to_string());
        let label = format!("{prefix}-{index:02}-{name}");
        let wanted = match &filter {
            None => true,
            Some(f) => label.contains(f) || prefix.contains(f.as_str()),
        };
        if expected.exists() && wanted {
            any = true;
            let phase = phases.get(index).copied().unwrap_or(PICO8_EXPORT_OSC_PHASE);
            let result = check_synth_slot(dir, prefix, index, &name, sfx, phase);
            push_result(rows, failed, &label, result);
        }
    }
    if !any && filter.is_none() {
        println!("no {prefix}-NN-expected.ogg");
    }
}

fn parse_synth_names(lua: &str) -> std::collections::HashMap<usize, String> {
    let mut names = std::collections::HashMap::new();
    for line in lua.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("-- sfx ") else {
            continue;
        };
        let Some((idx, name)) = rest.split_once(':') else {
            continue;
        };
        let Ok(index) = idx.trim().parse::<usize>() else {
            continue;
        };
        let name = name.trim();
        if !name.is_empty() {
            names.insert(index, name.to_string());
        }
    }
    names
}

fn check_synth_slot(
    dir: &Path,
    prefix: &str,
    index: usize,
    name: &str,
    sfx: &Sfx,
    phase: f32,
) -> Result<CartResult, String> {
    let expected = dir.join(format!("{prefix}-{index:02}-expected.ogg"));
    let actual = dir.join(format!("{prefix}-{index:02}-actual.ogg"));
    // Nano-9 starts at sample 0. Pico-8 EXPORT often writes a silent pad
    // first (osc-02 is 93 samples / 0.0042s). Keep that pad on expected and
    // strip it only when comparing. Encode both sides so Vorbis error is
    // symmetric.
    let pcm = render_sfx(sfx, phase);
    write_ogg(&actual, &pcm)?;
    let expected_pcm = load_ogg(&expected);
    let actual_pcm = load_ogg(&actual);
    let note_len = sfx.speed.max(1) as usize * SAMPLES_PER_TICK;
    Ok(compare_wav(
        &format!("{prefix}-{index:02}-{name}"),
        dir,
        &expected,
        &actual,
        &expected_pcm,
        &actual_pcm,
        note_len,
        true,
        false,
        Some(phase),
    ))
}

fn render_sfx(sfx: &Sfx, phase: f32) -> Vec<i16> {
    let export_len = EXPORT_NOTES * sfx.speed.max(1) as usize * SAMPLES_PER_TICK;
    let mut pcm: Vec<i16> = sfx
        .decode_with_phase(phase)
        .take(export_len)
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();
    pcm.resize(export_len, 0);
    pcm
}

/// Pico-8 `EXPORT %d.wav` leftover oscillator after one 32-note slot.
///
/// Empty tail notes keep the last sounding key at volume 0. Measured on 64
/// identical triangle-scale exports: Δφ ≈ 0.125 per slot, repeating every 8.
fn pico8_phase_after_export(sfx: &Sfx, phase: f32) -> f32 {
    let mut sfx = sfx.clone();
    if let Some(last) = sfx.notes.iter().rev().copied().find(|n| n.volume() > 0.0) {
        let held = Pico8Note(last.key() as u16);
        sfx.notes.truncate(EXPORT_NOTES);
        sfx.notes.resize(EXPORT_NOTES, held);
    }
    let n = EXPORT_NOTES * sfx.speed.max(1) as usize * SAMPLES_PER_TICK;
    let mut decoder = sfx.decode_with_phase(phase);
    for _ in 0..n {
        if decoder.next().is_none() {
            break;
        }
    }
    decoder.phase()
}

fn pico8_export_phases<'a, I>(sfxs: I, phase0: f32) -> Vec<f32>
where
    I: IntoIterator<Item = &'a Sfx>,
{
    let mut phase = phase0.fract().rem_euclid(1.0);
    let mut out = Vec::new();
    for sfx in sfxs {
        out.push(phase);
        phase = pico8_phase_after_export(sfx, phase);
    }
    out
}

fn check_pico8_export_phase_walk() {
    // Same triangle scale as tests/golden/synth/oscillator.p8.
    let sfx = Sfx::try_from(
        "000800000d0700f070110701207014070160701807019070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();
    let delta = (pico8_phase_after_export(&sfx, 0.39) - 0.39).rem_euclid(1.0);
    assert!(
        (delta - 0.125).abs() < 0.002,
        "Pico-8 EXPORT Δφ={delta} (expected ~0.125)"
    );
    let phases = pico8_export_phases(std::iter::repeat(&sfx).take(16), PICO8_EXPORT_OSC_PHASE);
    assert!((phases[0] - PICO8_EXPORT_OSC_PHASE).abs() < 0.001);
    assert!(((phases[1] - phases[0]).rem_euclid(1.0) - 0.125).abs() < 0.002);
    assert!(
        (phases[8] - phases[0]).abs() < 0.02,
        "slot 8 should match slot 0, got {} vs {}",
        phases[8],
        phases[0]
    );
}

fn compare_wav(
    name: &str,
    dir: &Path,
    expected_path: &Path,
    actual_path: &Path,
    expected_pcm: &[i16],
    actual_pcm: &[i16],
    note_len: usize,
    align_expected_onset: bool,
    align_actual_onset: bool,
    osc_phase: Option<f32>,
) -> CartResult {
    let e_on = if align_expected_onset {
        onset_index(expected_pcm, EXPORT_PAD_THRESHOLD).unwrap_or(0)
    } else {
        0
    };
    let a_on = if align_actual_onset {
        onset_index(actual_pcm, EXPORT_PAD_THRESHOLD).unwrap_or(0)
    } else {
        0
    };
    let aligned_e = &expected_pcm[e_on..];
    let aligned_a = &actual_pcm[a_on..];
    if aligned_e == aligned_a {
        return CartResult::Match;
    }

    let n = aligned_e.len().min(aligned_a.len());
    let e_cmp = &aligned_e[..n];
    let a_cmp = &aligned_a[..n];
    let mut diffs = Vec::with_capacity(n);
    let mut changed = 0usize;
    const SAMPLE_EPS: i16 = 256;
    for (e, a) in e_cmp.iter().zip(a_cmp.iter()) {
        let d = (*e as i32 - *a as i32).clamp(-32767, 32767) as i16;
        diffs.push(d);
        if d.abs() > SAMPLE_EPS {
            changed += 1;
        }
    }
    let rms_diff = rms(&diffs);
    let mut pitch_rows = String::new();
    let mut i = 0;
    let mut note_i = 0;
    let step = note_len.max(1);
    while i < n {
        let end = (i + step).min(n);
        let e_hz = zero_cross_hz(&e_cmp[i..end], 22_050.0);
        let a_hz = zero_cross_hz(&a_cmp[i..end], 22_050.0);
        pitch_rows.push_str(&format!(
            "  note {note_i}: Pico-8 {e_hz:.1} Hz / {e_rms:.0} rms, Nano-9 {a_hz:.1} Hz / {a_rms:.0} rms\n",
            e_rms = rms(&e_cmp[i..end]),
            a_rms = rms(&a_cmp[i..end]),
        ));
        note_i += 1;
        i = end;
        if note_i >= 16 {
            break;
        }
    }

    let (lo, hi) = wave_focus(aligned_e, aligned_a);
    let e_win = &aligned_e[lo.min(aligned_e.len())..hi.min(aligned_e.len())];
    let a_win = &aligned_a[lo.min(aligned_a.len())..hi.min(aligned_a.len())];
    let (cw, ch, compare) = compose_wave_compare(e_win, a_win);
    let compare_path = dir.join(format!("{name}-compare.png"));
    write_rgb_png(&compare_path, cw, ch, &compare);
    let pct = if n == 0 {
        100.0
    } else {
        (changed as f64) * 100.0 / n as f64
    };
    let onset_delta = a_on as i64 - e_on as i64;
    let len_note = if aligned_e.len() == aligned_a.len() {
        String::new()
    } else {
        format!(", len {} vs {}", aligned_e.len(), aligned_a.len())
    };
    println!("\n=== {name}: {changed}/{n} samples |diff|>{SAMPLE_EPS} ({pct:.2}%) ===");
    println!(
        "onset Pico-8 {e_on} ({:.4}s pad) Nano-9 {a_on} (delta {onset_delta}), lens {} vs {}, RMS diff {rms_diff:.1}, peak {} vs {}{len_note}",
        e_on as f64 / 22_050.0,
        aligned_e.len(),
        aligned_a.len(),
        peak(aligned_e),
        peak(aligned_a)
    );
    match osc_phase {
        Some(phase) => println!("waveform window samples {lo}..{hi}; oscillator phase {phase:.2}"),
        None => println!("waveform window samples {lo}..{hi}"),
    }
    print!("{pitch_rows}");
    println!("Pico-8:  {}", expected_path.display());
    println!("Nano-9:  {}", actual_path.display());
    println!("compare: {}", compare_path.display());
    let _ = std::io::stdout().flush();
    show_image(&format!("{name}: Pico-8 | Nano-9 | diff"), &compare_path);
    CartResult::Differ {
        detail: format!(
            "{changed}/{n} |d|>{SAMPLE_EPS} ({pct:.2}%), RMS {rms_diff:.1}, onset {onset_delta}"
        ),
        compare: compare_path,
    }
}

/// Positional args after skipping libtest flags. First one is the substring filter.
///
/// `cargo test sfx` passes `sfx` into every test binary. If that matches this
/// crate's name, treat it as "run this suite" rather than a cart filter.
fn filter_arg() -> Option<String> {
    let crate_name = env!("CARGO_CRATE_NAME");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--" {
            continue;
        }
        if arg.starts_with('-') {
            if !arg.contains('=')
                && matches!(
                    arg.as_str(),
                    "--test-threads"
                        | "--color"
                        | "--skip"
                        | "--format"
                        | "--ensure-time"
                        | "--report-time"
                )
            {
                let _ = args.next();
            }
            continue;
        }
        if arg == crate_name {
            continue;
        }
        return Some(arg);
    }
    None
}

fn load_rgb_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = File::open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let decoder = png::Decoder::new(file);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|e| panic!("png {}: {e}", path.display()));
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("frame {}: {e}", path.display()));
    let width = info.width;
    let height = info.height;
    let rgb = match info.color_type {
        png::ColorType::Rgb => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgba => buf[..info.buffer_size()]
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Grayscale => buf[..info.buffer_size()]
            .iter()
            .flat_map(|g| [*g, *g, *g])
            .collect(),
        other => panic!("{}: unsupported color type {other:?}", path.display()),
    };
    (width, height, rgb)
}

fn write_rgb_png(path: &Path, width: u32, height: u32, rgb: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let file = File::create(path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(rgb).unwrap();
}

fn onset_index(samples: &[i16], threshold: i16) -> Option<usize> {
    samples.iter().position(|s| s.abs() > threshold)
}

fn rms(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    (sum / samples.len() as f64).sqrt()
}

fn peak(samples: &[i16]) -> i16 {
    samples.iter().map(|s| s.abs()).max().unwrap_or(0)
}

fn zero_cross_hz(samples: &[i16], rate: f64) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mut crosses = 0usize;
    for pair in samples.windows(2) {
        if pair[0] == 0 || pair[0].signum() != pair[1].signum() && pair[1] != 0 {
            if pair[0] != 0 && pair[1] != 0 {
                crosses += 1;
            }
        }
    }
    (crosses as f64) * rate / (2.0 * samples.len() as f64)
}

fn wave_focus(expected: &[i16], actual: &[i16]) -> (usize, usize) {
    const WIN: usize = 512;
    const PRE: usize = 16;
    let n = expected.len().max(actual.len());
    if n == 0 {
        return (0, 0);
    }
    let mut onset = 0usize;
    for i in 0..n {
        let e = *expected.get(i).unwrap_or(&0);
        let a = *actual.get(i).unwrap_or(&0);
        if e.abs() > 64 || a.abs() > 64 {
            onset = i;
            break;
        }
    }
    let mut diverge = onset;
    for i in onset..n {
        let e = *expected.get(i).unwrap_or(&0) as i32;
        let a = *actual.get(i).unwrap_or(&0) as i32;
        if (e - a).abs() > 512 {
            diverge = i;
            break;
        }
    }
    let start = diverge.saturating_sub(PRE);
    let end = (start + WIN).min(n);
    (start, end)
}

fn waveform_strip(samples: &[i16], w: u32, h: u32, color: [u8; 3]) -> Vec<u8> {
    let mut rgb = vec![16u8; (w * h * 3) as usize];
    let mid = h as i32 / 2;
    for x in 0..w {
        put_pixel(&mut rgb, w, x, mid as u32, [32, 32, 32]);
    }
    let n = samples.len().max(1);
    if n <= w as usize * 2 {
        let mut prev: Option<(i32, i32)> = None;
        for (i, sample) in samples.iter().enumerate() {
            let x = if n <= 1 {
                0
            } else {
                (i as u32) * (w - 1) / (n as u32 - 1)
            } as i32;
            let y = (mid - (*sample as i32) * mid / 32767).clamp(0, h as i32 - 1);
            if let Some((px, py)) = prev {
                draw_line(&mut rgb, w, px, py, x, y, color);
            } else {
                put_pixel(&mut rgb, w, x as u32, y as u32, color);
            }
            prev = Some((x, y));
        }
    } else {
        for x in 0..w {
            let a = (x as usize) * n / w as usize;
            let b = ((x as usize + 1) * n / w as usize).max(a + 1).min(n);
            let (mn, mx) = samples[a..b]
                .iter()
                .fold((0i16, 0i16), |acc, s| (acc.0.min(*s), acc.1.max(*s)));
            let y0 = (mid - (mx as i32) * mid / 32767).clamp(0, h as i32 - 1) as u32;
            let y1 = (mid - (mn as i32) * mid / 32767).clamp(0, h as i32 - 1) as u32;
            let (lo, hi) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
            for y in lo..=hi {
                put_pixel(&mut rgb, w, x, y, color);
            }
        }
    }
    rgb
}

fn draw_line(
    rgb: &mut [u8],
    stride: u32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 {
            put_pixel(rgb, stride, x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn compose_wave_compare(expected: &[i16], actual: &[i16]) -> (u32, u32, Vec<u8>) {
    const W: u32 = 512;
    const H: u32 = 80;
    const GAP: u32 = 6;
    const BAR: u32 = 8;
    const BARS: [[u8; 3]; 3] = [[0, 231, 86], [255, 163, 0], [255, 0, 255]];
    let n = expected.len().max(actual.len());
    let mut diff = vec![0i16; n];
    for i in 0..n {
        let e = *expected.get(i).unwrap_or(&0) as i32;
        let a = *actual.get(i).unwrap_or(&0) as i32;
        diff[i] = (e - a).clamp(-32767, 32767) as i16;
    }
    let panels = [
        waveform_strip(expected, W, H, [0, 231, 86]),
        waveform_strip(actual, W, H, [255, 163, 0]),
        waveform_strip(&diff, W, H, [255, 0, 255]),
    ];
    let out_w = W;
    let out_h = (BAR + H) * 3 + GAP * 2;
    let mut out = vec![0u8; (out_w * out_h * 3) as usize];
    for (i, panel) in panels.iter().enumerate() {
        let oy = (BAR + H + GAP) * i as u32;
        for y in 0..BAR {
            for x in 0..W {
                put_pixel(&mut out, out_w, x, oy + y, BARS[i]);
            }
        }
        for y in 0..H {
            for x in 0..W {
                let si = ((y * W + x) * 3) as usize;
                put_pixel(
                    &mut out,
                    out_w,
                    x,
                    oy + BAR + y,
                    [panel[si], panel[si + 1], panel[si + 2]],
                );
            }
        }
    }
    (out_w, out_h, out)
}

fn make_diff(a: &[u8], b: &[u8]) -> (Vec<u8>, usize) {
    let n = a.len().min(b.len());
    let mut diff = vec![0u8; n];
    let mut changed = 0usize;
    for i in (0..n).step_by(3) {
        if a[i] != b[i] || a[i + 1] != b[i + 1] || a[i + 2] != b[i + 2] {
            changed += 1;
            diff[i] = 255;
            diff[i + 1] = 0;
            diff[i + 2] = 255;
        } else {
            diff[i] = a[i] / 3;
            diff[i + 1] = a[i + 1] / 3;
            diff[i + 2] = a[i + 2] / 3;
        }
    }
    (diff, changed)
}

fn put_pixel(out: &mut [u8], stride: u32, x: u32, y: u32, rgb: [u8; 3]) {
    let i = ((y * stride + x) * 3) as usize;
    out[i] = rgb[0];
    out[i + 1] = rgb[1];
    out[i + 2] = rgb[2];
}

/// Pico-8 | Nano-9 | magenta mismatches, nearest-neighbor scaled so pixels read.
fn compose_compare(
    expected: &[u8],
    actual: &[u8],
    diff: &[u8],
    w: u32,
    h: u32,
) -> (u32, u32, Vec<u8>) {
    const SCALE: u32 = 3;
    const GAP: u32 = 6;
    const BAR: u32 = 8;
    const BARS: [[u8; 3]; 3] = [[0, 231, 86], [255, 163, 0], [255, 0, 255]];
    let pw = w * SCALE;
    let ph = h * SCALE;
    let out_w = pw * 3 + GAP * 2;
    let out_h = BAR + ph;
    let mut out = vec![0u8; (out_w * out_h * 3) as usize];
    let panels = [expected, actual, diff];
    for (i, panel) in panels.iter().enumerate() {
        let ox = (pw + GAP) * i as u32;
        for y in 0..BAR {
            for x in 0..pw {
                put_pixel(&mut out, out_w, ox + x, y, BARS[i]);
            }
        }
        for y in 0..ph {
            let sy = y / SCALE;
            for x in 0..pw {
                let sx = x / SCALE;
                let si = ((sy * w + sx) * 3) as usize;
                put_pixel(
                    &mut out,
                    out_w,
                    ox + x,
                    BAR + y,
                    [panel[si], panel[si + 1], panel[si + 2]],
                );
            }
        }
    }
    (out_w, out_h, out)
}

fn open_tty() -> Option<File> {
    File::options().write(true).open("/dev/tty").ok()
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Show a PNG in WezTerm. Writes to `/dev/tty` so `cargo test` capture cannot swallow it.
fn show_image(label: &str, path: &Path) {
    let Some(mut tty) = open_tty() else {
        return;
    };
    let _ = writeln!(tty, "{label}");
    let _ = tty.flush();
    let shown = Command::new("wezterm")
        .args(["imgcat", "--width", "80%", "--resample-filter", "nearest"])
        .arg(path)
        .stdout(tty)
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if shown {
        return;
    }
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    if let Some(mut tty) = open_tty() {
        let b64 = base64_encode(&bytes);
        let _ = write!(
            tty,
            "\x1b]1337;File=inline=1;width=80%;height=auto;preserveAspectRatio=1:{b64}\x07\n"
        );
        let _ = tty.flush();
    }
}

fn run_n9(cart: &Path, actual_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(actual_dir).map_err(|e| e.to_string())?;
    let n9 = option_env!("CARGO_BIN_EXE_n9").unwrap_or("n9");
    let mut child = Command::new(n9)
        .args([
            "run",
            "--headless",
            "-p",
            "headless",
            cart.to_str().unwrap(),
        ])
        .env("NANO9_SCREENSHOT_DIR", actual_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn n9: {e}"))?;

    let stderr = child.stderr.take();
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut err) = stderr {
            use std::io::Read;
            let _ = err.read_to_string(&mut buf);
        }
        buf
    });

    let timeout = Duration::from_secs(40);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stderr = stderr_thread.join().unwrap_or_default();
                if status.success() {
                    return Ok(());
                }
                return Err(format!("n9 exited {status}: {stderr}"));
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let stderr = stderr_thread.join().unwrap_or_default();
                    let tail: String = stderr
                        .chars()
                        .rev()
                        .take(1500)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    return Err(format!("n9 timed out after {timeout:?}: {tail}"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("wait n9: {e}")),
        }
    }
}
