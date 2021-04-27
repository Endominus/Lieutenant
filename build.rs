use std::{env, fs};

fn main() {
    let build = env::var_os("PROFILE").unwrap();
    match build.to_str().unwrap() {
        "release" => { 
            let _a = fs::copy("lieutenant.db", "target/release/lieutenant.db"); 
            let _a = fs::copy("settings.toml", "target/release/settings.toml"); 
        }
        "debug" => { 
            let _a = fs::copy("lieutenant.db", "target/debug/lieutenant.db"); 
            let _a = fs::copy("settings.toml", "target/debug/settings.toml"); 
        }
        _ => {  }
    }
}
