use std::{env, fs};
use std::path::Path;

fn main() {
    let build = env::var_os("PROFILE").unwrap();
    match build.to_str().unwrap() {
        "release" => { 
            let _a = fs::copy("lieutenant_default.db", "target/release/lieutenant.db"); 
            let _a = fs::copy("settings_default.toml", "target/release/settings.toml"); 
        }
        "debug" => { 
            if !Path::new("target/debug/lieutenant.db").exists() { let _a = fs::copy("lieutenant_default.db", "target/debug/lieutenant.db"); }
            let _a = fs::copy("settings_default.toml", "target/debug/settings.toml"); 
        }
        _ => {  }
    }
}
