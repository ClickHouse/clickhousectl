fn main() {
    for (cargo_var, rust_var) in [
        ("CARGO_CFG_TARGET_ENV", "ANALYZER_TARGET_ENV"),
        ("CARGO_CFG_TARGET_VENDOR", "ANALYZER_TARGET_VENDOR"),
    ] {
        println!("cargo::rerun-if-env-changed={cargo_var}");
        println!(
            "cargo::rustc-env={rust_var}={}",
            std::env::var(cargo_var).expect("Cargo must provide target cfg values")
        );
    }
}
