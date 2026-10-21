# K758 / KHID: reference implementations and integration plan

**Status (2026-10-20): research and proposed implementation, NOT six tested K758 drivers.**
This is the companion to [README_WHAT_WE_TRIED.md](README_WHAT_WE_TRIED.md), which records what our own K758 hardware actually did. For the verified on-device protocol, see [docs/PROTOCOL_NOTES.md](docs/PROTOCOL_NOTES.md).
**New broader search:** [README_BROAD_GITHUB_SURVEY.md](README_BROAD_GITHUB_SURVEY.md) extends these original six references with key/macro projects from the full Redragon GitHub search, including firmware-level macro readers in K550 Yama and MotoSpeed CK108. [mlkamri's K596 G-key remapper](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper) remains a primary input-event reference and is re-examined separately for KHID. It is a press-only vendor event bridge, not the K758 macro storage protocol.

**Additional KHID runtime references:** [Input Remapper](https://github.com/sezanzeb/input-remapper) and [Makima](https://github.com/cyber-sushi/makima) are assessed in [the expanded survey](README_BROAD_GITHUB_SURVEY.md#7-general-purpose-linux-remapping-engines-user-suggested), alongside [mlkamri's K596 G-key remapper](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper). The former two provide software mapping engines; the latter provides a vendor-defined physical-key press notification on K596, **not yet verified on K758**.


## Objective and boundary

The desired input pipeline is **physical K758 input -> trustworthy physical key identity -> our KHID event -> selected profile and action**. Output actions may be keyboard/mouse events, software macros, or app commands. KHID is our *software-owned identity*, not a Windows VK or an existing firmware HID usage.

Our canonical event format can be a six-hex-digit peripheral and key identity, e.g. `KHID:000758:00001C:DOWN` / `KHID:000758:00001C:UP`. These numbers are **illustrative only**, not persisted registry IDs. A HID usage such as `0x07` (D) and a Windows `VK_D` are **not** KHID IDs. Detecting Windows VK_D after a firmware remap does not uniquely establish which physical switch generated it.

Two distinct operations must coexist:
- **Software remapping**: read physical events, translate to KHID, look up a software profile, dispatch the action while the bridge is running.
- **Optional onboard configuration**: read/write the K758's stored 3-byte key assignment and macro blocks; this survives without the bridge running.

The current independent K758 Rust repository implements a targeted **M1 onboard write/readback only**, plus M1-M4 bank detection/read operations. It does **not** implement the general software KHID event pipeline yet.

## Six concrete upstream references

The projects below are different models with different firmware; they are **technique references**, not compatible drivers to concatenate. Paths and links were inspected against upstream sources on 2026-10-20.

| # | Upstream project and relevant model | Key assignment implementation | Macro implementation | Keypress/event implementation | Usefulness for K758 |
|---|---|---|---|---|---|
| 1 | [HIDra](https://github.com/luizjr/HIDra), K586RGB-PRO, SONiX `0C45:5004` | **Implemented:** 3-byte options and paged keymap read/write | **Not found:** a completed macro table writer/reader | Configuration reads, not a physical-key runtime decoder | Best basis for typed key assignment and read-modify-write |
| 2 | [rgb_keyboard](https://github.com/dokutan/rgb_keyboard), several Redragon-compatible boards | **Implemented:** physical-key offset lookup, 3-byte output table, report sequence | **Incomplete:** parser/in-memory action structure; project warns macros are broken | No verified per-key event-to-KHID pipeline | Useful reference for macro representation and OEM transaction templates |
| 3 | [Redragon G-Keys Remapper](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper), K596 Vishnu | OEM configuration of G-keys is a prerequisite | Assign-string mode creates a distinct press notification on that K596; **not a generic macro storage writer** | **Implemented:** Raw Input vendor report -> software-generated F13-F22 tap | Best hypothesis for **non-VK vendor key identification**; report is press-only |
| 4 | [k596-signalrgb](https://github.com/peshke/k596-signalrgb), K596 Vishnu | Protocol docs show firmware settings / flash operations; not a K758 keymap library | No proven K758 macro writer or macro reader | Diagnostic HID reporting, firmware/USB capture workflow; predominantly lighting | Model-specific capture/reply discipline, *not* reusable packet opcodes |
| 5 | [kekboard-warrior](https://github.com/lucian-labs/kekboard-warrior), K585 DITI (Linux) | **Implemented:** JSON keymap, layers, chords, passthrough | **Implemented in software:** text and key-down/key-up event expansion | **Implemented:** evdev key events -> `UInput` virtual keyboard, using `grab()` to suppress source on Linux | Good software-engine design; Windows needs a different input suppression method |
| 6 | [redragonKB-remap](https://github.com/octenite/redragonKB-remap), K617 Fizz / similar OEM formats | **Implemented method:** editing OEM `Cfg.ini` `[KEY]` and `[FN]` codes then using OEM Restore | No independent firmware macro storage implementation | No runtime keypress interception | Inspiration for model-specific option tables/layers; **not** our K758 wire format |

**Not counted as one of the six:** [Redragon K585 Pro SignalRGB](https://github.com/convert313-oss/Redragon-k585-pro-signalrgb) handles **lighting**, not key assignment or macro presses; [QMK](https://github.com/qmk/qmk_firmware) / [VIA](https://github.com/the-via/website) have useful conceptual persistent macro APIs on **different, cooperating firmware**. Neither should be treated as proof of a K758 feature.

## 1. HIDra: copy the *method*, not the K586 memory map

**Source:** [`src-tauri/src/keyboard.rs`](https://github.com/luizjr/HIDra/blob/main/src-tauri/src/keyboard.rs), especially `encode_remap`, `read_keymap`, `write_keymap`, `remap`.

Upstream encodes a three-byte destination:

| Output kind | 3-byte assignment in HIDra | K758 evidence |
|---|---|---|
| Ordinary keyboard HID usage | `02 02 <usage>` | **Confirmed** on K758: `02 02 04` = A; `02 02 07` = D |
| Modifier | `02 01 <usage>` | Upstream-supported; **not independently tested** on K758 |
| Consumer/media | `03 <lo> <hi>` | Upstream-supported; **not independently tested** on K758 |
| Disabled | `02 00 00` | Upstream-supported; **not independently tested** on K758 |
| Macro reference | **Not a supported macro editor in HIDra** | K758 W slot `05 01 00`; preserved through M1 transaction |

HIDra uses a 420-byte K586 table (140 entries). **Our K758 captured active profile table is 192 bytes (64 triples), not 420.** HIDra reads its active table using opcode `0x0F` and its factory table using `0x07`. **Actual K758 testing established the opposite-looking result**: `0x07` returned the modified active M1 keymap, while `0x0F` returned a default-looking image. Never use the other model's read opcode or fallback-to-defaults rule blindly.

**K758 implementation:** `KeyAssignment` should represent `KeyboardUsage(u8)`, `Modifier(u8)`, `Consumer(u16)`, `MacroRef([u8;3])`, `Disabled`, and `Unknown([u8;3])`. Unknown entries must round-trip byte-for-byte; never replace unrecognized options with a default. Read the whole active bank using the verified K758-specific `0x07` routine, patch *exactly one selected triple*, recompute page checksum, and compare pre/post. We must first reverse-engineer the smallest safe transaction that preserves the **live** macro table; today's nine-frame M1 fixture replays a **historical** macro program.

## 2. rgb_keyboard: key-offset tables and incomplete macro parser

**Sources:** [`include/writers.cpp`](https://github.com/dokutan/rgb_keyboard/blob/master/include/writers.cpp) (`write_key_mapping_ansi`), [`include/data.cpp`](https://github.com/dokutan/rgb_keyboard/blob/master/include/data.cpp) (3-byte options and `_data_macros` template), [`include/macro.h`](https://github.com/dokutan/rgb_keyboard/blob/master/include/macro.h), [`include/fileio.cpp`](https://github.com/dokutan/rgb_keyboard/blob/master/include/fileio.cpp), [macro example](https://github.com/dokutan/rgb_keyboard/blob/master/examples/example.keymap), [TODO](https://github.com/dokutan/rgb_keyboard/blob/master/TODO.md).

Upstream locates physical keys by a table of packet/byte offsets, looks up each action's three output bytes, and sends begin -> complete keymap pages -> end. That is an applicable **structural pattern**, not a packet-for-packet K758 implementation.

Upstream macro class represents `{up/down, key, delay}` events and a repeat count. Its example uses entries such as `down=y=5` and `up=y=5`, with delays specified in units of 10 ms. A conditional `_data_macros` packet begins `04 5D 06 0A 2A`, but the project itself labels macros as **broken/unfinished** and has no verified uploader/reader. Matching the `0A` command *number* does not establish matching macro encoding across boards.

**K758 implementation:** first build an **offline decoder** for our own captured two `0x0A` macro frames. Compare captures after a controlled one-event change using OEM software, align the changed bytes and timing, identify slot/payload boundaries, then add parser/serializer tests. No guesswork writes to real storage. A software macro interpreter should be built separately and may store higher-level actions as KHID profile data; it is not limited by firmware macro capacity.

## 3. K596 G-key bridge: possible source-native key identity

**Sources:** [K596 README](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper/blob/main/README.md) and [`k596_gkey_bridge.py`](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper/blob/main/k596_gkey_bridge.py) (`handle_gkey_report`, `handle_wm_input`).

On the author's K596 (USB `062A:8519`), an OEM **Assign a string** G-key configuration makes the firmware emit a vendor HID notification on page/usage `FF19:FF19`:
- report bytes: `08 11 C0` through `08 11 C9` for G1 through G10;
- **one event per press**; it does **not** give independent DOWN/UP or hold/repeat;
- the author's Windows background OEM software separately handles placeholder string typing, so turning it off leaves vendor events without its software text insertion;
- the script captures via `RegisterRawInputDevices` / `WM_INPUT` and sends ordinary F13-F22 taps via `SendInput`.

**K758 implementation:** borrow only the *listener and correlation design*. Enumerate the **actual** K758 vendor HID collection `FF1C:0092` and its separate keyboard collection; record tagged reports when W's currently assigned macro plays, and compare with ordinary keys/M-bank keys. If a physically unique K758 vendor report exists, map report -> physical key -> KHID **directly**, removing the K596 F13 injection step. If the vendor signal is *press-only*, KHID must label it `TRIGGER`, **not fabricate a real DOWN/UP hold pair**. This mechanism is **not yet demonstrated** on the K758.

## 4. k596-signalrgb: how to discover a protocol safely

**Sources:** [`docs/PROTOCOL.md`](https://github.com/peshke/k596-signalrgb/blob/main/docs/PROTOCOL.md), [`tools/k596_tool.py`](https://github.com/peshke/k596-signalrgb/blob/main/tools/k596_tool.py).

This is a firmware/USB reverse-engineering project for K596 `062A:8519`, different MCU and command layout. Its approach is useful: inspect OEM USB captures, label each report family, record response echoes, probe existing **read-only** commands, and differentiate persistent writes from live color updates. The project warns that **K596 feature report `0x06` is a firmware-update unlock sequence**. On our *different* K758 protocol, command `0x06` is an already-captured **profile configuration output**; command-byte values are device-specific and must not be copied across models.

**K758 implementation:** maintain a `KnownPacket` table indexed by model, interface, report ID, command, length, address, direction, and ACK pattern; tests from OEM captures; read-only baseline snapshots. Never copy K596 flash/bootloader/feature reports onto K758.

## 5. K585 kekboard-warrior: software mapping and macro expansion

**Sources:** [`translator.py`](https://github.com/lucian-labs/kekboard-warrior/blob/main/translator.py), [`engine.py`](https://github.com/lucian-labs/kekboard-warrior/blob/main/engine.py).

On Linux, the project reads device-specific evdev codes (press `1`, release `0`, repeat `2`), resolves layer/chord mappings, expands software macro definitions to output key events, and writes via a virtual `UInput` device. Calling `dev.grab()` prevents the original physical events going to other consumers **on Linux**. No onboard macro storage has to be edited.

**K758 implementation:** reimplement the *platform-neutral state machine* in Rust: `physical_key -> KHID -> profile resolution -> Action` (keys, chords, timers, text, macros). Separate output from mapping, deduplicate key transitions, preserve real holds, and release all synthesized held keys on disconnect/stop. Keep input provenance with device fingerprint and USB port identity. A Windows `WM_INPUT` listener **observes** input but cannot suppress the original standard keyboard event. For key outputs that still leak into Windows, genuine replacement requires a separate Windows-compatible filter/interception strategy, or a K758 vendor-only input path with no original keyboard output. Do not assume Linux `grab()` exists on Windows.

## 6. K617 redragonKB-remap: option dictionaries and layers, not HID protocol

**Sources:** [README](https://github.com/octenite/redragonKB-remap) and [Porting notes](https://github.com/octenite/redragonKB-remap/blob/main/instructions/Porting%26modifying%20cgf.txt).

The project maps physical `Kxx` slot numbers to hex output options inside the manufacturer's `Cfg.ini` file. Sections `[KEY]` and `[FN]` express base/Fn layers; an OEM **Restore** command may apply the edited file. Upstream explicitly warns to preserve model-specific `[OPT]` and surrounding mapping metadata.

**K758 implementation:** use the *concept* of per-physical-key slot and typed option dictionaries. **Do not** copy `Cfg.ini` layouts, slot ordering, the K617's option byte formats, or OEM Restore behavior into the K758 driver. Our K758 source of truth is the live `0x07` 192-byte profile readback and validated transaction capture.

## Proposed K758 Rust modules (not yet implemented)

```text
src/
  lib.rs                 # shared entry point, one packet implementation
  device.rs              # VID/PID + actual HID collection + fingerprint + port
  hid_transport.rs       # packet builder, checksum, read ACK, USB write
  profile.rs             # M1-M4 status, active bank selection + snapshots
  keymap.rs              # 192-byte K758 triples, unknown-byte round trip
  macro_capture.rs       # offline decoder for captured 0x0A frames (initially READ ONLY)
  raw_input.rs           # MI_00 keyboard and MI_01 vendor event diagnostics
  khid.rs                # internal canonical key and trigger identity
  mapping.rs             # device-specific profiles, layers and actions
  macro_engine.rs        # software macro execution independent of EEPROM
  output.rs              # Windows action injector and release bookkeeping
src/bin/
  k758_cli.rs            # audit / preview / confirm / verify operations
```

Module boundaries are **proposed**, not existing paths. Keep the existing tested M1 writer and fixture as regressions while refactoring. `make_query` should build a packet; `query` should send/read/validate it. Those are useful separate responsibilities **inside one shared implementation**, not duplicated across binaries.

### Required interfaces (design sketches, NOT compiled API)

```rust
enum DeviceEdge {
    Down { physical_slot: u16 },
    Up { physical_slot: u16 },
    Trigger { physical_slot: u16 }, // press-only notification; no fake release
}

enum Assignment {
    KeyboardUsage(u8),
    Modifier(u8),
    Consumer(u16),
    MacroRef([u8; 3]),
    Disabled,
    Unknown([u8; 3]),  // lossless firmware preservation
}

enum Action {
    Passthrough,
    Key { vk: u16 },
    SoftwareMacro { name: String },
    Disabled,
}
```

Persistent mapping should use **real device identity + physical slot + software profile**; KHID registry IDs are independently allocated stable six-hex-digit identifiers. VID/PID or VK alone are not a sufficient identity. If the device moves USB ports, the registry must distinguish stable hardware identity from port attachment without treating an enumeration index as identity.

## Implementation order, with success criteria

1. **Noninvasive K758 physical event discovery**. Capture keyboard and vendor Raw Input simultaneously; instrument W's existing macro and at least one ordinary key across M1-M4. Label *device path, collection, time, profile*; keep recordings private. Pass only if a reproducible event signature can be correlated to a **specific physical key**. A VK alone is not proof.
2. **Normalize and preserve the firmware schema**. Decode the 64 triples of a K758 active map with correct source-specific `0x07` semantics. Unit-test the complete 192-byte roundtrip, including `05 01 00` references and unknown triples. Keep the four lighting banks separate from the profile selector; do not conflate offsets with M1-M4.
3. **Decode macros OFFLINE** from existing OEM `0x0A` reports, diff controlled captures (if needed) without direct arbitrary writes. Identify macro ID, event encoding and timing. Pass only when decode/re-encode reproduces the captured reports byte-for-byte.
4. **Build KHID runtime without firmware writes**. Accept verified `DeviceEdge`, produce stable KHID `DOWN/UP` or `TRIGGER`, run software mapping/macro actions, and test offline. If only press notifications exist, support trigger-only actions rather than pretending holds work.
5. **Solve Windows side effects**. If the same source sends standard keyboard VKs, identify a safe suppression route; Raw Input by itself **does not suppress**. Output reinjection must avoid loops and release keys on errors. Confirm behavior on real Windows applications before claiming remapping replaces original input.
6. **Expand to M2-M4 and arbitrary firmware editing only after full snapshots**. Snapshot configuration on first detection, correlate device fingerprint + USB port, save *all* readable settings, identify how live macros are preserved, require explicit user authorization and exact prewrite match. Never replay M1's historical nine-frame image into another profile.

### Acceptance test demonstrating the end goal

With the K758 OEM application closed, one chosen physical key yields a **proven device-specific identity**, a single KHID action on press, the appropriate real release (or an explicitly trigger-only action), and **no unexpected original character**. Profiles change the output action without altering that KHID identity. A failed write, disconnect, or restart must not destroy existing onboard macros or leave stuck held keys.

**Do not declare success solely from a USB ACK, a VK event, or an F13 assignment.** All three can happen without knowing which physical K758 key generated the event.
