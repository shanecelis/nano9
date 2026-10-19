//! Criterion-style baseline for synth wave fits.
//!
//! Each steady note records how far Nano-9 sits from the Pico-8 export:
//! frequency (cents), amplitude (gain ratio), and phase (samples), plus the
//! Pico-8 residual after that alignment. `--save-baseline` writes
//! `tests/golden/synth/wave-baseline.json`. `--save-baseline NAME` writes
//! `wave-baseline-NAME.json`, and `--baseline NAME` compares against it.
//! An unknown `--baseline` name fails the run. A drop is a closer match.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;
const FREQ_EPS: f64 = 0.05;
const AMP_EPS: f64 = 0.002;
const PHASE_EPS: f64 = 0.25;
const SHAPE_EPS: f64 = 5.0;
const NOTE_FREQ_EPS: f64 = 0.2;
const NOTE_AMP_EPS: f64 = 0.005;
const NOTE_PHASE_EPS: f64 = 0.3;
const NOTE_SHAPE_EPS: f64 = 5.0;
/// One tracker tick is 183 samples. Speed-1 notes are about one cycle, so the
/// frequency fit wanders. The pooled steady score keeps notes at least 4 ticks long.
const MIN_STEADY_SAMPLES: usize = 183 * 4;

#[derive(Clone, Serialize, Deserialize)]
pub struct NoteMatch {
    pub index: usize,
    pub wave: String,
    pub kind: String,
    pub cents: f64,
    pub pico_cents: f64,
    pub nano_cents: f64,
    pub amp_ratio: f64,
    pub phase_samples: f64,
    pub pico_shape_rms: f64,
    pub nano_shape_rms: f64,
    pub samples: usize,
}

#[derive(Clone)]
pub struct SlotWave {
    pub id: String,
    pub notes: Vec<NoteMatch>,
}

#[derive(Serialize, Deserialize)]
struct BaselineFile {
    version: u32,
    description: String,
    slots: BTreeMap<String, Vec<NoteMatch>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    Improved,
    Regressed,
    Same,
}

impl Change {
    fn label(self) -> &'static str {
        match self {
            Change::Improved => "improved",
            Change::Regressed => "regressed",
            Change::Same => "same",
        }
    }
}

struct Summary {
    n: usize,
    freq: f64,
    amp: f64,
    phase: f64,
    shape: f64,
}

/// `--baseline NAME` must name an existing `wave-baseline-NAME.json`.
pub fn require_known_baseline(dir: &Path) {
    match flag_state("--baseline") {
        Flag::Absent => {}
        Flag::Present => panic!("`--baseline` needs a name"),
        Flag::Invalid(err) => panic!("{err}"),
        Flag::Value(name) => {
            let path = baseline_file(dir, Some(&name));
            if !path.is_file() {
                panic!("{}", unrecognized_baseline(dir, &name));
            }
        }
    }
}

fn unrecognized_baseline(dir: &Path, name: &str) -> String {
    let known = known_baseline_names(dir);
    if known.is_empty() {
        format!("unrecognized baseline `{name}`")
    } else {
        format!(
            "unrecognized baseline `{name}`\nknown baselines: {}",
            known.join(", ")
        )
    }
}

fn known_baseline_names(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return names;
    };
    for entry in entries.flatten() {
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Some(stem) = file_name
            .strip_prefix("wave-baseline-")
            .and_then(|rest| rest.strip_suffix(".json"))
        else {
            continue;
        };
        if baseline_name(stem).is_ok() {
            names.push(stem.to_string());
        }
    }
    names.sort();
    names
}

pub fn report_wave_baseline(dir: &Path, slots: &[SlotWave]) {
    if slots.is_empty() {
        return;
    }
    let current = quantize_slots(slots);
    let selection = selection(dir);
    for err in &selection.errors {
        println!("{err}");
    }
    let loaded = selection
        .compare
        .as_ref()
        .map(|path| (path.clone(), load_baseline(path)));
    match &loaded {
        Some((path, loaded)) => print_report(path, &current, loaded),
        None => {
            println!("\n=== synth wave match ===");
            print_absolute(&current);
        }
    }
    if let Some(path) = selection.save {
        let for_save = if loaded.as_ref().is_some_and(|(compare, _)| compare == &path) {
            loaded.unwrap().1
        } else {
            load_baseline(&path)
        };
        save_baseline(&path, &current, for_save);
    }
    let _ = io::stdout().flush();
}

struct Selection {
    compare: Option<PathBuf>,
    save: Option<PathBuf>,
    errors: Vec<String>,
}

