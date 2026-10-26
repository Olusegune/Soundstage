//! What this computer can run (PRD-002 section 15, `docs/spec/05` section 4). Nothing is sent anywhere.
//!
//! RAM and free disk come from Windows itself. The GPU is read from `nvidia-smi` when an NVIDIA driver
//! is installed (decision D-12: NVIDIA first). Other GPUs are not guessed at: they show as unknown and
//! the verdict says so. The profile and verdict logic is plain functions so it can be tested without hardware.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Gpu {
    pub name: String,
    pub vram_mb: u64,
    pub driver: String,
    /// `cuda` for NVIDIA through nvidia-smi.
    pub backend: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HardwareInfo {
    pub cpu_name: String,
    pub cpu_threads: u32,
    pub ram_mb: Option<u64>,
    pub gpus: Vec<Gpu>,
    /// Free space on the drive that holds downloaded models.
    pub disk_free_mb: Option<u64>,
    /// False when something could not be read, so a verdict should not be taken as final.
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    CloudFirst,
    LocalLight,
    LocalStandard,
    LocalAdvanced,
    LocalWorkstation,
}

impl Profile {
    pub fn parse(s: &str) -> Option<Profile> {
        Some(match s {
            "cloud-first" => Profile::CloudFirst,
            "local-light" => Profile::LocalLight,
            "local-standard" => Profile::LocalStandard,
            "local-advanced" => Profile::LocalAdvanced,
            "local-workstation" => Profile::LocalWorkstation,
            _ => return None,
        })
    }
}

/// Starting thresholds from `docs/spec/05`. They are to be measured on real machines and adjusted there.
pub fn classify(ram_mb: u64, threads: u32, best_vram_mb: u64) -> Profile {
    // Reported sizes round down a little, so "16 GB" is not required to be exactly 16384 MB.
    let gb = |n: u64| n * 1000;
    if ram_mb >= gb(64) && best_vram_mb >= gb(24) {
        Profile::LocalWorkstation
    } else if ram_mb >= gb(32) && best_vram_mb >= gb(16) {
        Profile::LocalAdvanced
    } else if ram_mb >= gb(16) && best_vram_mb >= gb(8) {
        Profile::LocalStandard
    } else if ram_mb >= gb(8) && threads >= 4 {
        Profile::LocalLight
    } else {
        Profile::CloudFirst
    }
}

pub fn profile_of(info: &HardwareInfo) -> Option<Profile> {
    let ram = info.ram_mb?;
    let vram = info.gpus.iter().map(|g| g.vram_mb).max().unwrap_or(0);
    Some(classify(ram, info.cpu_threads, vram))
}

/// Parse `nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader,nounits`.
pub fn parse_nvidia_smi(out: &str) -> Vec<Gpu> {
    out.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split(',').map(str::trim).collect();
            if parts.len() != 3 || parts[0].is_empty() {
                return None;
            }
            let vram_mb = parts[1].parse::<u64>().ok()?;
            Some(Gpu { name: parts[0].chars().take(80).collect(), vram_mb, driver: parts[2].chars().take(20).collect(), backend: "cuda".into() })
        })
        .take(8)
        .collect()
}

#[cfg(windows)]
fn ram_mb() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: MEMORYSTATUSEX is plain data, valid when zeroed.
    let mut s: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    s.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: `s` is a properly sized MEMORYSTATUSEX that lives for the call.
    let ok = unsafe { GlobalMemoryStatusEx(&mut s) };
    (ok != 0).then_some(s.ullTotalPhys / (1024 * 1024))
}

#[cfg(not(windows))]
fn ram_mb() -> Option<u64> {
    None
}

#[cfg(windows)]
fn disk_free_mb(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    // The folder may not exist yet; ask about the nearest one that does.
    let mut p = path;
    while !p.exists() {
        p = p.parent()?;
    }
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free: u64 = 0;
    // SAFETY: `wide` is NUL-terminated and `free` is a valid out pointer; the other two may be null.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) };
    (ok != 0).then_some(free / (1024 * 1024))
}

#[cfg(not(windows))]
fn disk_free_mb(_path: &Path) -> Option<u64> {
    None
}

