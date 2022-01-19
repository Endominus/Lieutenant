#!/usr/bin/fish
cargo build --release --target x86_64-unknown-linux-gnu
tar -czvf Lieutenant-v$argv-x86_64-unknown-linux-gnu.tar.gz -C artifacts lieutenant.db settings.toml -C ../target/x86_64-unknown-linux-gnu/release lieutenant
cargo build --release --target x86_64-pc-windows-msvc
tar -cavf Lieutenant-v$argv-x86_64-pc-windows-msvc.zip -C artifacts lieutenant.db settings.toml -C ../target/x86_64-pc-windows-msvc/release lieutenant.exe
