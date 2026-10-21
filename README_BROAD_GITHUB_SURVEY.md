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

## 10. What to implement next in this Rust repository

**Priority is driven by what's already verified on the actual K758, not how many projects mention HID.**

1. **Firmware storage:** use **Yama + CK108** to port a **bounded, lossless 82-byte sample macro codec** and offline roundtrip tests. We've already decoded the historical M1 blob byte-identically. Only with separate permission should we try K758 `0x09` read-only to back up live macro storage. Track unknown fields and preserve unmodified macros.
2. **Physical input identification:** add a read-only Windows diagnostic informed by **HidForge + K596 G-key bridge + Neuron + WinRawInput**. Observe K758 `MI_00` keyboard and `MI_01/Col04` vendor collections side-by-side; explicitly distinguish real `Down`/`Up` from `Trigger`-only notifications. Do not interpret a firmware-produced VK as a physical slot when duplicates are possible.
3. **Software KHID bridge:** combine **Neuron's one Trigger->Action engine**, **Makima's Rust profile patterns**, and **Input Remapper/HidForge** macro state machines. Allocate stable custom KHID identities from verified device fingerprint + physical slot; use port identity as an attachment factor, not just a short name hash.
4. **Original-key suppression:** prototype **HIDeous-style** Windows Raw Input + keyboard-hook correlation **without suppressing by default**. Test two keyboards, simultaneous keys, mismatched edges and timeouts. Only then allow explicitly scoped suppression. A Windows `WH_KEYBOARD_LL` hook is an alternative with different timeout/attribution tradeoffs; a kernel device filter requires distinct engineering. Fail open (leave input alone) when uncertain.
5. **RGB:** treat it as a **parallel output subsystem**, not as key identification. Read existing K758 `0x05` lighting blocks and snapshot current state. Borrow Yama/CK108 packet/GRB decoding only after live byte comparison; use Neuron's RGB compositor for optional software-driven effects **only when the K758's streaming/per-key RGB writes are independently proven**. No lighting write yet.
6. **Safe M1–M4 editing:** read complete live keymaps + macros first, snapshot before any change, use verified per-bank selection/readback, exact prewrite checks, write only what we can restore, and independently confirm the result. No silent profile overwrites.

**Result we need before claiming success:** press K758 physical key; observe a uniquely sourced event; translate to `KHID:device:slot:edge`; choose a software action; no duplicate original Windows key when replacement is enabled; avoid stuck keys; keep the K758's hardware profiles, stored macros and existing RGB unchanged unless the user specifically authorizes writes.
