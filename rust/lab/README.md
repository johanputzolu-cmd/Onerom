# One ROM Lab

One ROM Lab firmware reads ROM images from external ROM chips (originals, One ROMs, and other replacements), by using the One ROM hardware with a female socket placed on top.  It can also be used to instrument the performance of these external ROM chips, by utilising additional equipment, such as logic analyzers or oscilloscopes.

It is currently hardcoded to run on an STM32F405RGT6, although it should be possible to adapt it to other STM32F4 variants with minimal changes.

## Building

From **this** directory run:

```bash
cargo build --release
```

## Running/Flasing

From **this** directory run:

```bash
DEFMT_LOG=info cargo run --release
```
