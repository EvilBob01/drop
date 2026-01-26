#![deny(clippy::all)]
#![feature(impl_trait_in_bindings)]
#![feature(nonpoison_mutex)]
#![feature(sync_nonpoison)]
pub mod file_utils;
pub mod ssl;
pub mod versions;
pub mod manifest;

pub use manifest::{CHUNK_SIZE, MAX_FILE_COUNT};

#[cfg(test)]
pub mod tests;
