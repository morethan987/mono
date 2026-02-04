pub mod niri;
mod traits;
mod unix;

pub use traits::Platform;
pub use unix::UnixPlatform;
