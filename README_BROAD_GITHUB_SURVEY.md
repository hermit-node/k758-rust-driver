# Expanded Redragon GitHub research — assignments, macros, input and KHID

Research performed on **2026-10-20**, using broad GitHub repository queries including "Redragon keyboard", "Redragon", "macro", "remap", and specific USB/protocol terms. **The six projects in [README_REFERENCE_IMPLEMENTATIONS.md](README_REFERENCE_IMPLEMENTATIONS.md) remain included**; this document adds further references, source links, and a better prioritization.

This is a **research record**, not a claim that these drivers can run on our K758. The actual physical K758 experiments are recorded in [README_WHAT_WE_TRIED.md](README_WHAT_WE_TRIED.md).

## 1. Two distinct layers to implement

**Onboard configuration**: read the K758's live hardware profile, 3-byte key assignment, lighting and **macro memory**, then apply a minimal validated update and read it back. This layer runs only when explicitly requested and keeps working when our Rust process exits.

**Runtime KHID interception**: attribute a key event to a specific physical switch, assign it a stable KHID, handle `DOWN`, `UP` or (if hardware provides only one notification) `TRIGGER`, select a software profile/macro, and produce any requested action. This requires a reliable K758 physical-origin signal and avoidance of unintended ordinary Windows keystrokes. It is **not** the same as changing the firmware keymap.

## 2. First priority — related SONiX/Redragon configuration protocol

