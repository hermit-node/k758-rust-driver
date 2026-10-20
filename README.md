# K758 Rust Driver

Standalone Windows Rust tools for the Redragon K758 keypad (SONiX `0C45:5004`). Independent of the PySide6 Studio and G13 bridge.

**Verified 2026-10-20:** Rust sent all nine M1 configuration frames, received nine USB acknowledgments, and independently confirmed physical D's stored mapping changed from A to D. The remaining 191 keymap bytes, W's macro reference, and all four lighting blocks were unchanged. User-confirmed physical keypress testing is not yet recorded.

## Source files

- `src/main.rs`: guarded M1 full-profile restore and independent readback
- `src/bin/k758_profile_audit.rs`: profile, lighting, keymap readout and optional M1 profile-selection command
- `fixtures/m1_D_A_to_D_PREVIEW_only.json`: complete historical OEM nine-frame M1 transaction plus the two-byte restoration edit
- `docs/PROTOCOL_NOTES.md`: observed protocol and uncertainties
- `docs/M1_WRITE_VERIFICATION.json`: sanitized independent postwrite verification

## Build (Windows)

Install stable Rust/Cargo. In this directory run:

```powershell
cargo test --release
cargo build --release
```

Binaries: `target\release\k758-protocol-lab.exe` and `target\release\k758_profile_audit.exe`.

## Inspect and restore

Close the Redragon OEM application `K758RGB.exe` completely, including its tray instance. All operations refuse to proceed while it is running.

Read the active M1-M4 hardware bank, four distinct lighting blocks, and keymaps (nonconfiguring USB reads):

```powershell
.\target\release\k758_profile_audit.exe
```

Select M1 (the second command **changes the active hardware profile** using a verbatim OEM packet):

```powershell
.\target\release\k758_profile_audit.exe --preview-select-m1
.\target\release\k758_profile_audit.exe --select-m1
```

Preview the nine frames without USB writes:

```powershell
.\target\release\k758-protocol-lab.exe .\fixtures\m1_D_A_to_D_PREVIEW_only.json --dry-run
```

Verify M1 still matches the old captured keymap, where D produced A (expected to refuse if restored already):

```powershell
.\target\release\k758-protocol-lab.exe .\fixtures\m1_D_A_to_D_PREVIEW_only.json --read-current
```

Verify the already-restored M1 keymap (read only):

```powershell
.\target\release\k758-protocol-lab.exe .\fixtures\m1_D_A_to_D_PREVIEW_only.json --verify-restored
```

Restore M1, **only** when it exactly matches the captured 192-byte keymap:

```powershell
.\target\release\k758-protocol-lab.exe .\fixtures\m1_D_A_to_D_PREVIEW_only.json --apply-m1-d --confirm-complete-m1-overwrite
```

The restore transmits the **entire historical M1 image**, including its macro program, not just one key. It validates nine 64-byte frames and their 16-bit checksums, confirms the active bank and complete prewrite keymap, reads every USB ACK, and verifies the resulting 192-byte map. If M1 is already restored, it makes no configuration changes.

**Limitation:** M1's live macro table cannot yet be independently read back. The fixture contains the original macro events and can overwrite subsequent macro edits. Do not use this keyboard-specific fixture as a general remapping template.

## Privacy

**Keep this repository private.** The verbatim M1 fixture contains recorded macro data. Raw keystroke captures, original Windows paths, local host credentials, secrets and compiled binaries were deliberately excluded.

Compared against [HIDra](https://github.com/luizjr/HIDra) and [rgb_keyboard](https://github.com/dokutan/rgb_keyboard). Matching VID/PID does not guarantee identical firmware. License GPL-3.0-or-later.
