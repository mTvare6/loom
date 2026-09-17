# loom

A plug and play real-time audio enhancer for Linux which makes anything sound instantly better.

### Libraries

PipeWire devel headers for your distribution.

### Running

```sh
# daemon
cargo run --release

# gui client
cargo run --bin loom --release

# cli client
cargo run --bin loomctl --release -- pitch_enabled true
```

### Installing

```sh
./res/install.sh
```

This installs the systemd service and starts the daemon. Start the the GUI with `loom` to play around with the settings.

### Roadmap

- [ ] EQ and presets
  - [ ] Presets
  - [ ] [Controls](https://signalsmith-audio.co.uk/writing/2021/monotonic-smooth-interpolation/) to create custom preset
- [ ] UX
  - [x] Persistence
  - [ ] Non-RT features
  - [x] deamon-mode and IPC support
  - [ ] Configuration files
  - [ ] Detailed IPC error handling
  - [ ] Better command-line parsing
  - [ ] Colors
- [ ] PipeWire features
  - [ ] n.1 input (n > 2)
  - [ ] Variable sample rate
- [ ] Desktop integration
  - [x] Init system integration
  - [ ] Package mangers support
  - [ ] Desktop environment-like projects
- [ ] Explore additional spatial-audio techniques

### License

The source code and documentation are licensed under the Mozilla Public License 2.0. See [LICENSE](LICENSE).
