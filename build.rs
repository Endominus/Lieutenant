use std::path::Path;
use std::{env, fs};

fn main() {
    let build = env::var_os("PROFILE").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let anc = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();

    match build.to_str().unwrap() {
        "release" => {
            let db_path = anc.join("lieutenant.db");
            let settings_path = anc.join("settings.toml");
            let _a = fs::copy("artifacts/lieutenant.db", db_path);
            let _a = fs::copy("artifacts/settings.toml", settings_path);
        }
        "debug" => {
            if !Path::new("target/debug/lieutenant.db").exists() {
                let _a = fs::copy("artifacts/lieutenant.db", "target/debug/lieutenant.db");
                let _a = fs::copy("artifacts/settings.toml", "target/debug/settings.toml");
            }
        }
        _ => {}
    }
}
