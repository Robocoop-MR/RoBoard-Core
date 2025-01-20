#![allow(dead_code, unused_imports)]
pub mod geometry {
    include!("flatbuffers/generated/geometry_generated.rs");
}

include!("flatbuffers/generated/messages_generated.rs");

pub use geometry::*;
