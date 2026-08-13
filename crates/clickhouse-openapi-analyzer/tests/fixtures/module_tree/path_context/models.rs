#[path = "model_files/direct.rs"]
mod direct;
pub use direct::*;

mod inline {
    #[path = "renamed.rs"]
    mod nested;

    pub use nested::*;
}
pub use inline::*;

#[path = "relocated"]
mod relocated {
    #[path = "leaf.rs"]
    mod leaf;

    pub use leaf::*;
}
pub use relocated::*;
