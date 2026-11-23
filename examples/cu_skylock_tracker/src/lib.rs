//! SKYLOCK's drone tracking pipeline on Copper, as a library that the app (`src/main.rs`)
//! and its log reader (`src/logreader.rs`) share.

pub mod frame_stats;
pub mod payloads;
pub mod rtsp;
pub mod tasks;
pub mod timing;
pub mod tracker;
pub mod yolo;
