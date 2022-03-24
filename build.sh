#!/usr/bin/fish
cargo build --release --target x86_64-unknown-linux-gnu
tar -czvf Lieutenant-v$argv-x86_64-unknown-linux-gnu.tar.gz -C target/x86_64-unknown-linux-gnu/release lieutenant lieutenant.db settings.toml
cargo build --release --target x86_64-pc-windows-msvc
7z a -tzip Lieutenant-v$argv-x86_64-pc-windows-msvc.zip artifacts\lieutenant.db artifacts\settings.toml .\target\x86_64-pc-windows-msvc\release\lieutenant.exe
