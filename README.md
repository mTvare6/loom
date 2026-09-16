# Śaq

A real-time stereo spatial audio processor built on PipeWire.

### Libraries

PipeWire devel headers for your distribution.

### Running

```sh
# daemon
cargo run --release

# gui client
cargo run --bin saq --release

# cli client
cargo run --bin saqctl --release -- pitch_enabled true
```

### Installing

```sh
./res/install.sh
```

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
