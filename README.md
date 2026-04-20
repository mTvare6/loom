# loom

Real time audio spatialiser/ambience filter for pipewire using a DSP pipeline, with a minimal egui controller inspired by [Boom 3d for macOS](https://www.globaldelight.com/boom/).

### Features
- Linkwitz-Riley crossovers and multiband processing to prevent phase destruction
- Crossfeed, delay line, decorrelation, early reflections, and stereo room tail for spatialising effects
- Side width control with transient aware modulation to prevent mono tracks sounding phase destroyed 

### Building & Running

```sh
cargo build --release
cargo run --release
```

### Usage
- Route audio into the `loom_virtual_sink` using [qpwgraph](https://github.com/rncbc/qpwgraph)
- Adjust a single master control similiar to Boom to adjust intensity of spatiality