fn selection(dir: &Path) -> Selection {
    let mut errors = Vec::new();
    let (baseline_name, baseline_ok) = match flag_state("--baseline") {
        Flag::Absent => (None, true),
        Flag::Present => {
            errors.push("`--baseline` needs a name".to_string());
            (None, false)
        }
        Flag::Value(name) => (Some(name), true),
        Flag::Invalid(err) => {
            errors.push(err);
            (None, false)
        }
    };
    let save_name = match flag_state("--save-baseline") {
        Flag::Absent => None,
        Flag::Present => Some(None),
        Flag::Value(name) => Some(Some(name)),
        Flag::Invalid(err) => {
            errors.push(err);
            None
        }
    };
    let compare = if baseline_ok {
        let name = baseline_name.or(match &save_name {
            Some(Some(name)) => Some(name.clone()),
            _ => None,
        });
        Some(baseline_file(dir, name.as_deref()))
    } else {
        None
    };
    let save = match save_name {
        None => None,
        Some(Some(name)) => Some(baseline_file(dir, Some(&name))),
        Some(None) => compare.clone(),
    };
    Selection {
        compare,
        save,
        errors,
    }
}

fn baseline_file(dir: &Path, name: Option<&str>) -> PathBuf {
    match name {
        Some(name) => dir.join(format!("wave-baseline-{name}.json")),
        None => dir.join("wave-baseline.json"),
    }
}

fn save_baseline(path: &Path, current: &[SlotWave], loaded: Loaded) {
    let mut file = match loaded {
        Loaded::Corrupt(err) => {
            println!("left {} in place: {err}", path.display());
            return;
        }
        Loaded::Missing => empty_file(),
        Loaded::Ready(file) => file,
    };
    let updated = current.len();
    for slot in current {
        file.slots.insert(slot.id.clone(), slot.notes.clone());
    }
    file.version = VERSION;
    file.description = description().to_string();
    match write_baseline(path, &file) {
        Ok(()) => println!("saved {} ({} slots in this run)", path.display(), updated),
        Err(err) => println!("could not save {}: {err}", path.display()),
    }
}

fn description() -> &'static str {
    "Pico-8 export vs Nano-9 synth fits. Lower frequency, amplitude, phase, and shape values are a closer match."
}

fn print_report(path: &Path, current: &[SlotWave], loaded: &Loaded) {
    println!("\n=== synth wave match ===");
    println!("Lower is a closer Pico-8 match.");
    println!("frequency   RMS cents between the two fits");
    println!("amplitude   mean |gain ratio - 1|");
    println!("phase       RMS sample lag between the two fits");
    println!("shape       mean Pico-8 residual after aligning phase and gain");
    println!("The steady pool keeps notes at least 4 tracker ticks long.");
    match loaded {
        Loaded::Corrupt(err) => {
            println!("baseline {} could not be read: {err}", path.display());
            print_absolute(current);
        }
        Loaded::Missing => {
            println!("no baseline at {}", path.display());
            print_absolute(current);
            if !saving() {
                println!("save this run with: {}", save_hint());
            }
        }
        Loaded::Ready(file) => {
            if file.version != VERSION {
                println!(
                    "baseline {} is version {}, this harness writes version {VERSION}",
                    path.display(),
                    file.version
                );
                print_absolute(current);
                return;
            }
            let paired: Vec<&SlotWave> = current
                .iter()
                .filter(|slot| file.slots.contains_key(&slot.id))
                .collect();
            let fresh: Vec<&SlotWave> = current
                .iter()
                .filter(|slot| !file.slots.contains_key(&slot.id))
                .collect();
            if paired.len() < file.slots.len() {
                println!(
                    "measured {} of {} baseline slots",
                    paired.len(),
                    file.slots.len()
                );
            }
            print_pool(
                "steady (4 ticks or longer)",
                &paired,
                file,
                "steady",
                MIN_STEADY_SAMPLES,
            );
            print_pool("effect (single-tone fit)", &paired, file, "effect", 0);
            println!("slots");
            for slot in current {
                match file.slots.get(&slot.id) {
                    Some(previous) => print_slot_delta(slot, previous),
                    None => print_slot_absolute(slot),
                }
            }
            if !fresh.is_empty() {
                println!("slots above with no baseline entry are measured only");
            }
            print_moved(current, file);
        }
    }
}

fn print_absolute(slots: &[SlotWave]) {
    print_pool_absolute(
        "steady (4 ticks or longer)",
        slots,
        "steady",
        MIN_STEADY_SAMPLES,
    );
    print_pool_absolute("effect (single-tone fit)", slots, "effect", 0);
    println!("slots");
    for slot in slots {
        print_slot_absolute(slot);
    }
}

