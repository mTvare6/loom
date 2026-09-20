# SPDX-License-Identifier: MPL-2.0

pkgname=loom-git
pkgver=0.1.0.r36.g919da2c
pkgrel=1
pkgdesc='Plug and Play real-time audio enhancer which makes anything sound instantly better'
arch=('x86_64')
url='https://github.com/mTvare6/loom'
license=('MPL-2.0')
depends=(
  'gcc-libs'
  'glibc'
  'libglvnd'
  'libx11'
  'libxcursor'
  'libxi'
  'libxkbcommon'
  'libxrandr'
  'pipewire'
  'wayland'
)
makedepends=('cargo' 'git' 'pkgconf')
provides=("loom=$pkgver")
conflicts=('loom')
options=('!lto')
source=('loom::git+https://github.com/mTvare6/loom.git')
sha256sums=('SKIP')

pkgver() {
  cd loom
  local version
  version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
  printf '%s.r%s.g%s' "$version" "$(git rev-list --count HEAD)" \
    "$(git rev-parse --short=7 HEAD)"
}

prepare() {
  cd loom
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd loom
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --bins --package loom
}

check() {
  cd loom
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo test --frozen --release --workspace
}

package() {
  cd loom
  install -Dm755 target/release/loom target/release/loomctl target/release/loomd \
    -t "$pkgdir/usr/bin"
  install -Dm644 res/loom.desktop "$pkgdir/usr/share/applications/loom.desktop"
  install -Dm644 res/loom.service "$pkgdir/usr/lib/systemd/user/loom.service"
  install -Dm644 res/loom.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/loom.svg"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
