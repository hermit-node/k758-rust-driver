//! Dedicated Rust K758 test. Never modifies Python, PySide6, or existing Rust.
use anyhow::{bail, Context, Result};
use hidapi::{HidApi, HidDevice};
use serde_json::Value;
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};
type P = [u8; 64];
fn hx(p: &[u8]) -> String {
    p.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
fn sum(p: &P) -> u16 {
    p[3..].iter().map(|v| *v as u16).sum()
}
fn check(p: &P) -> Result<()> {
    let c = u16::from_le_bytes([p[1], p[2]]);
    if p[0] != 4 || sum(p) != c {
        bail!(
            "Incorrect report or 16-bit checksum, expected {:04x}, got {:04x}",
            sum(p),
            c
        );
    }
    Ok(())
}
fn decode(s: &str) -> Result<P> {
    let t = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>();
    if t.len() != 128 {
        bail!("HID packet !=64 bytes");
    }
    let mut b = [0; 64];
    for i in 0..64 {
        b[i] = u8::from_str_radix(&t[2 * i..2 * i + 2], 16)?;
    }
    check(&b)?;
    Ok(b)
}
fn packet(cmd: u8, off: u16, len: u8) -> P {
    let mut b = [0; 64];
    b[0] = 4;
    b[3] = cmd;
    b[4] = len;
    b[5] = off as u8;
    b[6] = (off >> 8) as u8;
    let c = sum(&b).to_le_bytes();
    b[1] = c[0];
    b[2] = c[1];
    b
}
fn preview(path: &Path) -> Result<(Vec<P>, Vec<P>)> {
    let v: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    if v["status"] != "PREVIEW_ONLY_NO_HARDWARE_WRITES"
        || v["target_profile"] != "M1"
        || v["physical_key"] != "D"
        || v["source_capture_sha256"]
            != "bd0832ed99079054817dd5a5c2337d023514965f527a9a5621c5d13e83750d0f"
    {
        bail!("Unrecognized M1 preview/provenance");
    }
    let take = |name: &str| -> Result<Vec<P>> {
        let rows = v[name].as_array().context("Missing frames")?;
        if rows.len() != 9 {
            bail!("M1 transaction must have nine frames");
        }
        rows.iter()
            .map(|x| decode(x.as_str().context("Invalid frame hex")?))
            .collect()
    };
    let orig = take("original_frames_hex")?;
    let modified = take("proposed_frames_hex")?;
    let commands = [1, 10, 10, 6, 8, 8, 8, 8, 2];
    let lengths = [0, 56, 26, 4, 56, 56, 56, 24, 0];
    let offsets = [0, 0, 0x38, 0x12, 0, 0x38, 0x70, 0xa8, 0];
    for (i, b) in orig.iter().enumerate() {
        if b[3] != commands[i]
            || b[4] != lengths[i]
            || u16::from_le_bytes([b[5], b[6]]) != offsets[i]
        {
            bail!("Unrecognized M1 frame {i}");
        }
    }
    let mut diffs = Vec::new();
    for (i, (a, b)) in orig.iter().zip(modified.iter()).enumerate() {
        for j in 0..64 {
            if a[j] != b[j] {
                diffs.push((i, j, a[j], b[j]));
            }
        }
    }
    if diffs != vec![(5, 1, 0x7a, 0x7d), (5, 38, 4, 7)]
        || orig[5][36..39] != [2, 2, 4]
        || modified[5][36..39] != [2, 2, 7]
    {
        bail!("Unexpected byte diff in proposed M1 configuration: {diffs:?}");
    }
    Ok((orig, modified))
}
fn map(frames: &[P]) -> Vec<u8> {
    let mut out = Vec::new();
    for b in &frames[4..8] {
        out.extend_from_slice(&b[8..8 + b[4] as usize]);
    }
    out
}
fn oem() -> Result<bool> {
    let o = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq K758RGB.exe", "/FO", "CSV", "/NH"])
        .output()?;
    if !o.status.success() {
        bail!("Cannot inspect OEM process");
    }
    Ok(String::from_utf8_lossy(&o.stdout).lines().any(|s| {
        s.trim_start()
            .to_ascii_lowercase()
            .starts_with("\"k758rgb.exe\"")
    }))
}
struct Link {
    dev: HidDevice,
}
impl Link {
    fn open() -> Result<Self> {
        let api = HidApi::new()?;
        let rows = api
            .device_list()
            .filter(|d| {
                d.vendor_id() == 0x0c45
                    && d.product_id() == 0x5004
                    && d.interface_number() == 1
                    && d.usage_page() == 0xff1c
                    && d.usage() == 0x92
                    && d.path()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains("col04")
            })
            .collect::<Vec<_>>();
        if rows.len() != 1 {
            bail!(
                "Expected exactly one K758-compatible vendor endpoint, found {}",
                rows.len()
            );
        }
        let dev = api.open_path(rows[0].path())?;
        Ok(Self { dev })
    }
    fn send(&self, p: &P) -> Result<()> {
        check(p)?;
        let n = self
            .dev
            .write(p)
            .context("HIDra-compatible interrupt output report failed")?;
        if n != 64 {
            bail!("Short HID output report: {n}/64");
        }
        Ok(())
    }
    fn ack(&self, expected: &P) -> Result<P> {
        let until = Instant::now() + Duration::from_millis(1200);
        while Instant::now() < until {
            let mut b = [0; 64];
            let n = self.dev.read_timeout(&mut b, 120)?;
            if n == 0 {
                continue;
            }
            if n != 64 {
                bail!("Short vendor HID reply: {n}/64");
            }
            if b[0] != 4 || b[3] != expected[3] {
                continue;
            }
            // K758 retains request checksum/header in data-bearing replies.
            // Never validate a response checksum across returned payload bytes.
            if b[..8] != expected[..8] {
                bail!(
                    "Unexpected K758 ACK header: request={}, response={}",
                    hx(&expected[..8]),
                    hx(&b[..8])
                );
            }
            return Ok(b);
        }
        bail!("No valid K758 ACK for command {:02X}", expected[3])
    }
    fn query(&self, cmd: u8, off: u16, len: u8) -> Result<P> {
        let req = packet(cmd, off, len);
        self.send(&req)?;
        let b = self.ack(&req)?;
        Ok(b)
    }
    fn bank(&self) -> Result<u8> {
        let b = self.query(3, 0, 44)?;
        if b[8..18] != [0x55, 0xaa, 0xff, 2, 0x45, 0x0c, 4, 0x50, 9, 1]
            || b[19] != 0x18
            || b[18] > 3
        {
            bail!("Invalid K758 hardware bank signature");
        }
        Ok(b[18])
    }
    fn active_map(&self) -> Result<Vec<u8>> {
        let mut v = Vec::new();
        for (off, len) in [(0, 56), (0x38, 56), (0x70, 56), (0xa8, 24)] {
            let r = self.query(0x07, off, len)?;
            v.extend_from_slice(&r[8..8 + len as usize]);
        }
        Ok(v)
    }
    fn apply(&self, frames: &[P]) -> Result<()> {
        for (i, b) in frames.iter().enumerate() {
            self.send(b)
                .with_context(|| format!("M1 frame {}/{} send failed", i + 1, frames.len()))?;
            let reply = self
                .ack(b)
                .with_context(|| format!("M1 frame {}/{} response failed", i + 1, frames.len()))?;
            println!(
                "FRAME_ACK {}/{} op={:02X} {}",
                i + 1,
                frames.len(),
                b[3],
                hx(&reply[..8])
            );
            thread::sleep(Duration::from_millis(30));
        }
        Ok(())
    }
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        bail!("Usage: k758-protocol-lab <preview.json> <--dry-run|--read-current|--verify-restored|--apply-m1-d> [--confirm-complete-m1-overwrite]");
    }
    let (old, new) = preview(Path::new(&args[1]))?;
    println!(
        "CAPTURE_VALIDATED frames=9 length=64 checksum=16bit M1_macro_preserved=true D_A_to_D=true"
    );
    let mode = args[2].as_str();
    if mode == "--dry-run" {
        println!("DRY_RUN physical_writes=0");
        return Ok(());
    }
    if mode != "--read-current" && mode != "--verify-restored" && mode != "--apply-m1-d" {
        bail!("Unknown mode {mode}");
    }
    if mode == "--apply-m1-d" && (args.len() != 4 || args[3] != "--confirm-complete-m1-overwrite") {
        bail!("Missing full M1 write confirmation");
    }
    if oem()? {
        bail!("OEM K758RGB.exe is still running; no HID queries/writes allowed");
    }
    let link = Link::open()?;
    let bank = link.bank()?;
    println!("HARDWARE_PROFILE=M{}", bank + 1);
    if bank != 0 {
        bail!("M1 must be active: switch keyboard physically and retry");
    }
    let curr = link.active_map()?;
    let prev = map(&old);
    println!("MAP_LENGTH read={} expected={}", curr.len(), prev.len());
    println!("PHYSICAL_D_ENTRY {}", hx(curr.get(84..87).unwrap_or(&[])));
    if mode == "--verify-restored" {
        if curr != map(&new) {
            bail!(
                "M1 does NOT match the restored profile: current D={}",
                hx(curr.get(84..87).unwrap_or(&[]))
            );
        }
        println!("RESTORATION_VERIFIED full_keymap=192/192 D=02_02_07 W_macro=05_01_00 configuration_writes=0");
        return Ok(());
    }
    if mode == "--apply-m1-d" && curr == map(&new) {
        println!("ALREADY_RESTORED configuration_writes=0");
        return Ok(());
    }
    if curr != prev {
        let mismatches = curr
            .iter()
            .zip(prev.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("{i}:{a:02X}!={b:02X}"))
            .collect::<Vec<_>>();
        bail!(
            "CURRENT M1 MAP DOES NOT MATCH CAPTURE ({} mismatches): {}",
            mismatches.len(),
            mismatches
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if mode == "--read-current" {
        println!("M1_MAPPING_MATCHES_CAPTURE; configuration_writes=0");
        return Ok(());
    }
    if oem()? {
        bail!("OEM restarted during preflight");
    }
    if link.bank()? != 0 {
        bail!("Profile switched away from M1 before write");
    }
    link.apply(&new)?;
    let actual = link.active_map()?;
    if actual != map(&new) {
        bail!("Written M1 keymap failed read-back comparison");
    }
    if link.bank()? != 0 {
        bail!("M1 write finished but device switched profile unexpectedly");
    }
    println!(
        "SUCCESS M1 keymap matches requested D={} (physical D press still needs verification)",
        hx(&actual[84..87])
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_m1_transaction_is_exact_and_checksummed() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let (old, new) = preview(&root.join("fixtures/m1_D_A_to_D_PREVIEW_only.json")).unwrap();
        assert_eq!(old.len(), 9);
        assert_eq!(new.len(), 9);
        assert_eq!(map(&old).len(), 192);
        assert_eq!(map(&old)[84..87], [2, 2, 4]);
        assert_eq!(map(&new)[84..87], [2, 2, 7]);
        assert_eq!(map(&old)[57..60], [5, 1, 0]);
        assert_eq!(map(&new)[57..60], [5, 1, 0]);
    }
    #[test]
    fn invalid_checksum_is_rejected() {
        let mut p = packet(0x08, 0x38, 56);
        assert!(check(&p).is_ok());
        p[8] = 0xff;
        assert!(check(&p).is_err());
    }
}
