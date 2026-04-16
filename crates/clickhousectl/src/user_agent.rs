/// Returns the canonical user-agent string for all outbound HTTP requests.
pub fn user_agent() -> String {
    format!("clickhousectl/{}", env!("CARGO_PKG_VERSION"))
}
