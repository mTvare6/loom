#!/bin/sh

set -eu

cd "$(dirname "$0")/.."

cargo build --release --locked --bins --package saq
sudo install -Dm755 target/release/saq target/release/saqctl target/release/saqd -t /usr/bin
sudo install -Dm644 res/com.epestr.saq.desktop /usr/share/applications/com.epestr.saq.desktop
sudo install -Dm644 res/saq.service /usr/lib/systemd/user/saq.service
sudo install -Dm644 res/com.epestr.saq.svg /usr/share/icons/hicolor/scalable/apps/com.epestr.saq.svg
sudo install -Dm644 LICENSE /usr/share/licenses/saq/LICENSE

systemctl --user daemon-reload
systemctl --user enable --now saq.service
systemctl --user start --now saq.service

printf '%s\n' 'Śaq is running. Control it via the GUI or CLI controller.'
