# loom

A realtime stereo spatialised spatial audio processor (using crossfeed, decorrelation, early reflections and reverb) built with pipewire-rs and egui.

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
- [ ] Explore more techniques and sound as addictive as [Boom3d](https://www.globaldelight.com/boom/)
- [ ] Non-RT features
