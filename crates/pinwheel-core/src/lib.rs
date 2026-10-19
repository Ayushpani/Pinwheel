//! Pinwheel engine core.
//!
//! The pure, codec-free logic shared by every shell and the command-line
//! tool: how big an output should be, how to hit a file-size target, what to
//! name the result, what a recipe or spec card means, and what a file really
//! is. Nothing in here touches the network, and nothing in here learns.

pub mod filetype;
pub mod naming;
pub mod recipe;
pub mod resize;
pub mod spec;
pub mod target;
pub mod units;

mod error;

pub use error::Error;