/// Run nvidia-smi from its fixed install locations (never searched for on PATH), with a time limit.
fn nvidia_gpus() -> Option<Vec<Gpu>> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = [format!("{root}\\System32\\nvidia-smi.exe"), "C:\\Program Files\\NVIDIA Corporation\\NVSMI\\nvidia-smi.exe".to_string()];
    let exe = candidates.iter().find(|p| Path::new(p).is_file())?;
    let mut child = Command::new(exe)
        .args(["--query-gpu=name,memory.total,driver_version", "--format=csv,noheader,nounits"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(st) if st.success() => break,
            Some(_) => return None,
            None if start.elapsed() > std::time::Duration::from_secs(5) => {
                let _ = child.kill();
                return None;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(40)),
        }
    }
    let mut s = String::new();
    child.stdout.take()?.take(16 * 1024).read_to_string(&mut s).ok()?;
    Some(parse_nvidia_smi(&s))
}

pub fn detect(models_dir: &Path) -> HardwareInfo {
    let ram = ram_mb();
    let disk = disk_free_mb(models_dir);
    let nv = nvidia_gpus();
    HardwareInfo {
        cpu_name: std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default().chars().take(100).collect(),
        cpu_threads: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        ram_mb: ram,
        gpus: nv.unwrap_or_default(),
        disk_free_mb: disk,
        // No nvidia-smi is a normal answer (no NVIDIA card), so only RAM and disk count as unreadable.
        complete: ram.is_some() && disk.is_some(),
    }
}

/// What a model asks of the computer.
#[derive(Clone, Debug, Default)]
pub struct Need {
    pub profile_min: String,
    pub vram_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    /// True when the model can still run, slowly, one profile below its minimum (for example on the CPU).
    pub cpu_fallback: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    RunsWell,
    RunsSlowly,
    WillNotRun,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Verdict {
    pub fit: Fit,
    pub reason: String,
}

fn v(fit: Fit, reason: &str) -> Verdict {
    Verdict { fit, reason: reason.into() }
}

pub fn verdict(info: &HardwareInfo, need: &Need) -> Verdict {
    let Some(min) = Profile::parse(&need.profile_min) else { return v(Fit::Unknown, "This model's requirements are not known.") };
    let Some(have) = profile_of(info) else { return v(Fit::Unknown, "Soundstage could not read this computer's memory, so it cannot say.") };
    // Disk is a hard limit whatever the profile: the download needs its size plus a fifth.
    if let (Some(need_mb), Some(free)) = (need.disk_mb, info.disk_free_mb) {
        if free < need_mb + need_mb / 5 {
            return v(Fit::WillNotRun, "There is not enough free disk space for this model.");
        }
    }
    if let Some(vram) = need.vram_mb {
        let best = info.gpus.iter().map(|g| g.vram_mb).max().unwrap_or(0);
        if info.gpus.is_empty() {
            return if need.cpu_fallback {
                v(Fit::RunsSlowly, "No supported graphics card was found, so it would run on the processor and be slow.")
            } else {
                v(Fit::WillNotRun, "This model needs an NVIDIA graphics card, and none was found.")
            };
        }
        if best < vram {
            return if need.cpu_fallback {
                v(Fit::RunsSlowly, "The graphics card has less memory than recommended, so it would be slow.")
            } else {
                v(Fit::WillNotRun, "The graphics card does not have enough memory for this model.")
            };
        }
    }
    if have >= min {
        v(Fit::RunsWell, "This computer meets what the model asks for.")
    } else if need.cpu_fallback && have as u8 + 1 >= min as u8 {
        v(Fit::RunsSlowly, "This computer is just under what the model asks for, so it would be slow.")
    } else {
        v(Fit::WillNotRun, "This computer is below what the model asks for.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(ram: Option<u64>, threads: u32, vram: &[u64], disk: Option<u64>) -> HardwareInfo {
        HardwareInfo {
            cpu_name: "test".into(),
            cpu_threads: threads,
            ram_mb: ram,
            gpus: vram.iter().map(|&v| Gpu { name: "GPU".into(), vram_mb: v, driver: "1".into(), backend: "cuda".into() }).collect(),
            disk_free_mb: disk,
            complete: ram.is_some() && disk.is_some(),
        }
    }

    #[test]
    fn profiles_follow_the_documented_thresholds() {
        assert_eq!(classify(4_000, 2, 0), Profile::CloudFirst);
        assert_eq!(classify(8_192, 4, 0), Profile::LocalLight);
        assert_eq!(classify(8_192, 2, 0), Profile::CloudFirst, "too few cores");
        assert_eq!(classify(16_384, 8, 8_192), Profile::LocalStandard);
        assert_eq!(classify(32_768, 16, 16_384), Profile::LocalAdvanced);
        assert_eq!(classify(65_536, 24, 24_576), Profile::LocalWorkstation);
        assert_eq!(classify(131_072, 32, 8_192), Profile::LocalStandard, "plenty of RAM does not make up for a small card");
        assert!(Profile::LocalAdvanced > Profile::LocalStandard);
    }

    #[test]
    fn nvidia_smi_output_is_parsed_and_junk_is_ignored() {
        let g = parse_nvidia_smi("NVIDIA GeForce RTX 4070, 12282, 552.44\nNVIDIA T400, 4096, 552.44\n");
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].vram_mb, 12282);
        assert!(parse_nvidia_smi("NVIDIA-SMI has failed because it could not talk to the driver").is_empty());
        assert!(parse_nvidia_smi("a, notanumber, 1").is_empty());
        assert!(parse_nvidia_smi("").is_empty());
    }