| Project | Real implementation | What to transfer to Rust | Hardware / caution |
|---|---|---|---|
| **[Redragon Yama control](https://github.com/aldoboomer/redragon-yama-control)** | Read/write complete onboard keymaps `0x07`/`0x08`, macros `0x09`/`0x0A`, config, RGB; backup/restore; macro binary parser | Macro blob codec, bounded 56-byte read chunks, full snapshot and read-before-write, unknown event retention | Redragon K550RGB-1; VID/PID `320F:5000`; different profile count and keymap dimensions |
| **[MotoSpeed CK108](https://github.com/FireDeveloper/CK108)** | Complete hardware-verified SONiX protocol; macros stored, read back and physically played; keymaps and profiles in Python + WebHID configurator | Full protocol tests, typed assignment/macro refs, lossless parsing, UI boundary, independent verification of read command `0x09` | SONiX `0C45:8502`; vendor collection `FF1C:0092`; not our K758 PID |
| **[HIDra](https://github.com/luizjr/HIDra)** | Read/write three-byte key assignments, including keyboard and media usages | Key assignment modeling and editable per-slot codec | K586RGB-PRO, `0C45:5004` also used by several other models; reported `0x07`/`0x0F` semantics differ |
| **[rgb_keyboard](https://github.com/dokutan/rgb_keyboard)** | K758-related family of HID commands and physical offset tables; incomplete macro parser | Command-level comparison, physical slots/option dictionaries | Upstream macros labeled incomplete/broken |
| **[rgb-keyboard-studio](https://github.com/fedecarpi/rgb-keyboard-studio)** | Expanded fork of rgb_keyboard, model profiles and browser/Electron UI; K589/K556/K587/K582 listed with `0C45:5004` | Device-family handling and visual key layouts; avoid VID/PID-only identity | Primarily lighting; not an independently solved K758 macro manager |
| **[k596-signalrgb](https://github.com/peshke/k596-signalrgb)** | USB captures and research for another controller | Careful capture labeling, safe read probes, reply checks | K596 protocol is **different**; its firmware unlock and write commands must never be replayed to K758 |

**Technical match worth preserving:** Both Yama and CK108 use 64-byte packets, report ID `04`, payloads up to 56 bytes, additive little-endian checksum over bytes 3..63, and command families `01`/`02` begin/end, `03` status, `07` read keymap, `08` write keymap, **`09` read macro**, **`0A` write macro**. We have physically verified many corresponding pieces on the K758, *except* `0x09`, which remains a read-only probe hypothesis.

Their keymap table sizes and profile count **must not be transplanted**. The K758's verified active M1 table is **192 bytes** (64 triples), and hardware status reports four M1–M4 banks.

### Macro payload comparison — concrete new offline proof

Our existing K758 historical OEM M1 apply fixture, `fixtures/m1_D_A_to_D_PREVIEW_only.json`, includes two `0x0A` macro writes. Reassembling their payloads at the 16-bit packet offsets `0` and `56` yields one **82-byte** macro blob.

The format described and implemented independently by Yama and CK108 decodes this entire blob correctly:

| Field | Actual K758 captured bytes / decoded value |
|---|---|
| Header magic | `AA 55` (LE `0x55AA`) |
| Declared total | `0x0052` = **82 bytes**, matches actual blob length |
| Macro count | **1** |
| Names flag | **1** |
| Macro 1 offset | **0x0012** = 18, directly after 16-byte header + 2-byte offset table |
| Event count | **12** |
| Repeat count | **1** |
| UTF-16LE name | **Macro1**, length six code units |
| Event data | **12 × 4 bytes** = 48; six key-downs, six key-ups |
| Timing units | Lower 12 bits × 10 ms; captured total 550 ms |
| Exact reconstructed binary roundtrip | **82/82 bytes identical**, independently tested offline |

The event records use kind `2` (keyboard), subcode `2` (HID key usage), and the high bit of a 16-bit event word for key-down. This macro is a sequence of **ordinary HID key events** inside firmware storage; the macro reference (`05 01 00`) is **not** itself what Windows receives.

**This strongly validates the K758 historical capture's macro file format. It does not prove that reading `0x09` on the live K758 succeeds or that an ordinary macro press generates an extra vendor-defined report.**

Sources:
- [Yama `yama/device.py`](https://github.com/aldoboomer/redragon-yama-control/blob/main/yama/device.py), [`yama/macros.py`](https://github.com/aldoboomer/redragon-yama-control/blob/main/yama/macros.py), [macro tests](https://github.com/aldoboomer/redragon-yama-control/blob/main/tests/test_macros.py).
- [CK108 `PROTOCOL.md`](https://github.com/FireDeveloper/CK108/blob/main/PROTOCOL.md), [Python reference](https://github.com/FireDeveloper/CK108/blob/main/ck108/ck108.py), [WebHID macro codec](https://github.com/FireDeveloper/CK108/blob/main/configurator/src/lib/ck108/macros.ts).

## 3. Input-specific project — K596 G-keys, explicitly included

**[Redragon Keyboards G-Keys Remapper](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper)** by `mlkamri` is **one of our highest-value input-source references**, not merely another lighting utility. Inspect [`k596_gkey_bridge.py`](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper/blob/main/k596_gkey_bridge.py).

On that author's **K596 Vishnu**, setting G1–G10 to **"Assign a string"** causes a **vendor-specific HID notification per press**:
- Vendor collection `FF19:FF19` on a different device `062A:8519`.
- Report signature `08 11 C0` through `08 11 C9`, identifying G1–G10.
- The author's code uses Windows `RegisterRawInputDevices`, `WM_INPUT` and `GetRawInputData`, then `SendInput` to synthesize F13–F22.
- **Important:** reports are **press-only**, not independently observed physical DOWN/UP. The repository explicitly warns about unwanted string output when the OEM software is still running.

**The KHID adaptation we actually want:** replace the F13–F22 injection with `vendor report -> known K758 physical slot -> custom KHID TRIGGER`, or `DOWN/UP` *only if the K758 reports both real edges*. A vendor notification could avoid a VK path if the originating switch doesn't also emit normal keyboard events.

**DO NOT assume K596 vendor packet bytes apply to K758**. K758's observed vendor endpoint is `0C45:5004` / `FF1C:0092`, and we have not yet proved a separate vendor keypress event exists there. Test the existing K758 W macro and ordinary keys **read-only**, with concurrent vendor and keyboard input observation.

This K596 project is fundamentally different from the **firmware macro storage** projects above. It is relevant to **physical key event identification**, not downloading firmware macro blobs.

## 4. Software remapping engines: profiles, holds, layers and macros

| Project | What code does | K758 adaptation |
|---|---|---|
| [kekboard-warrior](https://github.com/lucian-labs/kekboard-warrior) (K585) | Linux evdev physical key edges, layers/chords, software macro actions, virtual `UInput` output | Port the independent mapping state machine, **not** Linux evdev APIs |
| [Redragon-keyboard-linux-app](https://github.com/Syozik/Redragon-keyboard-linux-app) | Selects keyboard(s); evdev `grab()` suppresses originals; action macros support `tap`/`down`/`up`/`text`/`delay`; layer key | Useful profile and macro schema, source selection, reconnect protection; Windows needs a different suppression layer |
| [Redragon-Control](https://github.com/wfoyt/Redragon-Control) (K686) | Robust device identity, reversible Linux grab/uinput remapping, stuck-key cleanup and device reconnection | Use the safety invariants, test failure paths; no K758 wire-format compatibility |
| [K617 Fizz AHK macros](https://github.com/sameerasw/REDRAGON-FIZZ-K617-macro) | Windows AutoHotkey v2 hotkeys, chords and text expansions | Output ideas only; Windows VK hotkeys **cannot identify physical device** reliably by themselves |

**Windows-specific guardrail:** `WM_INPUT` can attribute keyboard events to a source device but does **not** suppress normal keyboard output. A Windows low-level keyboard hook can suppress some input but does not itself supply the same raw HID source identity. This needs a careful per-device bridge or alternative hardware-generated non-keyboard signal, not a naive raw-input listener followed by `SendInput`.

## 5. Distinct on-device keymap editors and macro protocols

| Project | Mechanism | Transferable / nontransferable |
|---|---|---|
| [K673 Control](https://github.com/A7ALABS/redragon-k673-macos) | macOS native configurator: read/write key matrix, typed actions, JSON backup and fragment acknowledgments | Excellent backup and command sequencing pattern; but receiver `3554:FA09`, 20-byte `0x13` reports, different action record |
| [fizzctl](https://github.com/ayoubdya/fizzctl) | K617 macro read/write, 8 × 128-byte slots, 2-byte events, keymap slot binding; checks unchanged state | **Preserve-live-table-first** design; K617's 1032-byte wire protocol unrelated to SONiX family |
| [K617 web config](https://github.com/Eric1797/K617-family-keyboard-remapper) | OEM `Cfg.ini` base/Fn keymap changes via OEM Restore | Option-table and Fn-layer design, **not** K758 direct HID write method |
| [redragonKB-remap](https://github.com/octenite/redragonKB-remap) | Earlier K617 OEM configuration editing | Original source of preceding technique; a derivative tool is not independent protocol confirmation |
| [Redragon Ashe profile switcher](https://github.com/gabrielmrp/redragon-keyboard-profile-switcher) | Automates OEM GUI click locations to select hardware profiles | Only GUI reference; no native USB protocol |
| [FTL-RedDragon](https://github.com/kreperu/FTL-RedDragon) | CircuitPython/RP2040 **replacement firmware** scans matrix coords, maps via JSON, and calls Adafruit HID `Keyboard.press/release` | Demonstrates direct physical-source control and separate HID usages (F13–F24); **does not operate the stock K758** |
| [Redragon K617 QMK port](https://github.com/abhijithabhiakl/Redragon_K617-QMK_port) | Stock MCU removed, Pro Micro soldered, QMK firmware flashed to new hardware | Genuine physical-custom-firmware route, **out of scope** for noninvasive K758 software |
| [Sinodragon](https://github.com/EvanSunde/Sinodragon) / [C++](https://github.com/EvanSunde/Sinodragon-cpp) | Reactive Linux RGB lighting; app-specific profile overlay | Later possible RGB side channel, **not KHID** |

Some other "Redragon" results are forks, product installers, RGB-only controllers, mouse drivers or unrelated projects with that name. Searching broadly finds these, but **not every result should become a driver dependency**.

## 6. Practical Rust implementation sequence (updated priorities)

1. **OFFLINE FIRST**: implement a tested and bounded `MacroBlob` parser/encoder using the Yama/CK108 format; exact byte roundtrip for our existing 82-byte K758 M1 capture; preserve every unknown/reserved byte; no HID commands needed.
2. **READ ONLY WITH APPROVAL**: probe K758 `0x09` macro read at length 16 / offset zero. Validate echo, status, magic, bounded declared length and capability before reading further chunks. If anything differs, stop. Save timestamped private raw backup plus SHA256. **Do not enable automatic hardware writes.**
3. **KEYMAP MODEL**: use the verified K758 `0x07` active map, 192-byte raw roundtrip with typed 3-byte key assignments (`02 02 usage`, `05 01 macro_ref`, unknown). Before modifying any of M1–M4, identify how complete live macros/config are preserved.
4. **EVENT-DISCOVERY PHASE**: capture **keyboard + vendor input reports simultaneously** around physically pressed K758 keys, especially W's known macro. Check for K596-style vendor press signals on K758 without assuming `08 11 C0`. Label press-only as `TRIGGER`; never fake hold/release.
5. **KHID RUNTIME**: stable physical identity -> KHID -> independent action profile -> software macros -> Windows output. Cross-reference user-configured device fingerprint + USB port and never conflate VK/firmware-output usage with physical identity.
6. **WINDOWS SUPPRESSION**: only after stage 4 confirms what native input leaks to Windows. Ensure original key isn't also typed, prevent reinjection loops, release held keys on disconnect, test recoverability.
7. **FIRMWARE WRITE GENERALIZATION**: after complete verified snapshots and explicit user-selected target bank, minimal change + full readback + reversible backup. Retain the verified M1 nine-frame write as regression evidence, not a general substitute for live read-modify-write.

**Three crucial don'ts:** do not flash firmware, do not send other-model opcodes blindly, and do not let a K596 press-only report masquerade as a K758 physically held key.


## 7. General-purpose Linux remapping engines (user-suggested)

These **two additional references** were recommended by the project owner. They are not Redragon-specific and do not know the K758's `0x09`/`0x0A` macro storage or physical slot layout. However, they are among the best references for implementing the **KHID runtime's software input/mapping/macro/output pipeline** independently from hardware configuration.

| Project | Existing implementation | How to adapt it to KHID | What must NOT be copied |
|---|---|---|---|
| **[Input Remapper](https://github.com/sezanzeb/input-remapper)** (Linux, Python) | Device-grouped input presets, source capabilities, combination state machine, named output symbols, virtual device forwarding and rich programmable software macro language | Model `KHID(source_identity, physical_slot, edge)` as the source token, use its handler separation (key, combo, macro, passthrough) and robust trigger/held/released state transitions as a design reference | Linux `evdev`, exclusive `grab()`, `uinput` permissions, device nodes and shell integration; these are not Windows APIs |
| **[Makima](https://github.com/cyber-sushi/makima)** (Linux, Rust) | Per-device TOML configs, event-stream translation, combinations, sequences, shell actions, active-application-specific bindings, four selectable layouts, hotplug monitoring and virtual keyboard/mouse/tablet outputs | Port the **conceptual Rust event -> normalized key identity -> mapping lookup -> dispatch** state machine; adapt layouts to K758 profile tracking and device fingerprint + USB port association | Linux `evdev` input/event names, `udev` notifications, `uinput` output and `grab()`; its config identifies devices by name, which is **not unique enough** for our K758 identity |

### Input Remapper: code-level patterns

Source entry points:
- [`inputremapper/injection/injector.py`](https://github.com/sezanzeb/input-remapper/blob/main/inputremapper/injection/injector.py) — source selection, forwarding and virtual-device setup, including explicit device hashes and recognizing forwarded devices.
- [`inputremapper/injection/mapping_handlers/combination_handler.py`](https://github.com/sezanzeb/input-remapper/blob/main/inputremapper/injection/mapping_handlers/combination_handler.py) — per-input pressed state, combination activation/release and avoidance of stray key-up forwarding.
- [`inputremapper/injection/mapping_handlers/macro_handler.py`](https://github.com/sezanzeb/input-remapper/blob/main/inputremapper/injection/mapping_handlers/macro_handler.py) — macro triggers, trigger-release notifications and asynchronous execution.
- [`inputremapper/injection/macros/`](https://github.com/sezanzeb/input-remapper/tree/main/inputremapper/injection/macros) and [macro language documentation](https://github.com/sezanzeb/input-remapper/blob/main/readme/macros.md) — `key`, `key_down`, `key_up`, `wait`, `repeat`, `hold`, `hold_keys`, `toggle` and conditional macros.

**Design we can adopt:**
1. After a device-specific K758 `InputEdge` has been *proven*, normalize it into a KHID key or `TRIGGER`. Do not use a VK as a substitute for unknown physical identity.
2. Resolve the software profile to a `MappingHandler` (passthrough, disabled, remap, chord, macro, command). Track pressed state **per KHID and device source**, not globally per Windows VK.
3. A macro handler should track its *actual* input trigger. A physical **press-only vendor report** can fire `TRIGGER` macros, but `hold_keys` and hold-to-repeat require genuine UP reporting or an explicitly documented alternative policy.
4. For chords, remember which keys' DOWN events were forwarded/suppressed so the corresponding UP events are handled correctly; otherwise stuck keys are possible.
5. On stop, hot-unplug, profile switch or macro cancellation, release every synthesized held output exactly once.
6. Tag/reject synthetic output when it loops back into our listener; don't recursively remap our own injections.

This is a **software macro interpreter**, not a K758 EEPROM macro codec. The two macro formats should share a higher-level action/event model but should **not** be conflated byte-for-byte. Copying source code directly also entails respecting each project's applicable software license; independent Rust implementations of the ideas are preferable.

### Makima: Rust event pipeline and profile handling

Source entry points:
- [`src/config.rs`](https://github.com/cyber-sushi/makima/blob/main/src/config.rs) — parsed configurations, mapping lookup, modifiers, event variants and device-specific bindings.
- [`src/event_reader.rs`](https://github.com/cyber-sushi/makima/blob/main/src/event_reader.rs) — input handling, state, layout switching, action dispatch and active-window conditions.
- [`src/udev_monitor.rs`](https://github.com/cyber-sushi/makima/blob/main/src/udev_monitor.rs) — event device discovery, config association and hotplug.
- [`src/virtual_devices.rs`](https://github.com/cyber-sushi/makima/blob/main/src/virtual_devices.rs) — virtual keyboard, pointer and tablet outputs, **via Linux `uinput`**.
- [`examples/config-keyboard.toml`](https://github.com/cyber-sushi/makima/blob/main/examples/config-keyboard.toml) — user-facing config syntax.

**Proposed Rust adaptation:**
- Make an internal `SourceId` (validated K758 hardware fingerprint, port attachment, HID collection) and `KhidId` (stable physical slot assignment). Give every incoming event a source timestamp and an explicit `Down`, `Up`, `Repeat` or `Trigger`.
- Separate `InputBackend` from `MappingEngine` from `ActionOutput`. Only `InputBackend` is Windows-specific; `MappingEngine` can be tested on synthetic events without hardware.
- Represent K758 M1–M4 as **onboard profile selections**, while keeping software layers/app-specific bindings orthogonal. Switching a local software layer must not automatically write or switch the hardware profile.
- Use Windows-aware foreground-window lookup and device arrival/removal notification, **not** Makima's Linux compositor or udev code.
- Use a **more precise identity than device name**. Two identically named or similarly enumerated keyboards must never be remapped together by accident; the earlier user requirement is hardware fingerprint **plus USB port attachment**.
- For output, first implement tested `SendInput` with injection-loop tagging. If ordinary original K758 key events still reach Windows, a separate device-scoped suppression/filter solution or vendor-only source is needed for genuine replacement. Windows `Raw Input` *observes*, it does not cancel standard keyboard input.

### Where these two fit relative to the K596 vendor-report bridge

[mlkamri's Redragon G-key remapper](https://github.com/mlkamri/Redragon-Keyboards-G-Keys-Remapper) is an **input source example**: a *specific* K596 vendor report `08 11 C0..C9` becomes a G-key press notification. The software synthesizes F13–F22 merely as output so other programs can bind to it.

**Our desired combination:** K596-style *verified K758-specific* physical event capture **if such vendor reports exist** -> independent KHID token -> Makima-style Rust profile/dispatch engine -> Input-Remapper-style robust macro/combination execution -> Windows output backend. The actual K758 vendor event is **still not verified**. If all we can observe is the remapped keyboard usage, two keys with the same usage may be ambiguous; the system must reject claims of physical uniqueness until disproved or resolved.

### Suggested test matrix before declaring "KHID remapping works"

| Test | Correct result |
|---|---|
| Replay genuine `DOWN -> UP` and rapid repeats for the same KHID | Exactly one held output per press, exactly one release; no stuck output |
| Process a vendor `TRIGGER`-only notification | One-shot action; never fabricate real physical UP |
| Press two independent physical keys with identical firmware output usage | Unique KHIDs **only if** source reports actually distinguish the physical switches; otherwise mark ambiguous |
| Assign chord and software macro, switch mapping layers mid-hold | Expected events, output cleaned up correctly |
| Disconnect/reconnect or stop daemon during a held action | All synthesized outputs released; mapping attached to the same device identity and port policy |
| Inject output while listening for inputs | Output tagged or filtered; no recursive remapping |
| Observe K758 ordinary keyboard event through Windows Raw Input | No claim of original-key suppression: original still reaches apps unless separately blocked |
| View/edit M1–M4 mapping in software | No onboard firmware write unless explicitly requested; hardware backup and KHID profile remain independent |

**Practical consequence:** The Windows `InputBackend`/suppression mechanism is the crucial nonportable module; the Rust mapping and macro logic can be implemented and unit-tested now. Neither input-remapper nor Makima makes stock K758 vendor events appear automatically.


## 8. Windows capture and software macro references

### [HIDeous](https://github.com/dero/HIDeous) — the Windows interception/suppression reference

HIDeous is a C++ Windows utility that remaps **different physical keyboards** by combining two streams:

- [`src/app/processing/raw_input_handler.cpp`](https://github.com/dero/HIDeous/blob/main/src/app/processing/raw_input_handler.cpp) parses `WM_INPUT` / `GetRawInputData` to identify the **keyboard device** (`hDevice`) and obtain VK, scan code and extended-key flags.
- [`src/hook_dll/dll_main.cpp`](https://github.com/dero/HIDeous/blob/main/src/hook_dll/dll_main.cpp) installs a global `WH_KEYBOARD` DLL hook. It filters likely-bound VKs/scan codes and asks the main app over `SendMessageTimeout` whether to block the original key.
- [`src/app/processing/macro_executor.cpp`](https://github.com/dero/HIDeous/blob/main/src/app/processing/macro_executor.cpp) correlates the **most recently received** raw-input source and VK with the hook event, then dispatches `keys:`, `text:`, `run:`, `profile:` commands.
- [`docs/KEYPRESS_FLOW.md`](https://github.com/dero/HIDeous/blob/main/docs/KEYPRESS_FLOW.md) documents this hybrid interception.

**What we can use:** the Windows source-attribution plus suppression architecture, opt-in software actions, per-device profiles and synthetic-key tagging. **What we must improve:** matching just one most-recent VK by timestamp is vulnerable to simultaneous/ambiguous input; use a bounded per-source queue, report uncertainty, and refuse unsafe suppression. Its hook code concentrates on **keydown**, not full verified physical UP tracking. Global hook process/desktop privileges and DLL bitness also constrain coverage. A short hash of the Windows device path is not a stable device-unique fingerprint plus port identity. HIDeous contains **no K758 onboard RGB or firmware macro protocol**.

### [Neuron](https://github.com/worflor/neuron) — Rust unified action engine and vendor-held keys

Neuron is a Windows-first **Rust** Razer Synapse replacement, not primarily an AI input recognizer. Important implementation paths:

- [`crates/neuron-core/src/controls.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-core/src/controls.rs): Raw Input HID decoding, device/collection/stream identities, and `HoldEdges`, which derive independent key **DOWN/UP** edges from per-stream held-control snapshots.
- [`crates/neuron-app/src/macrokeys.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-app/src/macrokeys.rs): on **supported Razer devices**, reads vendor `0x04` reports containing currently held macro-key codes and their release. This is a **different device's input report**, not the K758 `0x04` *output* select-profile command.
- [`engine.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-core/src/engine.rs), [`action.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-core/src/action.rs), [`dispatch.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-app/src/dispatch.rs): a shared trigger/action engine, app-aware profiles/layers, macro execution, and held-output cleanup.
- [`hook.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-core/src/hook.rs): a dedicated-thread Windows `WH_KEYBOARD_LL` hook **for specific gaming/system chords**, not yet a universal, device-scoped keyboard suppression engine.
- [`lighting.rs`](https://github.com/worflor/neuron/blob/main/crates/neuron-core/src/lighting.rs): model-independent RGB composition with vendor-specific device transports for effects and full per-LED frames.
- [`docs/STATUS.md`](https://github.com/worflor/neuron/blob/main/docs/STATUS.md) and [`LICENSE.md`](https://github.com/worflor/neuron/blob/main/LICENSE.md): evidence-gated features and source licensing, including project-specific exceptions.

**What we can use:** a single `KHID -> Action` core, segregated HID streams, stream-specific release tracking, separate RGB compositor, disarmed test modes, verified hardware writes. **What it does NOT establish:** that a K758 has a Razer-style vendor held-key stream, or that its Razer RGB protocol can program the K758.

### [WinRawInput](https://github.com/TheLazyTomcat/bnd.WinRawInput) — independent Windows input API reference

[`WinRawInput.pas`](https://github.com/TheLazyTomcat/bnd.WinRawInput/blob/master/WinRawInput.pas) is a Pascal/Delphi declaration library (MPL-2.0) describing `WM_INPUT`, `WM_INPUT_DEVICE_CHANGE`, `RAWKEYBOARD`, `RAWHID`, `GetRawInputDeviceInfo`, input-report buffers and WoW64 memory alignment. Use it to cross-check our `windows-sys` ABI/struct parsing. It does **not** perform RGB control, macro binding or input suppression by itself.

### [HidForge](https://github.com/KernelSpecter/hidforge) — Windows Rust signal learning, macro replay and honest limits

This is a Rust Windows **software macro engine**. Its README explicitly states that it does **not** read/write onboard macro memory and that bindings are **additive**, since Raw Input alone cannot suppress original input.

- [`src/rawinput.rs`](https://github.com/KernelSpecter/hidforge/blob/main/src/rawinput.rs) discovers keyboard/mouse/HID input; dynamically represents signals such as `Key {device,scancode,vk}` or `HidBit {device,byte,bit}`. For eligible generic HID reports, it **XORs adjacent snapshots per device** to identify flipped bits. It excludes noisy digitizer collections. This is an exploratory technique, **not proof that every changed bit represents a physical key**: report IDs, padding, counters, analog movement and coalesced controls require report-aware filtering.
- [`src/macros.rs`](https://github.com/KernelSpecter/hidforge/blob/main/src/macros.rs) defines software action bindings, macro steps and recorder; it records key press/release, scan-code output, mouse actions and delays; stores profiles as JSON. It intentionally does not replay arbitrary vendor `HidBit` signals as known key actions.
- [`src/engine.rs`](https://github.com/KernelSpecter/hidforge/blob/main/src/engine.rs) supports cycles **until released**, **toggle until clicked again**, or **N repeats**, with absolute-deadline timing on a worker thread.
- [`src/clock.rs`](https://github.com/KernelSpecter/hidforge/blob/main/src/clock.rs) implements QPC timing with a hybrid waitable-timer/small spin approach. This is useful for optional high-precision macro replay but not a prerequisite for correctly identifying K758 keys.
- [`src/inject.rs`](https://github.com/KernelSpecter/hidforge/blob/main/src/inject.rs) sends tagged `SendInput` keyboard/mouse events and documents why injected events may be filtered by some games.

**What we can use:** signal-learning diagnostic for vendor input reports, device/collection provenance, macro recorder, real down/up/hold/repeat modes, loop-safe injections, test probe counters. **Caveats:** use careful per-report identity to avoid false HID-bit “buttons,” and do not claim its signal `device` field maps to a stable K758 physical-slot KHID without verification. HIDForge has **no RGB**, **no stock-firmware macro uploader**, and **no input suppression**.

## 9. Feature matrix: input capture, RGB, and macro binding

**HW** = onboard USB configuration for the referenced hardware; **SW** = computer-side software; **Reference** = formats/design, not a running remapper. None of these labels establishes K758 compatibility.

| Repository / model | Input capture | RGB | Key and macro binding | K758 usefulness |
|---|---|---|---|---|
| **K550 Yama** | Hardware configuration reads, not physical event capture | **HW** modes, config, per-key colors (device uses GRB ordering) | **HW** 3-byte keymap, `0x09` macro read, `0x0A` macro write and backup | **Highest** for firmware macros, configs and lighting |
| **MotoSpeed CK108** | USB configuration readback, not device-specific runtime input remapping | **HW** lighting effects/per-key frames | **HW** keymap, macro slot codec and read/write | **Highest** independent SONiX protocol corroboration |
| **HIDra K586** | Device configuration queries | Lighting support | **HW** 3-byte assignments; no finished macro manager identified | Key assignment encoding / active-map warnings |
| **rgb_keyboard / studio fork** | HID configuration and lighting observations | **HW** RGB and layouts | Keymap option tables; upstream macro writing unfinished | K758-family layouts and OEM report structure |
| **K596 G-Key Remapper (mlkamri)** | Windows **vendor HID press notifications** on K596 | None | **SW** G-key source -> F13–F22 taps | Priority K758 vendor-event hypothesis (not yet verified) |
| **k596-signalrgb** | Protocol capture diagnostics | **HW** K596 lighting | No general working macro editor demonstrated | Careful device-specific packet research |
| **K585 kekboard-warrior** | Linux `evdev` keyboard events + grab | None | **SW** layers/chords/macros via `uinput` | Input state machine |
| **K585 Redragon Linux app** | Linux `evdev` + grab | None | **SW** profiles and timed macro actions | Device-specific software binding |
| **K686 Redragon-Control** | Linux device-specific `evdev` | Device-specific RGB commands | **SW** remap, cleanup/hotplug | Reliable device identity and recovery |
| **K673 Control** | Config read/write, not general host interception | **HW** effect/color settings | **HW** typed key assignment and backups | Snapshot/preserve-unknown strategy |
| **K617 fizzctl** | Firmware readback | K617-specific controls | **HW** macro slots and binding | Preserve untouched live state; different wire format |
| **K617 Cfg.ini remappers** | None | None | OEM software configuration-file edits | Per-model option dictionary only |
| **FTL-RedDragon** | DIY RP2040 physical matrix scanning | DIY firmware only | **FW** JSON keymap and HID output | Hardware replacement architecture, not stock K758 |
| **Input Remapper** | Linux `evdev` + `grab()` | None | **SW** chords, conditional macros, hold/release/toggle | Robust macro handlers |
| **Makima** | Linux `evdev`/udev | None | **SW** Rust layouts, app-aware mappings | Rust dispatch and hotplug |
| **HIDeous** | Windows Raw Input **plus suppressible keyboard hook** | None | **SW** keys/text/run/profile actions | **Highest** Windows original-key suppression reference |
| **Neuron** | Windows Raw Input + vendor stream decoder; limited suppression hook | **SW** RGB compositor with Razer HW adapters | **SW** trigger/action/macros; Razer vendor held-key reports | **Highest** unified Rust event/RGB/action architecture |
| **HidForge** | Windows Raw Input, learnable HID bits, scan codes | None | **SW** timed macros, record/playback, trigger profiles | **High** discovery and macro runtime |
| **WinRawInput** | Windows Raw Input Pascal type/ABI declarations | None | None | Read-only Windows ABI reference |
| **[Wyrm](https://github.com/Maxwegy/wyrm)** (Redragon M811 mouse) | Windows Raw Input mouse activity and polling | **HW** mouse-specific effect/color settings | **HW** button assignments and macro slots/readback | Safe configuration approach; incompatible protocol; snapshot lacks macro inventory |
| **[HidRawData](https://github.com/djlastnight/HidRawData)** | Windows Raw Input keyboard/mouse/vendor reports, actual key up/down/repeat | None | None (reader only) | Device-aware event decoding; no suppression |
| **[HIDAPI](https://github.com/libusb/hidapi)** | HID input/output/feature transport, **not** OS keyboard interception | None itself | No K758 codec; transport only | **Already verified** in our Rust K758 HID configuration path |
| **[Spratley HIDra](https://github.com/Spratley/HIDra)** | C++ Windows Raw Input keyboard/gamepad with per-frame key state | None | None | Frame state/edge tracking only; **not** luizjr/HIDra firmware editor |
| **[karstx/hidra](https://github.com/karstx/hidra)** | Pure-Rust HID I/O and report-descriptor parsing | None directly | None directly | Read-only descriptor parser and error-handling reference; keep proven `hidapi` transport |
| **[hidraw-rs](https://github.com/douglaz/hidraw-rs)** | Linux `hidraw` I/O only (not Windows Raw Input) | None | None | Future Linux backend inspiration; unsupported on Windows |


## 10. What to implement next in this Rust repository

**Priority is driven by what's already verified on the actual K758, not how many projects mention HID.**

1. **Firmware storage:** use **Yama + CK108** to port a **bounded, lossless 82-byte sample macro codec** and offline roundtrip tests. We've already decoded the historical M1 blob byte-identically. Only with separate permission should we try K758 `0x09` read-only to back up live macro storage. Track unknown fields and preserve unmodified macros.
2. **Physical input identification:** add a read-only Windows diagnostic informed by **HidForge + K596 G-key bridge + Neuron + WinRawInput**. Observe K758 `MI_00` keyboard and `MI_01/Col04` vendor collections side-by-side; explicitly distinguish real `Down`/`Up` from `Trigger`-only notifications. Do not interpret a firmware-produced VK as a physical slot when duplicates are possible.
3. **Software KHID bridge:** combine **Neuron's one Trigger->Action engine**, **Makima's Rust profile patterns**, and **Input Remapper/HidForge** macro state machines. Allocate stable custom KHID identities from verified device fingerprint + physical slot; use port identity as an attachment factor, not just a short name hash.
4. **Original-key suppression:** prototype **HIDeous-style** Windows Raw Input + keyboard-hook correlation **without suppressing by default**. Test two keyboards, simultaneous keys, mismatched edges and timeouts. Only then allow explicitly scoped suppression. A Windows `WH_KEYBOARD_LL` hook is an alternative with different timeout/attribution tradeoffs; a kernel device filter requires distinct engineering. Fail open (leave input alone) when uncertain.
5. **RGB:** treat it as a **parallel output subsystem**, not as key identification. Read existing K758 `0x05` lighting blocks and snapshot current state. Borrow Yama/CK108 packet/GRB decoding only after live byte comparison; use Neuron's RGB compositor for optional software-driven effects **only when the K758's streaming/per-key RGB writes are independently proven**. No lighting write yet.
6. **Safe M1–M4 editing:** read complete live keymaps + macros first, snapshot before any change, use verified per-bank selection/readback, exact prewrite checks, write only what we can restore, and independently confirm the result. No silent profile overwrites.

**Result we need before claiming success:** press K758 physical key; observe a uniquely sourced event; translate to `KHID:device:slot:edge`; choose a software action; no duplicate original Windows key when replacement is enabled; avoid stuck keys; keep the K758's hardware profiles, stored macros and existing RGB unchanged unless the user specifically authorizes writes.


## 11. [Wyrm](https://github.com/Maxwegy/wyrm): Windows Redragon mouse configuration, RGB, onboard macros

Wyrm is a **Windows/PySide6, Python-based HID controller for multiple mouse brands**, with Redragon M811 PRO as the author's substantially verified SinoWealth target. The project handles DPI, buttons, macro slots, RGB, readback, profiles and app-aware automatic switching. It is **not** a K758 driver and is **not** a Windows keyboard suppression engine.

### Input capture — activity and rate measurement, NOT KHID identity

[`wyrm/core/rawinput.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/core/rawinput.py) implements one reference-counted **Raw Input mouse** listener shared by the Qt activity badge and polling-rate tester. It registers Generic Desktop page/usage `01:02` with `RegisterRawInputDevices`, receives `WM_INPUT`, reads `RAWINPUTHEADER.hDevice`, resolves device paths through `GetRawInputDeviceInfoW`, extracts VID/PID, emits throttled device *activity* notifications and timestamps for rate measurements. It **does not decode individual keyboard switch events, identify a K758 physical key, block an original VK or bind macros to raw-input edges**.

**Applicability:** reuse the single-owner Raw Input registration pattern and device-arrival provenance model; extend to K758 keyboard and vendor input collections while retaining report ID, source path/handle and timestamp. VID/PID and device activity alone are insufficient KHID physical-slot identities. For keyboard suppression, continue investigating [HIDeous](https://github.com/dero/HIDeous), not Wyrm's activity hub.

### RGB — firmware effect/configuration codec, separate from event handling

[`wyrm/drivers/sinowealth/codec/settings.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/drivers/sinowealth/codec/settings.py) decodes an **onboard 154-byte settings block** into LED effect, color, effect speed/brightness and DPI-stage colors. The codec copies the original buffer and patches only known offsets; unknown bytes are preserved. Its model-specific JSON layout describes offsets and which effect fields are proven. The author treats **streaming, steady and breathing** as verified effects; other effects are experimental.

**Applicability:** build a lossless K758 `LightingConfig` codec, preserving unknown bits/bytes, with explicit per-feature verification gates and a separate optional software RGB renderer. Our K758 **`0x05` lighting-block read** is verified, but a corresponding safe RGB write and per-key streaming protocol are **not yet verified**. The SinoWealth mouse's lighting offsets, commands and RGB ordering must **not** be applied to K758.

### Onboard key assignments — typed action -> stored hardware slot

[`wyrm/drivers/sinowealth/codec/buttons.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/drivers/sinowealth/codec/buttons.py) uses a **4-byte button action** inside an **88-byte button block**. It encodes mouse buttons (`11`), HID keyboard plus modifiers (`21`), consumer controls (`22`), DPI, fire, special actions and a macro reference (`70 <macro-index> <mode> <count>`). [`wyrm/drivers/sinowealth/driver.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/drivers/sinowealth/driver.py) constructs a complete desired button block, compares it with a newly read live block, and skips unchanged writes.

**Applicability:** borrow the action enum/model and encode/decode/compare design. Do **not** copy 4-byte action codes or macro slot numbers to K758: K758 stores **3-byte keymap triples**, including `02 02 07` for D and `05 01 00` for an M1 macro reference. Preserve unknown K758 triples exactly; check whole-bank prewrite state and avoid altering unrelated keys.

### Onboard macros — explicit event codec and read-before-write

[`wyrm/drivers/sinowealth/codec/macro.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/drivers/sinowealth/codec/macro.py) encodes keyboard down/up and mouse down/up with a **three-byte** event `(command, delay_ms_after, code)`. It uses command bytes `50/D0` for keyboard and `10/90` for mouse events; delays longer than 255 ms use filler events. A button's macro action points to a firmware macro slot. In [`driver.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/drivers/sinowealth/driver.py), `apply` uses `read_macro` to **skip an unchanged macro**, then `write_macro` for changes. This is independent **firmware macro binding**, not a software macro playback daemon.

**Applicability:** maintain a high-level event model (`KeyDown`/`KeyUp`/`MouseDown`/`MouseUp`/`Delay`/`Repeat`) with a **K758-specific binary serializer**. For the actual K758 captured 82-byte `0x0A` macro blob, use the already validated **Yama/CK108 16-byte header, offset table, and four-byte events**, **not** Wyrm's unrelated 3-byte SinoWealth encoding. A safe macro writer first requires independently verifying K758 live macro reading (`0x09` is a hypothesis, not yet tested).

### Profiles, backups, and an important incompleteness

[`wyrm/core/profile_manager.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/core/profile_manager.py) maintains PC-side activity profiles and app/foreground rules and writes the relevant settings to each supported mouse when switched. [`driver.py`](https://github.com/Maxwegy/wyrm/blob/main/wyrm/drivers/sinowealth/driver.py) performs: link check -> fresh settings/buttons read -> patch only known bytes -> skip no-op -> write macros then settings/buttons -> read back/compare. It also requires user consent before first write, gates experimental features, and supports dry runs.

**Crucial limitation discovered in source:** `read_state()` returns a snapshot of **`settings.bin` and `buttons.bin` only**, and `restore()` likewise restores only those two blocks. It does **not** save/restore a complete array of live macro slots, despite the separate macro read/write functions. We should **not** describe this as a complete device memory backup or replicate that omission.

**KHID usage:** emulate its device-family adapter boundary, original-byte preservation, dry run, per-device permission, model allowlist and verify-after-write, but require the **full retrievable macro inventory** (or mark the snapshot explicitly partial) before general K758 firmware remapping. PC-side profile switches must not silently rewrite hardware M1–M4 on every application change; software KHID profiles and firmware banks are different constructs.

### Strong protocol incompatibility — do not send mouse packets to K758

Wyrm's Redragon mouse uses VID/PID `258A:002E` cable or `258A:002F` receiver, with report `05` command channel, report `08` block transfers (up to 519-byte feature report), and 154-byte settings / 88-byte button blocks. See [`docs/protocols/sinowealth-wireless.md`](https://github.com/Maxwegy/wyrm/blob/main/docs/protocols/sinowealth-wireless.md).

Our K758 uses VID/PID `0C45:5004`, vendor collection `FF1C:0092`, 64-byte report ID `04`, and the independently verified K758 read/write command family. The fact both devices are Redragon and use HID does **not** make their command bytes, RGB offsets, event records or macro storage compatible.

**Interesting KHID-related precedent:** Wyrm's mouse-specific vendor profile-cycle assignment originally depended on OEM software; Wyrm maps its PC-side profile-next/previous actions to **F24/F23** as software-interpreted markers. This supports the *general pattern* of mapping a physical control to a distinctive host-visible usage. It does **not** prove the K758 has that marker option or a separate vendor-defined press report.

**Bottom line:** Wyrm is a **high-value configuration and safety reference**, a **moderate RGB codec reference**, and only a **low-value physical-key capture/suppression reference**. Use its software architecture and state protection, not its mouse-specific packet format.


## 12. [HidRawData](https://github.com/djlastnight/HidRawData): Windows C# raw keyboard, mouse and vendor HID event reader

This project is a **Windows Raw Input capture/decoding library**, not an onboard keyboard controller. It is implemented in .NET C# (documented as tested on Windows 7/10); its companion visual tool is [HidRawDataWatcher](https://github.com/djlastnight/HidRawDataWatcher).

- [`HidDataReader.cs`](https://github.com/djlastnight/HidRawData/blob/master/HidRawData/Hid/HidDataReader.cs): enumerates device usage page/usage, registers `RIDEV_INPUTSINK`, hooks `WM_INPUT` in a WPF window, and raises events.
- [`HidEvent.cs`](https://github.com/djlastnight/HidRawData/blob/master/HidRawData/Hid/HidEvent.cs): distinguishes `RIM_TYPEKEYBOARD`, `RIM_TYPEMOUSE` and `RIM_TYPEHID`, obtaining source `hDevice`, VK/scan code/extended key flags, modifiers, usage information, and raw HID report data. Importantly, `RAWHID.dwCount` may contain **multiple** reports in one event; it iterates that count instead of assuming one payload.
- [`HidHandler.cs`](https://github.com/djlastnight/HidRawData/blob/master/HidRawData/Hid/HidHandler.cs): tracks actual key-down and key-up, pressed-key state, modifiers and repeat events, with optional synthesized repeats.
- [`RawInputHelper.cs`](https://github.com/djlastnight/HidRawData/blob/master/HidRawData/Hid/RawInputHelper.cs) and [Win32 Raw Input declarations](https://github.com/djlastnight/HidRawData/tree/master/HidRawData/Win32/Win32RawInput): enumerate HID collections, discover device identity/descriptor capabilities and parse `GetRawInputData` buffers.

**K758 applicability:** high for building a **single read-only Rust event sink** covering K758 `MI_00` keyboard and `MI_01` vendor collection. Retain source/collection, report ID, timestamp, keyboard VK/scancode when present, and genuine down/up versus repeat. Derive KHID from a verified physical source; keyboard VK alone is **not** proof of which physical switch generated it, especially with onboard remapping. Validate all buffer/count bounds; handle reconnect and release any software-held keys.

**Limits:** the library does **not** suppress Windows keyboard input, provide K758 RGB, program onboard macros, or guarantee that the K758 produces a per-physical-key vendor report. Windows Raw Input registrations are by usage **per process**; own/fan out registration instead of letting multiple listeners compete for the same usage.

## 13. [HIDAPI](https://github.com/libusb/hidapi): the transport we already proved on K758

**This is not another remapping engine.** HIDAPI is the cross-platform C library for USB/Bluetooth **HID device I/O**: enumerate devices, open a specific HID interface/collection, read input reports, write output reports, and exchange feature reports. Windows backend uses `hid.dll`; Linux has `hidraw` and `libusb` backends; macOS uses `IOHidManager`.

**Our existing Windows Rust project already uses the Rust `hidapi` crate** (`Cargo.toml` `hidapi = "2.6.7"` with `windows-native`), backed by HIDAPI's Windows-native backend. No new transport process is needed.

Specific upstream reference points:
- [`hidapi/hidapi.h`](https://github.com/libusb/hidapi/blob/master/hidapi/hidapi.h): `hid_enumerate` exposes interface number, usage page and usage; `hid_open_path` selects a discovered interface; `hid_write` sends an output report; `hid_read_timeout` waits for an input report; `hid_send_feature_report`/`hid_get_feature_report` handle *feature* reports; some versions provide report-descriptor access.
- [`windows/hid.c`](https://github.com/libusb/hidapi/blob/master/windows/hid.c): native Windows output path (`hid_write` uses overlapped `WriteFile`); feature reports use `HidD_SetFeature` and other Windows HID APIs. These are **different report types**, not interchangeable command channels.
- [`hidtest/test.c`](https://github.com/libusb/hidapi/blob/master/hidtest/test.c): example enumeration and read/write diagnostics. Upstream explicitly warns that blind writes can damage devices.

**Verified on our K758:** USB VID/PID `0C45:5004`, vendor-defined interface `MI_01 / Col04` usage `FF1C:0092`, report ID `04`, 64-byte **output** packets. The working Rust path is `HidApi::new` + **select exact enumerated collection** + `HidDevice::write(&[u8; 64])` + `read_timeout` to verify ACKs. A nine-frame M1 transaction wrote the keymap and was independently read back. Every outgoing request uses the **device-specific 16-bit additive checksum** on bytes `[3..63]`; responses echo the original request header/checksum and must be validated accordingly.

**Transport subtlety:** although earlier *hand-written* Windows `WriteFile` attempts failed, this does **not** mean the `WriteFile` API cannot work: upstream HIDAPI's **working `hid_write` implementation itself uses `WriteFile`** with its own valid HID handle, framing and overlapped-I/O handling. The important proof is *our exact HIDAPI invocation* on the correct collection, not that Windows `WriteFile` is intrinsically incompatible.

**K758 application boundaries:**
1. Keep **one Rust HIDAPI transport** for **explicit configuration** reads/writes (`0x03` status, `0x05` lighting reads, `0x07` active keymap reads, historical `0x08` keymap and `0x0A` macro uploads, plus tentative read-only `0x09` macro probe). Require model/collection verification, 64-byte packet layout, byte-for-byte preflight and independent post-write readback.
2. Keep **Windows Raw Input `WM_INPUT`** as a distinct **runtime event capture path**: device-source keyboard/VK/scan events and any actual vendor *input* notifications. HIDAPI does **not** replace Windows system keyboard interception or per-device original-key suppression.
3. Keep **KHID** a third layer: decoded and verified physical source -> stable KHID identity -> profile/action/macro dispatch. HIDAPI alone cannot invent arbitrary native KHID report types or make a device emit unique physical-key codes.
4. Never use `hid_send_feature_report` for our proven `hid_write` output report just because both are HID; never send arbitrary reports or another model's command. The OEM application must be closed during our K758 configuration I/O.

| Capability | HIDAPI alone | Proven in our K758 driver? |
|---|---|---|
| Enumerate/open a specific HID collection | **Yes** | **Yes**, exact vendor collection |
| Read/write device input/output reports | **Yes** | **Yes**, `hid_write` + `read_timeout`, nine ACKs |
| Access feature reports | **Yes**, if device supports them | **No need** for successful M1 path |
| Interpret K758 keymap/macros/RGB | **No** — protocol belongs to our K758 layer | Keymap and lighting read yes; historical macro blob decoded offline |
| Observe ordinary keyboard presses and suppress original Windows VK | **No** | **Not implemented**; use Windows Raw Input and separate suppression design |
| Assign a new KHID output directly in firmware | **No** | **Not proven** |

**Practical outcome:** keep `hidapi`, consolidate the duplicated Rust query/transport functions, and add a separate Windows input-capture backend. This is a compatibility/foundation reference, **not a replacement** for HIDeous, HidRawData or Neuron.


## 14. [Spratley/HIDra](https://github.com/Spratley/HIDra): C++ Windows Raw Input and per-frame keyboard/gamepad state

**Do not confuse this project with [luizjr/HIDra](https://github.com/luizjr/HIDra).** Despite sharing a name, they are different repositories with different purposes. Spratley's HIDra is a work-in-progress **C++ real-time input library**, with Windows `Raw Input`, gamepad and keyboard polling, and Emscripten/WASM support. It is not Redragon firmware software.

- [`Core/HIDraCore_Windows.cpp`](https://github.com/Spratley/HIDra/blob/main/Core/HIDraCore_Windows.cpp): subclasses a host application's window procedure, registers Generic Desktop keyboard and gamepad collections via `RegisterRawInputDevices`, retrieves `WM_INPUT` using `GetRawInputData`, and forwards keyboard VK/flags or gamepad reports into handlers. Its existing keyboard path passes `VKey` plus flags to a virtual-keystroke parser, **not a durable physical K758 switch ID**. This implementation is intended to run inside the application with an existing `HWND`, not as a background per-device interception service.
- [`Keyboard/HIDraKeyboardHandler.cpp`](https://github.com/Spratley/HIDra/blob/main/Keyboard/HIDraKeyboardHandler.cpp): stores current key state plus a second bitfield indicating key transitions in the current application frame. `GetKey` means held; `GetKeyDown` means newly pressed; `GetKeyUp` means newly released. `Flush` clears transition flags after a frame.
- [`Platforms/Windows/Gamepad/HIDraGamepadDecoder_Windows.cpp`](https://github.com/Spratley/HIDra/blob/main/Platforms/Windows/Gamepad/HIDraGamepadDecoder_Windows.cpp): normalizes particular HID gamepad report shapes into buttons and axes; an illustration of **device-specific report decoder -> normalized state**, but it does not decode K758 input reports.
- [README](https://github.com/Spratley/HIDra): identifies known limitations including device hotplug handling and left/right Shift distinction (scan-code parsing required).

**KHID relevance:** reasonable reference for `raw event -> normalized current held state + transitions`, typed input-source adapters, and per-frame consumers. Rust's KHID core should track edge state by **device identity + verified physical key**, not just aggregate VK state. A UI frame-flush model must **not discard events** between frames; use an ordered event queue and independent release handling in the service, then expose optional frame snapshots to GUI clients.

**RGB / macro binding:** neither stock-firmware RGB commands, macro upload/download, nor a high-level macro action engine was found. It does not suppress default Windows typing. For K758 configuration use **Yama + CK108 + our HIDAPI Rust driver**, and for Windows event capture use **HidRawData + HidForge + Neuron**; for safe replacement/suppression investigate **HIDeous**.

### Consolidated conclusion after the full user-supplied reference list

| Subsystem | Best available code references | First verifiable K758 milestone |
|---|---|---|
| USB HID configuration | HIDAPI (already used) + Yama + CK108 | Lossless macro codec test, then separately authorized read-only `0x09` investigation |
| Physical key discovery | K596 vendor-event bridge + HidForge + HidRawData + Neuron | Trace physical W and an ordinary key with **both** K758 keyboard and vendor HID streams |
| Stateful KHID events | Neuron held-edge state + Spratley HIDra state + Input Remapper handlers | Real `DOWN/UP` vs `TRIGGER` distinction; correct release on disconnect |
| Windows suppression | HIDeous correlation and hook architecture | Original VK does **not** leak during explicitly armed remap, while unrelated keyboards still type normally |
| Software macro engine | Input Remapper + Makima + HidForge | Bind custom KHID to timed press/release/macro, with no recursion or stuck keys |
| Firmware keymap/macro writing | Yama + CK108 + K758 captured transactions + Wyrm's safety boundaries | Preserve *live* keymap, all retrievable macro data, and all four lighting blocks before any general write |
| RGB | Yama/CK108 device codecs + Neuron compositor + Wyrm's byte-preserving config strategy | Decode actual K758 `0x05` lighting data; verify writes separately, never guess |

**This survey documents source mechanisms, not a claim that these repositories can already be combined or that KHID interception has been implemented.** The K758 Rust repository's physically verified success remains the M1 keymap restoration via HIDAPI.


## 15. [karstx/hidra](https://github.com/karstx/hidra): alternative pure-Rust HID transport and descriptor parser

There are now **three distinct "HIDra" projects** in this survey:

1. [luizjr/HIDra](https://github.com/luizjr/HIDra) — Redragon K586-family **onboard keymap/configuration** (TypeScript/Rust app).
2. [Spratley/HIDra](https://github.com/Spratley/HIDra) — C++ **Windows Raw Input** keyboard/gamepad polling and per-frame edges.
3. **[karstx/hidra](https://github.com/karstx/hidra)** — a **pure-Rust, cross-platform HID device I/O library**, an architectural alternative to the already working [libusb/hidapi](https://github.com/libusb/hidapi) + Rust `hidapi` crate.

The karstx library's [README](https://github.com/karstx/hidra) describes an asynchronous API (with native blocking `.wait()`) across Windows (`hid.dll`/`SetupAPI`), Linux `hidraw`, macOS, and browser WebHID. Optional [`nusb` backend](https://github.com/karstx/hidra/blob/master/src/backend/nusb.rs) performs raw USB operations and can take exclusive ownership of a USB interface, unlike the normal OS HID path.

**Source details and K758 applicability:**
- [`src/backend/windows.rs`](https://github.com/karstx/hidra/blob/master/src/backend/windows.rs): Windows HID enumeration, overlapped `ReadFile`/`WriteFile` and feature report I/O, configurable write timeout, cancellation/disconnection handling and descriptor reconstruction through documented `HidP` capabilities. Potentially useful transport error-handling patterns.
- [`src/device_info.rs`](https://github.com/karstx/hidra/blob/master/src/device_info.rs): device interface paths, usage page, usage and interface number. Fits our requirement to explicitly select K758 `MI_01` / `FF1C:0092`, but a path or VID/PID alone is **not** a permanent device fingerprint.
- [`src/descriptor/parse.rs`](https://github.com/karstx/hidra/blob/master/src/descriptor/parse.rs) and [`src/report_info.rs`](https://github.com/karstx/hidra/blob/master/src/report_info.rs): standalone HID **report-descriptor** model: collections, report IDs, usages, input/output/feature fields, bit widths and offsets. This is the **most directly useful additional feature** for offline K758 HID report inspection, particularly distinguishing a vendor input report from a K758 config output report. Descriptors describe the *structure* and *capabilities* of reports, not undocumented proprietary command semantics or exact physical key identities.
- [`src/descriptor/build.rs`](https://github.com/karstx/hidra/blob/master/src/descriptor/build.rs): report-descriptor construction useful for synthetic/mock HID test fixtures. Building a descriptor does **not** reprogram the stock K758's firmware/USB descriptor.

| Area | What karstx/hidra does | K758 conclusion |
|---|---|---|
| **Input capture** | Reads input reports from an opened HID device; parses descriptors | **Useful** for vendor report decoding and mock tests; **not** a Windows `WM_INPUT` interceptor for ordinary keyboard event attribution |
| **RGB** | Raw HID I/O only, no K758 effect/LED commands | **No direct support**; the K758 protocol codec remains our responsibility |
| **Macro binding** | No onboard K758 macro format, keymap decoder or software action engine | **No direct support**; use Yama/CK108 for firmware format and Neuron/HidForge for runtime mapping |
| **Windows suppression** | None | Does not block the K758's original keyboard output |
| **Firmware writes** | Generic HID output/feature transfer | Technically possible, but **unverified on our hardware**, unlike existing Rust `hidapi` path |

**Recommendation:** **Do not replace `hidapi`** in our K758 Rust driver merely because another library is written in pure Rust. Our `hidapi::write` + `read_timeout` path has already passed nine hardware ACKs and independent readback on M1. We can borrow karstx/hidra's **descriptor parser** or test-fixture design as a separate, bounded utility and compare it against the actual K758 descriptor **read only**, without changing the proven transport. The optional raw USB `nusb` backend may claim an interface and require driver changes; it is **not** a safe drop-in Windows alternative for this keypad.

For actual source-aware event interception see [HidRawData](https://github.com/djlastnight/HidRawData), [HidForge](https://github.com/KernelSpecter/hidforge) and [HIDeous](https://github.com/dero/HIDeous). A descriptor or a vendor report can only be translated to a physical KHID if a **specific switch-to-report relationship is independently demonstrated**.


## 16. [douglaz/hidraw-rs](https://github.com/douglaz/hidraw-rs): Linux-only Rust HID transport

This is a **Linux hidraw API/transport** library: enumerate and open `/dev/hidraw*`, exchange input/output/feature reports through kernel interfaces, handle timeouts and optionally use Tokio async calls. Its [README](https://github.com/douglaz/hidraw-rs) and [implementation status](https://github.com/douglaz/hidraw-rs/blob/master/IMPLEMENTATION_STATUS.md) explicitly say **Windows is unsupported**.

- [`src/hidraw/device.rs`](https://github.com/douglaz/hidraw-rs/blob/master/src/hidraw/device.rs) and [`src/hidraw/ioctl.rs`](https://github.com/douglaz/hidraw-rs/blob/master/src/hidraw/ioctl.rs) implement Linux kernel HID I/O; [`src/async_io.rs`](https://github.com/douglaz/hidraw-rs/blob/master/src/async_io.rs) adds optional async behavior.
- [`hidapi-compat`](https://github.com/douglaz/hidraw-rs/tree/master/hidapi-compat) exposes many familiar Rust `hidapi` API shapes, but is **not** a Windows replacement merely because its method names match.
- [`src/protocol/framing.rs`](https://github.com/douglaz/hidraw-rs/blob/master/src/protocol/framing.rs) illustrates packet framing for another device family, not the K758's OEM protocol.

**Input capture:** only Linux HID reports, no Windows Raw Input `WM_INPUT` and no keyboard suppression. **RGB:** no K758 LED protocol. **Macros/key binding:** neither a K758 macro codec nor a software mapping/action engine. **K758 use:** optional inspiration if adding a Linux backend someday, **not** an immediate change to our tested Windows transport.

**Decision:** retain the proven `hidapi` crate with `windows-native`. Our successful K758 M1 transaction used `hidapi::write`, nine verified ACKs, and independent configuration readback. The Linux-only hidraw-rs alternative provides no benefit to the Windows-specific immediate KHID work.