fn print_pool(
    title: &str,
    slots: &[&SlotWave],
    file: &BaselineFile,
    kind: &str,
    min_samples: usize,
) {
    let now = pool_current(slots, kind, min_samples);
    let base = pool_baseline(slots, file, kind, min_samples);
    let (Some(now), Some(base)) = (summarize(&now), summarize(&base)) else {
        return;
    };
    println!("{title} ({} notes)", now.n);
    print_summary_delta(&base, &now);
}

fn print_pool_absolute(title: &str, slots: &[SlotWave], kind: &str, min_samples: usize) {
    let refs: Vec<&SlotWave> = slots.iter().collect();
    let notes = pool_current(&refs, kind, min_samples);
    let Some(summary) = summarize(&notes) else {
        return;
    };
    println!("{title} ({} notes)", summary.n);
    print_summary_absolute(&summary);
}

fn print_slot_absolute(slot: &SlotWave) {
    if slot.notes.is_empty() {
        println!("  {:<28} no tonal notes", slot.id);
        return;
    }
    let Some((kind, summary)) = slot_summary(&slot.notes) else {
        println!("  {:<28} no tonal notes", slot.id);
        return;
    };
    println!("  {} ({} {kind})", slot.id, summary.n);
    print_summary_absolute(&summary);
}

fn print_slot_delta(slot: &SlotWave, previous: &[NoteMatch]) {
    if slot.notes.is_empty() && previous.is_empty() {
        println!("  {:<28} no tonal notes", slot.id);
        return;
    }
    let (kind, now) = match slot_summary(&slot.notes) {
        Some(summary) => summary,
        None => {
            println!("  {:<28} no tonal notes", slot.id);
            return;
        }
    };
    let base_notes: Vec<NoteMatch> = previous
        .iter()
        .filter(|note| note.kind == kind)
        .cloned()
        .collect();
    let Some(base) = summarize(&base_notes) else {
        print_slot_absolute(slot);
        return;
    };
    if same_summary(&base, &now) && base.n == now.n && !notes_moved(&slot.notes, previous) {
        let status = paint(Change::Same, &format!("same ({} {kind})", now.n));
        println!("  {:<28} {status}", slot.id);
        return;
    }
    println!("  {} ({} {kind})", slot.id, now.n);
    if base.n != now.n {
        println!("    notes        {} -> {}", base.n, now.n);
    }
    print_summary_delta(&base, &now);
}

fn print_moved(current: &[SlotWave], file: &BaselineFile) {
    let mut lines = Vec::new();
    for slot in current {
        let Some(previous) = file.slots.get(&slot.id) else {
            continue;
        };
        for note in &slot.notes {
            match previous.iter().find(|old| old.index == note.index) {
                None => lines.push(format!("  {} note {}: added", slot.id, note.index)),
                Some(old) => {
                    if let Some(delta) = note_delta(old, note) {
                        lines.push(format!(
                            "  {} note {} {} {delta}",
                            slot.id, note.index, note.wave
                        ));
                    }
                }
            }
        }
        for old in previous {
            if !slot.notes.iter().any(|note| note.index == old.index) {
                lines.push(format!(
                    "  {} note {} {}: removed",
                    slot.id, old.index, old.wave
                ));
            }
        }
    }
    if lines.is_empty() {
        println!("{}", paint(Change::Same, "no note moved"));
    } else {
        println!("notes that moved");
        for line in lines {
            println!("{line}");
        }
    }
}

fn print_summary_absolute(summary: &Summary) {
    println!("    frequency    {} cents rms", format_num(summary.freq, 2));
    println!("    amplitude    {} |ratio-1|", format_num(summary.amp, 3));
    println!(
        "    phase        {} samples rms",
        format_num(summary.phase, 2)
    );
    println!(
        "    shape        {} pico residual",
        format_num(summary.shape, 1)
    );
}

fn print_summary_delta(base: &Summary, now: &Summary) {
    println!(
        "    {}",
        metric_line("frequency", "cents rms", 2, FREQ_EPS, base.freq, now.freq)
    );
    println!(
        "    {}",
        metric_line("amplitude", "|ratio-1|", 3, AMP_EPS, base.amp, now.amp)
    );
    println!(
        "    {}",
        metric_line("phase", "samples rms", 2, PHASE_EPS, base.phase, now.phase)
    );
    println!(
        "    {}",
        metric_line(
            "shape",
            "pico residual",
            1,
            SHAPE_EPS,
            base.shape,
            now.shape
        )
    );
}