    #[test]
    fn verdicts_cover_each_outcome() {
        let pc = info(Some(32_768), 16, &[12_000], Some(500_000));
        let need = |p: &str| Need { profile_min: p.into(), ..Default::default() };
        assert_eq!(verdict(&pc, &need("local-standard")).fit, Fit::RunsWell);
        assert_eq!(verdict(&pc, &need("local-workstation")).fit, Fit::WillNotRun);
        assert_eq!(verdict(&pc, &Need { profile_min: "local-advanced".into(), cpu_fallback: true, ..Default::default() }).fit, Fit::RunsSlowly, "one profile under, with a slow fallback");
        assert_eq!(verdict(&pc, &Need { profile_min: "local-standard".into(), vram_mb: Some(24_000), ..Default::default() }).fit, Fit::WillNotRun);
        assert_eq!(verdict(&pc, &Need { profile_min: "local-standard".into(), vram_mb: Some(8_000), ..Default::default() }).fit, Fit::RunsWell);
        assert_eq!(verdict(&pc, &need("who-knows")).fit, Fit::Unknown);
    }

    #[test]
    fn disk_space_needs_the_size_plus_a_fifth_and_unreadable_hardware_is_unknown() {
        let pc = info(Some(32_768), 16, &[12_000], Some(1_000));
        let big = Need { profile_min: "local-light".into(), disk_mb: Some(900), ..Default::default() };
        assert_eq!(verdict(&pc, &big).fit, Fit::WillNotRun);
        let fits = Need { profile_min: "local-light".into(), disk_mb: Some(800), ..Default::default() };
        assert_eq!(verdict(&pc, &fits).fit, Fit::RunsWell);
        assert_eq!(verdict(&info(None, 8, &[], None), &fits).fit, Fit::Unknown);
    }

    #[test]
    fn no_graphics_card_means_slow_or_no_depending_on_the_model() {
        let pc = info(Some(32_768), 16, &[], Some(500_000));
        let gpu_only = Need { profile_min: "local-light".into(), vram_mb: Some(8_000), ..Default::default() };
        assert_eq!(verdict(&pc, &gpu_only).fit, Fit::WillNotRun);
        assert_eq!(verdict(&pc, &Need { cpu_fallback: true, ..gpu_only }).fit, Fit::RunsSlowly);
    }

    #[test]
    fn detect_runs_on_this_machine_without_panicking() {
        let h = detect(&std::env::temp_dir());
        assert!(h.cpu_threads >= 1);
        if cfg!(windows) {
            assert!(h.ram_mb.unwrap_or(0) > 512, "RAM should be readable on Windows");
            assert!(h.disk_free_mb.is_some());
        }
    }
}
