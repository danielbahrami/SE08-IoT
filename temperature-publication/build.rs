use embuild::cargo::set_rustc_env;
use embuild::kconfig::{try_from_config_file, Value};

fn main() {
    embuild::espidf::sysenv::output();

    match try_from_config_file("kconfig.projbuild") {
        Ok(configurations) => {
            for (key, value) in configurations {
                if let Value::String(string) = value {
                    set_rustc_env(&key, &string);
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to load configurations: {:?}", err);
        }
    }
}
