use std::env;
use std::fs;
use std::path::{self, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let host_path_separator_file = out_dir.join("host_path_separator");
    let literal = format!("'{}'", path::MAIN_SEPARATOR.escape_default());
    fs::write(host_path_separator_file, literal).unwrap();
}
