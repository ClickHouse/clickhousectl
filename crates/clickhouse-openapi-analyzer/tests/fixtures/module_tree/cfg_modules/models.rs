#[cfg(test)]
mod test_only {
    pub struct TestOnlyModel;
}

#[cfg(any())]
mod missing_never;

#[cfg_attr(not(test), cfg(any()))]
mod missing_via_cfg_attr;

#[cfg(all(unix, windows))]
mod missing_inactive_platform;

#[cfg_attr(test, path = "missing_test.rs")]
#[cfg_attr(not(test), path = "production.rs")]
mod selected;
pub use selected::*;

#[cfg(feature = "deprecated-fields")]
mod deprecated {
    pub struct DeprecatedModel;
}
pub use deprecated::*;

#[cfg(feature = "not-enabled")]
mod feature_gated {
    pub struct FeatureModel;
}
pub use feature_gated::*;

#[cfg(clickhouse_custom)]
mod custom_cfg {
    pub struct CustomCfgModel;
}
pub use custom_cfg::*;

#[cfg_attr(target_family = "unix", path = "platform_unix.rs")]
#[cfg_attr(target_family = "windows", path = "platform_windows.rs")]
#[cfg_attr(
    not(any(target_family = "unix", target_family = "windows")),
    path = "platform_other.rs"
)]
mod platform;
pub use platform::*;
