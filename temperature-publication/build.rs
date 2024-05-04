use embuild::cargo::set_rustc_env;
use embuild::kconfig::{try_from_config_file, Value};

fn main() {
    embuild::espidf::sysenv::output();

    let path = "src/kconfig.projbuild";
    if let Ok(configurations) = try_from_config_file(path) {
        for (key, value) in configurations {
            if let Value::String(string) = value {
                set_rustc_env(&key, &string);
            }
        }
    } else {
        eprintln!("Failed to read configurations from '{}'", path);
    }
}
