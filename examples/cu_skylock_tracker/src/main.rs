//! SKYLOCK's drone tracking pipeline on Copper, ported from `examples/cu_human_pose`.
//!
//! An RTSP camera feeds GStreamer, `gst_to_image` turns each frame into a `CuImage`, the
//! detector (YOLOv8 through Candle) finds the targets in it, the tracker follows one of them
//! with a Kalman filter, and the Rerun view shows the frame, the detections and the track.
//!
//! The camera URL comes only from `SKYLOCK_RTSP_URL`, at run time (see `rtsp`). Copper
//! records the effective config, URL included, in every log under `logs/`, so treat those
//! files as secret.
//!
//! Run from this directory, because the log path and `copperconfig.ron` are relative:
//!
//! ```text
//! cargo run -r                    # CPU inference
//! cargo run -r --features cuda    # CUDA inference
//! ```

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use cu_skylock_tracker::rtsp;
// The RON config names its tasks and messages through these modules.
use cu_skylock_tracker::{payloads, tasks};
use cu29::prelude::*;

const CONFIG_PATH: &str = "copperconfig.ron";
const LOG_PATH: &str = "logs/skylock.copper";
/// NTFS allocates each slab file in full, so the slabs stay small (BREAKLOG.md).
const SLAB_SIZE: Option<usize> = Some(256 * 1024 * 1024);

#[copper_runtime(config = "copperconfig.ron")]
struct SkylockTrackerApplication {}

fn main() -> ExitCode {
    let url = match rtsp::url_from_env() {
        Ok(url) => url,
        Err(message) => return fail(&message),
    };
    // Any message from here on may quote the pipeline, so it goes through `redact`.
    let fail_redacted = |message: String| fail(&rtsp::redact(&message, &url));

    let mut config = match read_configuration(CONFIG_PATH) {
        Ok(config) => config,
        Err(e) => {
            return fail_redacted(format!(
                "cannot read {CONFIG_PATH}; run from examples/cu_skylock_tracker: {e}"
            ));
        }
    };
    if let Err(e) = rtsp::insert_url(&mut config, "camera", &url) {
        return fail_redacted(e.to_string());
    }
    if let Some(parent) = Path::new(LOG_PATH).parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return fail(&format!("cannot create {}: {e}", parent.display()));
    }

    let application = match SkylockTrackerApplication::builder()
        .with_log_path(LOG_PATH, SLAB_SIZE)
        .and_then(|builder| builder.with_config(config).build())
    {
        Ok(application) => application,
        Err(e) => return fail_redacted(format!("cannot start: {e}")),
    };
    if let Err(e) = application.run_until_shutdown() {
        let message = rtsp::redact(&e.error.to_string(), &url);
        error!("Error during iteration: {}", message.clone());
        return fail(&message);
    }
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    eprintln!("cu-skylock-tracker: {message}");
    ExitCode::FAILURE
}