fn metric_line(label: &str, unit: &str, digits: usize, eps: f64, base: f64, now: f64) -> String {
    let change = classify(base, now, eps);
    let base_s = format_num(base, digits);
    let now_s = format_num(now, digits);
    if change == Change::Same {
        return format!(
            "{label:<12} {base_s} -> {now_s} {unit:<14} {}",
            paint(change, "same")
        );
    }
    let pct = if base.abs() > eps {
        format!("{:+.1}%", (now - base) / base * 100.0)
    } else {
        format!("{:+.3}", now - base)
    };
    let verdict = format!("{pct:>8}  {}", change.label());
    format!(
        "{label:<12} {base_s} -> {now_s} {unit:<14} {}",
        paint(change, &verdict)
    )
}

fn paint(change: Change, text: &str) -> String {
    let code = match change {
        Change::Improved => "32",
        Change::Regressed => "31",
        Change::Same => "33",
    };
    if !color_enabled() {
        return text.to_string();
    }
    format!("\x1b[{code}m{text}\x1b[0m")
}

fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|value| value != "0") {
        return true;
    }
    io::stdout().is_terminal()
}

fn notes_moved(current: &[NoteMatch], previous: &[NoteMatch]) -> bool {
    current.iter().any(
        |note| match previous.iter().find(|old| old.index == note.index) {
            None => true,
            Some(old) => note_delta(old, note).is_some(),
        },
    ) || previous
        .iter()
        .any(|old| !current.iter().any(|note| note.index == old.index))
}

