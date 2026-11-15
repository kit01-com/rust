#![cfg_attr(docsrs, feature(doc_cfg))]

mod conn;
mod error;
mod get;
mod http;
mod resume;
mod sched;
mod target;
mod verify;

pub use error::{Error, Result};
pub use get::{CONC, Get, TIMEOUT, get};
pub use resume::{PART_EXT, part_path};
pub use sched::{MAX_CHUNK, MIN_CHUNK, plan_chunk_size};
pub use verify::Verify;
