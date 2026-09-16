#!/bin/sh

set -eu

cd "$(dirname "$0")/.."

cargo build --release --locked --bins --package loom
sudo install -Dm755 target/release/loom target/release/loomctl target/release/loomd -t /usr/bin
sudo install -Dm644 res/loom.desktop /usr/share/applications/loom.desktop
sudo install -Dm644 res/loom.service /usr/lib/systemd/user/loom.service
sudo install -Dm644 res/loom.svg /usr/share/icons/hicolor/scalable/apps/loom.svg
sudo install -Dm644 LICENSE /usr/share/licenses/loom/LICENSE

systemctl --user daemon-reload
systemctl --user enable --now loom.service
systemctl --user start --now loom.service

printf '%s\n' 'Loom is running. Control it via the GUI or CLI controller.'
