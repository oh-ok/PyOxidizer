use pyo3_build_config::{InterpreterConfig, get, use_pyo3_cfgs};

// nabbed from pyo3-ffi
fn emit_link_config(interpreter_config: &InterpreterConfig) -> Result<(), &'static str> {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!(
        "cargo:rustc-link-lib={link_model}{alias}{lib_name}",
        link_model = if interpreter_config.shared {
            ""
        } else {
            "static="
        },
        alias = if target_os == "windows" {
            "pythonXY:"
        } else {
            ""
        },
        lib_name = interpreter_config.lib_name.as_ref().ok_or(
            "attempted to link to Python shared library but config does not contain lib_name"
        )?,
    );

    if let Some(lib_dir) = &interpreter_config.lib_dir {
        println!("cargo:rustc-link-search=native={lib_dir}");
    }

    Ok(())
}

fn main() -> Result<(), &'static str>{
    use_pyo3_cfgs();
    emit_link_config(get())
}