fn note_delta(old: &NoteMatch, new: &NoteMatch) -> Option<String> {
    if old.wave != new.wave || old.kind != new.kind {
        return Some(format!(
            "{} {} -> {} {}",
            old.kind, old.wave, new.kind, new.wave
        ));
    }
    let mut parts = Vec::new();
    if (old.cents - new.cents).abs() > NOTE_FREQ_EPS {
        parts.push(format!(
            "freq {:+.2} -> {:+.2} cents (pico {:+.2} -> {:+.2} / nano {:+.2} -> {:+.2})",
            old.cents, new.cents, old.pico_cents, new.pico_cents, old.nano_cents, new.nano_cents
        ));
    }
    if (old.amp_ratio - new.amp_ratio).abs() > NOTE_AMP_EPS {
        parts.push(format!("amp {:.3} -> {:.3}", old.amp_ratio, new.amp_ratio));
    }
    if (old.phase_samples - new.phase_samples).abs() > NOTE_PHASE_EPS {
        parts.push(format!(
            "phase {:+.2} -> {:+.2} samples",
            old.phase_samples, new.phase_samples
        ));
    }
    if (old.pico_shape_rms - new.pico_shape_rms).abs() > NOTE_SHAPE_EPS
        || (old.nano_shape_rms - new.nano_shape_rms).abs() > NOTE_SHAPE_EPS
    {
        parts.push(format!(
            "shape {:.0}/{:.0} -> {:.0}/{:.0}",
            old.pico_shape_rms, old.nano_shape_rms, new.pico_shape_rms, new.nano_shape_rms
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn same_summary(base: &Summary, now: &Summary) -> bool {
    classify(base.freq, now.freq, FREQ_EPS) == Change::Same
        && classify(base.amp, now.amp, AMP_EPS) == Change::Same
        && classify(base.phase, now.phase, PHASE_EPS) == Change::Same
        && classify(base.shape, now.shape, SHAPE_EPS) == Change::Same
}

fn classify(base: f64, now: f64, eps: f64) -> Change {
    if now < base - eps {
        Change::Improved
    } else if now > base + eps {
        Change::Regressed
    } else {
        Change::Same
    }
}

fn slot_summary(notes: &[NoteMatch]) -> Option<(&'static str, Summary)> {
    let steady: Vec<NoteMatch> = notes
        .iter()
        .filter(|note| note.kind == "steady")
        .cloned()
        .collect();
    if let Some(summary) = summarize(&steady) {
        return Some(("steady", summary));
    }
    let effect: Vec<NoteMatch> = notes
        .iter()
        .filter(|note| note.kind == "effect")
        .cloned()
        .collect();
    summarize(&effect).map(|summary| ("effect", summary))
}

fn pool_current(slots: &[&SlotWave], kind: &str, min_samples: usize) -> Vec<NoteMatch> {
    slots
        .iter()
        .flat_map(|slot| {
            slot.notes
                .iter()
                .filter(|note| note.kind == kind && note.samples >= min_samples)
                .cloned()
        })
        .collect()
}

fn pool_baseline(
    slots: &[&SlotWave],
    file: &BaselineFile,
    kind: &str,
    min_samples: usize,
) -> Vec<NoteMatch> {
    slots
        .iter()
        .filter_map(|slot| file.slots.get(&slot.id))
        .flat_map(|notes| {
            notes
                .iter()
                .filter(|note| note.kind == kind && note.samples >= min_samples)
                .cloned()
        })
        .collect()
}

fn summarize(notes: &[NoteMatch]) -> Option<Summary> {
    if notes.is_empty() {
        return None;
    }
    let n = notes.len() as f64;
    let freq = (notes
        .iter()
        .map(|note| note.cents * note.cents)
        .sum::<f64>()
        / n)
        .sqrt();
    let amp = notes
        .iter()
        .map(|note| (note.amp_ratio - 1.0).abs())
        .sum::<f64>()
        / n;
    let phase = (notes
        .iter()
        .map(|note| note.phase_samples * note.phase_samples)
        .sum::<f64>()
        / n)
        .sqrt();
    let shape = notes.iter().map(|note| note.pico_shape_rms).sum::<f64>() / n;
    Some(Summary {
        n: notes.len(),
        freq,
        amp,
        phase,
        shape,
    })
}

fn quantize_slots(slots: &[SlotWave]) -> Vec<SlotWave> {
    slots
        .iter()
        .map(|slot| SlotWave {
            id: slot.id.clone(),
            notes: slot.notes.iter().cloned().map(quantize_note).collect(),
        })
        .collect()
}

fn quantize_note(mut note: NoteMatch) -> NoteMatch {
    note.cents = round_digits(note.cents, 2);
    note.pico_cents = round_digits(note.pico_cents, 2);
    note.nano_cents = round_digits(note.nano_cents, 2);
    note.amp_ratio = round_digits(note.amp_ratio, 4);
    note.phase_samples = round_digits(note.phase_samples, 2);
    note.pico_shape_rms = round_digits(note.pico_shape_rms, 1);
    note.nano_shape_rms = round_digits(note.nano_shape_rms, 1);
    note
}

fn round_digits(value: f64, digits: i32) -> f64 {
    let scale = 10f64.powi(digits);
    (value * scale).round() / scale
}

fn format_num(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

enum Loaded {
    Missing,
    Corrupt(String),
    Ready(BaselineFile),
}

fn empty_file() -> BaselineFile {
    BaselineFile {
        version: VERSION,
        description: description().to_string(),
        slots: BTreeMap::new(),
    }
}

fn load_baseline(path: &Path) -> Loaded {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Loaded::Missing,
        Err(err) => return Loaded::Corrupt(err.to_string()),
    };
    match serde_json::from_str(&text) {
        Ok(file) => Loaded::Ready(file),
        Err(err) => Loaded::Corrupt(err.to_string()),
    }
}

fn write_baseline(path: &Path, file: &BaselineFile) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(file).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(path, text)
}

enum Flag {
    Absent,
    Present,
    Value(String),
    Invalid(String),
}

fn saving() -> bool {
    !matches!(flag_state("--save-baseline"), Flag::Absent)
}

fn save_hint() -> String {
    let name = match flag_state("--baseline") {
        Flag::Value(name) => Some(name),
        _ => match flag_state("--save-baseline") {
            Flag::Value(name) => Some(name),
            _ => None,
        },
    };
    match name {
        Some(name) => format!("cargo test-sfx synth --save-baseline {name}"),
        None => "cargo test-sfx synth --save-baseline".to_string(),
    }
}

fn flag_state(flag: &str) -> Flag {
    let args: Vec<String> = std::env::args().skip(1).filter(|arg| arg != "--").collect();
    let mut saw = false;
    let mut raw = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            saw = true;
            raw = Some(value.to_string());
        } else if arg == flag {
            saw = true;
            raw = None;
            if let Some(next) = args.get(i + 1)
                && !next.starts_with('-')
            {
                raw = Some(next.clone());
            }
        }
        i += 1;
    }
    if !saw {
        Flag::Absent
    } else if let Some(raw) = raw {
        match baseline_name(&raw) {
            Ok(name) => Flag::Value(name),
            Err(err) => Flag::Invalid(err),
        }
    } else {
        Flag::Present
    }
}

fn baseline_name(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Ok(raw.to_string())
    } else {
        Err(format!(
            "baseline name {raw:?} must use letters, digits, '-' or '_'"
        ))
    }
}
