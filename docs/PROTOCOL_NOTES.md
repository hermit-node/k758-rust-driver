# K758 protocol and test notes

This protocol is verified against the user's Redragon K758, not all devices with USB ID `0C45:5004`.

## Device identity and reports

- VID/PID: `0C45:5004`; USB interface `MI_01`, collection `Col04`, page `FF1C`, usage `0092`.
- Each output report is 64 bytes with report ID `04`.
- Output checksum = 16-bit sum of bytes [3..63], stored little-endian in bytes [1..2].
- Input replies echo the request header/checksum while returning new data. Do not validate their checksum over the returned payload.
- Working Windows HID transport: `hidapi.write` (interrupt output) and `read_timeout` on the vendor collection.

## Captured OEM M1 Apply

Nine packets: `01` BEGIN; `0A` macro 1; `0A` macro 2; `06` profile parameter; four `08` keymap pages at `0000`, `0038`, `0070`, `00A8`; `02` END.

Physical D's three-byte map entry, on keymap packet 6, changed `02 02 04` (A) to `02 02 07` (D). Only two packet bytes changed: zero-based byte 38 `04->07` and checksum byte 1 `7A->7D`. Every other byte including macro events was preserved.

## Device observations, 2026-10-20

- `03` length 44 returns active bank at response offset 18: 00=M1, 01=M2, 02=M3, 03=M4.
- OEM-captured `04` profile-select transaction changed M2 to M1. Readback verified M1.
- `05` length 56 at offsets `0000`, `0040`, `0080`, `00C0` reads four distinct lighting blocks.
- `07` read across offsets `0`, `0x38`, `0x70`, `0xa8` returns K758's **customized active keymap**. All 192 bytes exactly matched the OEM M1 image before restoration.
- `0F` at the same offsets returned a different default-looking keymap. This differs from the `07`/`0F` semantics documented for another Redragon keyboard; the K758's live observations take precedence.
- The M1 write sent nine frames, with nine matching vendor input ACKs.
- A second independent readback showed exactly one modified keymap byte at offset 86: `04->07`. W's macro reference stayed `05 01 00`; all other key assignments remained identical.
- All four lighting blocks matched their prewrite values byte-for-byte.
- Successful USB write and stored-state readback **do not substitute for** a user-observed physical D keypress test. The live macro-table bytes are not independently backed up.

No original G13 code, PySide6 code, or the three earlier Studio distributions were modified.
