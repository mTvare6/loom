# SPDX-License-Identifier: MPL-2.0

pkgname=saq-git
pkgver=0.1.0.r36.g919da2c
pkgrel=1
pkgdesc='A real-time easy to use audio enhancer for Linux'
arch=('x86_64')
url='https://github.com/mTvare6/saq'
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
provides=("saq=$pkgver")
conflicts=('saq')
options=('!lto')
source=('saq::git+https://github.com/mTvare6/saq.git')
sha256sums=('SKIP')

pkgver() {
  cd saq
  local version
  version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
  printf '%s.r%s.g%s' "$version" "$(git rev-list --count HEAD)" \
    "$(git rev-parse --short=7 HEAD)"
}

prepare() {
  cd saq
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
  cd saq
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --bins --package saq
}

check() {
  cd saq
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo test --frozen --release --workspace
}

package() {
  cd saq
  install -Dm755 target/release/saq target/release/saqctl target/release/saqd \
    -t "$pkgdir/usr/bin"
  install -Dm644 res/com.epestr.saq.desktop "$pkgdir/usr/share/applications/com.epestr.saq.desktop"
  install -Dm644 res/saq.service "$pkgdir/usr/lib/systemd/user/saq.service"
  install -Dm644 res/com.epestr.saq.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/com.epestr.saq.svg"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
