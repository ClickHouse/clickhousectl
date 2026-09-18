pub struct Widget {
    #[serde(rename = "itemCount", default)]
    pub item_count: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf: Option<WidgetLeaf>,
}

mod nested {
    pub struct WidgetLeaf {
        pub name: String,
    }
}

pub use nested::*;

pub enum WidgetState {
    #[serde(rename = "ready")]
    Ready,
    #[serde(untagged)]
    Unknown(String),
}

impl WidgetState {
    pub const VALUES: &'static [&'static str] = &["ready"];
}

impl Default for WidgetState {
    fn default() -> Self {
        Self::Ready
    }
}

pub type WidgetAlias = Vec<Widget>;
