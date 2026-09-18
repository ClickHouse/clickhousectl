pub struct Client;

#[cfg(test)]
mod missing_tests;

#[cfg(any())]
mod missing_never;

#[cfg(not(test))]
mod operations {
    impl Client {
        pub async fn production_operation(&self) {}
    }
}
