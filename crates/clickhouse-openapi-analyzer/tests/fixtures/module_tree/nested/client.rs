pub struct Client;

mod operations {
    mod widgets;

    pub use widgets::*;
}

pub use operations::*;
