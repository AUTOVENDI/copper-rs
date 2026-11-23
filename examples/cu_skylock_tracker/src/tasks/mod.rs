//! The app's tasks, as `copperconfig.ron` names them, and the camera source's type.

mod gst_to_image;
mod kalman_tracker;
mod track_viz;
mod yolo_detect;

pub use gst_to_image::GstToCuImage;
pub use kalman_tracker::KalmanTracker;
pub use track_viz::RerunTrackViz;
pub use yolo_detect::YoloDetect;

/// How many frames the camera source holds; when the ring is full, a new frame overwrites
/// the oldest. `cu_gstreamer::CuDefaultGStreamer` uses 8, and M1 measures 8 against 1.
pub const CAMERA_RING_DEPTH: usize = 8;

/// The camera source: GStreamer running the RTSP pipeline from `copperconfig.ron`.
pub type RtspCamera = cu_gstreamer::CuGStreamer<CAMERA_RING_DEPTH>;
