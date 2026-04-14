# loom

Real‑time audio processor built with PipeWire which makes a filter, applying a low‑shelf biquad (bass boost) in a lock‑free ring buffer, and shared atomic state between GUI written in egui and audio processor for lock free parameter config.

### Features
- Low‑shelf biquad filter for bass boost
- egui control panel (master volume + bass)

### Libraries
PipeWire dev headers on your distro

### Building & Running

```sh
cargo build --release
cargo run --release
```

### Usage
- Route audio into the `loom_virtual_sink` using [qpwgraph](https://github.com/rncbc/qpwgraph)
- Adjust master volume and bass in the GUI

### Caveats
- No persistence (settings reset everytime)
