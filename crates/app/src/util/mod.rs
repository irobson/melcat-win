//! Small Win32 helpers shared by every module.

pub mod log;
pub mod paths;
pub mod wide;

pub use wide::{from_wide, wide};
