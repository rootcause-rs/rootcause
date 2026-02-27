use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let host_path_separator_file = PathBuf::from(&out_dir).join("host_path_separator");
    // Detect the host path separator at runtime by examining OUT_DIR, which
    // uses the native separator of the OS the build script is running on.
    // The previous approach used path::MAIN_SEPARATOR, which is a compile-time
    // constant. In distributed build systems where the build script and the
    // compiler may run on different platforms, the compile-time constant can
    // disagree with the paths produced by Location::caller(). Checking OUT_DIR
    // at runtime avoids this mismatch.
    let separator = if out_dir.contains('\\') { '\\' } else { '/' };
    let literal = format!("'{}'", separator.escape_default());
    fs::write(host_path_separator_file, literal).unwrap();
}
