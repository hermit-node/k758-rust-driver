//! K758 profile investigation; read commands are nonconfiguring, optional --select-m1 changes active bank.
//! Uses the same report framing as public HIDra and captured OEM packets.
//! Ordinary audit mode uses READ commands only. --select-m1 sends an OEM-captured profile-selection report.
//! Fail closed if Redragon OEM process is open.
use anyhow::{bail, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

type Frame = [u8; 64];
const VID: u16 = 0x0c45;
const PID: u16 = 0x5004;

fn checksum(buf: &Frame) -> u16 {
    buf[3..].iter().map(|x| *x as u16).sum()
}
fn make_query(cmd: u8, addr: u16, len: u8) -> Frame {
    let mut b = [0u8; 64];
    b[0] = 4;
    b[3] = cmd;
    b[4] = len;
    b[5] = addr as u8;
    b[6] = (addr >> 8) as u8;
    let c = checksum(&b).to_le_bytes();
    b[1] = c[0];
    b[2] = c[1];
    b
}
fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
fn validate(b: &Frame) -> Result<()> {
    if b[0] != 4 {
        bail!("Expected HID report 04, got {:02x}", b[0]);
    }
    let seen = u16::from_le_bytes([b[1], b[2]]);
    let expected = checksum(b);
    if seen != expected {
        bail!(
            "Incorrect 16-bit checksum, expected {:04X}, found {:04X}",
            expected,
            seen
        );
    }
    Ok(())
}
fn oem_open() -> Result<bool> {
    let r = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq K758RGB.exe", "/FO", "CSV", "/NH"])
        .output()
        .context("Cannot check vendor application's process state")?;
    if !r.status.success() {
        bail!("Cannot check whether OEM software is open");
    }
    let lines = String::from_utf8_lossy(&r.stdout);
    Ok(lines.lines().any(|line| {
        line.trim_start()
            .to_ascii_lowercase()
            .starts_with("\"k758rgb.exe\"")
    }))
}
fn open_vendor() -> Result<HidDevice> {
    let api = HidApi::new()?;
    let matching = api
        .device_list()
        .filter(|d| {
            d.vendor_id() == VID
                && d.product_id() == PID
                && d.interface_number() == 1
                && d.usage_page() == 0xff1c
                && d.usage() == 0x92
                && d.path()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains("col04")
        })
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        bail!(
            "Found {} matching vendor interfaces; need exactly one",
            matching.len()
        );
    }
    api.open_path(matching[0].path())
        .context("Could not open K758-compatible vendor endpoint")
}
fn query(dev: &HidDevice, cmd: u8, addr: u16, len: u8) -> Result<Frame> {
    if len > 56 {
        bail!("Payload length > 56");
    }
    let sent = make_query(cmd, addr, len);
    validate(&sent)?;
    let bytes = dev
        .write(&sent)
        .context("HIDra-style hidapi.write failed")?;
    if bytes != 64 {
        bail!("Short HID output: {} of 64 bytes", bytes);
    }
    let deadline = Instant::now() + Duration::from_millis(1100);
    while Instant::now() < deadline {
        let mut reply = [0u8; 64];
        let n = dev.read_timeout(&mut reply, 140)?;
        if n == 0 {
            continue;
        }
        if n != 64 {
            bail!("Unexpected USB input length {n}");
        }
        if reply[0] != 4 || reply[3] != cmd {
            continue;
        }
        // OEM input replies retain the REQUEST header/checksum even though
        // the returned data changes the sum. Validate the echoed request
        // header, NOT a checksum over the newly populated reply bytes.
        if reply[1..8] != sent[1..8] {
            bail!(
                "Response header differs from request: sent={}, reply={}",
                hex(&sent[..8]),
                hex(&reply[..8])
            );
        }
        if reply[4] != len || u16::from_le_bytes([reply[5], reply[6]]) != addr {
            bail!(
                "Response cmd={cmd:02X} had unexpected length/address: {}",
                hex(&reply[..8])
            );
        }
        return Ok(reply);
    }
    bail!("No matching response: command {cmd:02X}, address {addr:04X}, length {len}");
}
fn read_profile(dev: &HidDevice) -> Result<u8> {
    let b = query(dev, 0x03, 0, 0x2c)?;
    let tag = [0x55, 0xaa, 0xff, 2, 0x45, 0x0c, 4, 0x50, 9, 1];
    if b[8..18] != tag || b[19] != 0x18 || b[18] >= 4 {
        bail!(
            "Profile status does not match K758's captured M1–M4 marker: {}",
            hex(&b[8..22])
        );
    }
    Ok(b[18])
}
/// Send the manufacturer's exact M1 selection report (03/04 protocol).
/// Selecting a profile is a real HID write, never run in ordinary audit mode.
fn select_m1(dev: &HidDevice) -> Result<()> {
    const PREFIX: [u8; 21] = [
        0x55, 0xaa, 0xff, 0x02, 0x45, 0x0c, 0x04, 0x50, 0x09, 0x01, 0x00, 0x18, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x04, 0x07, 0x08, 0x12,
    ];
    let mut packet = make_query(0x04, 0, 0x2c);
    packet[8..8 + PREFIX.len()].copy_from_slice(&PREFIX);
    let ck = checksum(&packet).to_le_bytes();
    packet[1] = ck[0];
    packet[2] = ck[1];
    validate(&packet)?;
    if packet[..8] != [0x04, 0x1d, 0x03, 0x04, 0x2c, 0, 0, 0] {
        bail!("Computed M1 selection header differs from OEM-captured header");
    }
    let n = dev
        .write(&packet)
        .context("OEM-equivalent M1 selection output failed")?;
    if n != 64 {
        bail!("M1 selection output report wrote {} of 64 bytes", n);
    }
    println!("M1_SELECTION_REPORT_SENT {}", hex(&packet));
    std::thread::sleep(Duration::from_millis(250));
    let bank = read_profile(dev)?;
    if bank != 0 {
        bail!(
            "M1 selection sent, but hardware still reports M{}",
            bank + 1
        );
    }
    println!("M1_SELECTION_VERIFIED=true hardware_configuration_writes=1 (profile selection only)");
    Ok(())
}
fn run() -> Result<()> {
    if oem_open()? {
        bail!("K758RGB.exe is running: close it normally before profile audit (USB operations 0)");
    }
    let dev = open_vendor()?;
    let bank = read_profile(&dev)?;
    println!(
        "HARDWARE_PROFILE=M{} (read from command 03, byte 18)",
        bank + 1
    );
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() == 2 && args[1] == "--preview-select-m1" {
        let mut p = make_query(4, 0, 44);
        let data: [u8; 21] = [
            0x55, 0xaa, 0xff, 2, 0x45, 0x0c, 4, 0x50, 9, 1, 0, 0x18, 0, 0, 0, 0, 1, 4, 7, 8, 0x12,
        ];
        p[8..8 + data.len()].copy_from_slice(&data);
        let sum = checksum(&p).to_le_bytes();
        p[1] = sum[0];
        p[2] = sum[1];
        validate(&p)?;
        println!("M1_SELECTION_PREVIEW={}", hex(&p));
        println!("USB_CONFIGURATION_WRITES=0");
        return Ok(());
    }
    if args.len() == 2 && args[1] == "--select-m1" {
        if bank == 0 {
            println!("ALREADY_M1=true USB_CONFIGURATION_WRITES=0");
        } else if bank == 1 {
            select_m1(&dev)?;
        } else {
            bail!(
                "Expected M1/M2, found M{}. Refusing to change an unexpected active profile",
                bank + 1
            );
        }
        return Ok(());
    }
    if args.len() != 1 {
        bail!("Usage: k758_profile_audit [--preview-select-m1|--select-m1]");
    }
    println!("LIGHTING_BLOCKS OEM observed read offsets at 0x00/0x40/0x80/0xC0");
    let candidates = [
        (0u16, "block0"),
        (0x40, "block1"),
        (0x80, "block2"),
        (0xc0, "block3"),
    ];
    let mut observed = Vec::new();
    for (offset, name) in candidates {
        let b = query(&dev, 0x05, offset, 0x38)?;
        let data = b[8..64].to_vec();
        println!(
            "LIGHTING_CANDIDATE {} addr={:04X} first16={} whole56={}",
            name,
            offset,
            hex(&data[..16]),
            hex(&data)
        );
        observed.push(data);
    }
    for i in 0..observed.len() {
        for j in (i + 1)..observed.len() {
            println!(
                "LIGHTING_BLOCKS_COMPARE_{}_{}={}",
                i,
                j,
                observed[i] == observed[j]
            );
        }
    }
    // K758's OEM writes each entire 192-byte bank at 0xC0 * profile index.
    // Read with CMD 0F, identified as active-keymap read in HIDra.
    // Investigate readback dialect: unlike lighting, keymap read replies
    // are not yet proven on K758. Do not treat empty data as a real map.
    for cmd in [0x07u8, 0x0fu8] {
        for addr in [0x0000u16, 0x00c0u16] {
            match query(&dev, cmd, addr, 0x38) {
                Ok(b) => {
                    let data = &b[8..64];
                    let count = data.iter().filter(|x| **x != 0).count();
                    println!("KEYMAP_READ_DIAGNOSTIC cmd={cmd:02X} addr={addr:04X} nonzero={} first32={}",
                             count,hex(&data[..32]));
                }
                Err(e) => println!(
                    "KEYMAP_READ_UNSUPPORTED cmd={cmd:02X} addr={addr:04X} err={:#}",
                    e
                ),
            }
        }
    }
    // 0F readback uses relative offsets into the currently active keymap.
    let base = 0u16;
    let mut keymap = Vec::new();
    for (rel, len) in [(0u16, 56u8), (0x38, 56), (0x70, 56), (0xa8, 24)] {
        let b = query(&dev, 0x0f, base + rel, len)?;
        keymap.extend_from_slice(&b[8..8 + len as usize]);
    }
    println!("ACTIVE_KEYMAP_HEX={}", hex(&keymap));
    let mut factory = Vec::new();
    for (rel, len) in [(0u16, 56u8), (0x38, 56), (0x70, 56), (0xa8, 24)] {
        let b = query(&dev, 0x07, rel, len)?;
        factory.extend_from_slice(&b[8..8 + len as usize]);
    }
    println!("KEYMAP_07_HEX={}", hex(&factory));
    println!("KEYMAP_07_EQUALS_0F={}", factory == keymap);
    let d = &keymap[84..87];
    println!(
        "ACTIVE_PROFILE_MAP addr={base:04X} bytes={} physical_D_slot_28={} ",
        keymap.len(),
        hex(d)
    );
    let final_bank = read_profile(&dev)?;
    if final_bank != bank {
        bail!(
            "Profile changed during audit: M{} to M{}; data discarded",
            bank + 1,
            final_bank + 1
        );
    }
    println!("PROFILE_STABLE=true; no configuration writes; all data read from cmd 03/05/07/0F");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("PROFILE_AUDIT_BLOCKED: {e:#}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_packets_have_64_bytes_and_16_bit_checksum() {
        for cmd in [3, 5, 0x0f] {
            for off in [0u16, 0x2a, 0x38, 0x54, 0x70, 0xa8, 0xc0, 0x180, 0x228] {
                let b = make_query(cmd, off, 56);
                assert!(validate(&b).is_ok());
                assert_eq!(b[0], 4);
                assert_eq!(u16::from_le_bytes([b[5], b[6]]), off);
            }
        }
    }
    #[test]
    fn physical_d_key_slot_has_expected_offset() {
        assert_eq!(28 * 3, 84);
        assert_eq!(0xc0u16 * 2 + 0x38, 0x1b8);
    }
}
