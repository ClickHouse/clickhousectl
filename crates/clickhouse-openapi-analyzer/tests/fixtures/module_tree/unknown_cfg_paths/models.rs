mod inline {
    #[cfg_attr(clickhouse_custom, path = "custom_enabled.rs")]
    #[cfg_attr(not(clickhouse_custom), path = "custom_disabled.rs")]
    mod selected;

    pub use selected::*;
}
pub use inline::*;
