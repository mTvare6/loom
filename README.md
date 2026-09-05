# loom

A realtime stereo spatialised spatial audio processor built with pipewire-rs and egui.

### Libraries

PipeWire devel headers for your distro.

### Building & Running

```sh
cargo build --release
cargo run --release
```

### Usage

Route audio into the `loom_virtual_sink` inputs using [qpwgraph](https://github.com/rncbc/qpwgraph)

### Roadmap

- [ ] Persistence
- [ ] Explore additional spatial-audio techniques
- [ ] Non-RT features
