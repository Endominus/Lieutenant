#!/usr/bin/fish
cargo build --release --target x86_64-unknown-linux-gnu
tar -czvf Lieutenant-v$argv-x86_64-unknown-linux-gnu.tar.gz -C target/x86_64-unknown-linux-gnu/release lieutenant lieutenant.db settings.toml
cargo build --release --target x86_64-pc-windows-msvc
$dir = ".\target\x86_64-pc-windows-msvc\release"
$argv = "1.0.1"
7z a -tzip Lieutenant-v$argv-x86_64-pc-windows-msvc.zip $dir\lieutenant.db $dir\settings.toml $dir\lieutenant.exe
