# One ROM Lab NEW

A new version of One ROM Lab, rebuilt:
- Fire (RP2350) only
- Primary use case is currently reading external ROMs

In time it is expected this will replace the existing Lab implementation.

To use to test a One ROM Fire 40:

```bash
cargo run --release
```

Output serving [images/test/rand_512KB.rom](../../images/test/rand_512KB.rom).

```text
14:47:51.720: INFO  [onerom_lab_fire] -----
14:47:52.666: INFO  [onerom_lab_fire] Reading 27C400 ...
14:47:59.898: INFO  [onerom_lab_fire] 8-bit  SHA1: d98ec9a8375cf3d3000fccdec176849c25feb34e checksum: 0x03FB87C9
14:47:59.898: INFO  [onerom_lab_fire] 16-bit SHA1: d98ec9a8375cf3d3000fccdec176849c25feb34e checksum: 0x03FB87C9
14:47:59.898: INFO  [onerom_lab_fire] Match: true
14:47:59.898: INFO  [onerom_lab_fire] Tristate failures: 8-bit: 0 16-bit: 0
14:47:59.898: INFO  [onerom_lab_fire] -----
```

Dissecting the output:

- Both 8-bit and 16-bit SHA1 and 32-bit summing checksums should match and be the correct value for the ROM being served.

  Timings are relatively aggressive checking that both words and bytes are served.

- Tristate failures should be 0.

  This covers each of /OE and /CE independently being driven high and checking that the data lines are tristated - pulled down using the test board's internal pulls.  The timing for checking tri-stating is relatively relaxed to overcome weak-pulls and any capacitance/inductance of the test setup (e.g. pogo pins).