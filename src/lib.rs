pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// Event implementations
pub mod events;
/// Lock implementations
pub mod locks;

#[derive(Copy, Clone, Debug)]
pub enum Timeout {
    Infinite,
    Val(std::time::Duration),
}
