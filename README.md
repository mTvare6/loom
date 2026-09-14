# loom

A real-time stereo spatial audio processor built on PipeWire.

### Libraries

PipeWire devel headers for your distribution.

### Building & Running

```sh
cargo build --release
cargo run --release
```

### Roadmap

- [ ] EQ and presets
  - [ ] Presets
  - [ ] [Controls](https://signalsmith-audio.co.uk/writing/2021/monotonic-smooth-interpolation/) to create preset
- [ ] UX
  - [ ] Persistence
  - [ ] Non-RT features
  - [ ] deamon-mode and IPC support
- [ ] PipeWire features
  - [ ] n.1 input (n > 2)
  - [ ] Variable sample rate
- [ ] Desktop integration
  - [ ] Init systems
  - [ ] Package mangers
  - [ ] Desktop environment-like projects
- [ ] Explore additional spatial-audio techniques

### License

The source code and documentation are licensed under the Mozilla Public License 2.0. See [LICENSE](LICENSE).
