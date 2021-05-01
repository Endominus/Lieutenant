#!/usr/bin/fish
cargo build --release --target x86_64-unknown-linux-gnu
tar -czvf Lieutenant-v0.7.0-x86_64-unknown-linux-gnu.tar.gz -C artifacts lieutenant.db settings.toml -C ../target/x86_64-unknown-linux-gnu/release lieutenant
cargo build --release --target x86_64-pc-windows-gnu
tar -cavf Lieutenant-v0.7.0-x86_64-pc-windows-msvc.zip -C artifacts lieutenant.db settings.toml -C ../target/x86_64-pc-windows-gnu/release lieutenant.exe