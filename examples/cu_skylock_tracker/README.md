# cu-skylock-tracker: SKYLOCK's camera-to-track pipeline on Copper

The tracking half of SKYLOCK, WhiteHat Design Studio's drone detection and tracking ground
station, ported to Copper from `examples/cu_human_pose`. An RTSP camera feeds GStreamer, a
YOLOv8n detector in Candle finds the targets in each frame, a single-target Kalman tracker
follows one of them, and a Rerun viewer shows the frame, the detections and the track.

```text
camera ──► gst_to_image ──► detector ──► tracker ──► viz
(RTSP)     (CuImage)        (YOLOv8n)    (Kalman)    (Rerun; also takes the image
                                                      and the detections)
```

This is a bench prototype. The detector is stock YOLOv8n with the COCO classes, tested on a
person, and SKYLOCK's own model, the gimbal, replay and reconnect aren't done; see
[What isn't done](#what-isnt-done).

## Run it

You need Rust 1.98 or newer, GStreamer with its development files where `pkg-config` finds
them (tested with 1.28.9), and a Rerun viewer 0.43.x (`rerun`) on `PATH`. The `cuda` feature
needs an NVIDIA GPU and a CUDA toolkit no newer than 13.4 (tested with 12.9). On Windows,
`WINDOWS_SETUP.md` installs and checks the tools, and shows how to start a session with the
camera URL set.

Run from this directory, because the config and the log paths are relative:

```bash
export SKYLOCK_RTSP_URL='rtsp://...'   # the camera, required
cargo run -r --features cuda           # or `cargo run -r` for the CPU
```

The first run downloads `yolov8n.safetensors` from Hugging Face (`lmz/candle-yolo-v8`) into
the local Hugging Face cache; the repository holds no weights. YOLOv8's weights are
Ultralytics', under AGPL-3.0 unless an Enterprise licence covers you.

| Variable | What it does |
|---|---|
| `SKYLOCK_RTSP_URL` | The camera's `rtsp://` URL. The app exits if it is missing or not `rtsp`, without printing it. |
| `RERUN=off` | Runs without the viewer. |
| `GST_DEBUG=2` | Prints GStreamer's warnings and errors, which the camera source doesn't report. |

Ctrl+C stops the app and closes its log (on Windows, Ctrl+Break does too) once it has
started. In its first seconds, while the tasks are being built, Ctrl+C ends it with the log
left open, and `fsck` then refuses that log.

The pipeline in `copperconfig.ron` expects H.265 (`rtph265depay ! h265parse ! avdec_h265`);
for an H.264 camera, use the `h264` elements. The app reads the config when it starts, so a
change to a task's settings needs no rebuild; a change to the graph does.

### The log holds the camera URL

Copper writes the effective config into every log, and the camera source logs its pipeline,
so `logs/skylock.copper` holds the URL twice. Treat logs as secret. They are also large:
12 to 14 GB per 10 minutes of camera, because each frame is logged twice, in 256 MiB slab
files that NTFS allocates in full. Each run replaces the last run's log.

To read one, with the log reader that `cargo build -r` builds:

```bash
../../target/release/cu-skylock-tracker-logreader logs/skylock.copper \
    extract-text-log ../../target/release/build/cu29_log_index
../../target/release/cu-skylock-tracker-logreader logs/skylock.copper fsck
```

Cargo 1.100 puts the string index in `target/<profile>/build/`; older Cargo puts it in
`target/<profile>/`.

## Configuration

`copperconfig.ron` holds the graph and each task's settings:

- `camera` (`tasks::RtspCamera`, a `cu_gstreamer` source with a ring of 8 frames): the
  `pipeline`, with `RTSP_URL_PLACEHOLDER` where the app puts the URL and an `appsink` named
  `copper` at the end, and its `caps`.
- `gst_to_image`: the frame's `width`, `height` and `fourcc`, and delivery statistics in the
  text log every `stats_period_s`: frames, the longest gap, and each gap over `stall_ms`.
  `stats_skip_s` (10 here) leaves the first seconds, such as the camera's start-up burst, out
  of the run's summary.
- `detector` (`tasks::YoloDetect`): `variant` (`yolov8n` to `yolov8x`), `conf_threshold`,
  `iou_threshold`, and `classes`, the COCO class ids to keep (`[0]`, people, for the bench).
- `tracker` (`tasks::KalmanTracker`): `confirm_hits` (3), `gate_chi2` (9.21),
  `max_coast_ms` (1000), `lead_ms` (0), `meas_std_px` (5), `accel_psd` (20000 px²/s³) and
  `init_vel_std` (300 px/s). `src/tasks/kalman_tracker.rs` describes each. The values are
  estimates, not tuned.
- `viz` (`tasks::RerunTrackViz`): the viewer's `memory_limit` (`2GB`).

`runtime: (rate_target_hz: 120)` runs the loop at about four times the camera's rate.

The tracker runs a constant-velocity Kalman filter on the box centre, with state
`[x, y, vx, vy]` in px and px/s, and takes `dt` from the frames' `tov`. A candidate becomes
the target after `confirm_hits` detections in a row. The target then takes the most
confident detection inside its Mahalanobis gate, coasts on the prediction when the gate is
empty, and is lost after `max_coast_ms` without a detection. Its whole state freezes into
Copper's keyframes. The view draws the target in its status's colour (green tracking, yellow
coasting, red lost, lavender searching) with a velocity arrow for 0.5 s of travel and the
lead point.

## Measured

On a Windows 11 laptop with an Intel i7-14700HX, an NVIDIA RTX 5060 Laptop GPU (8 GB),
CUDA 12.9, GStreamer 1.28.9 and Rust 1.100, with the CUDA build. The camera is SKYLOCK's,
H.265 at 1920×1080 and 30 fps, decoded on the CPU and scaled to 640×360, unless the table
says otherwise. `BREAKLOG.md` has each measurement in full.

| What | Result |
|---|---|
| Detector, YOLOv8n on 640×352, CUDA, camera | p50 12.5 to 13.3 ms, p95 14.5 to 21.5 ms a frame, over four runs |
| Detector, CPU, synthetic 30 fps stream | p50 200 ms, p95 211 ms a frame; the stream falls to about 5 fps |
| Tracker | p50 1 µs, p99 3 µs a frame |
| Frames over 10 minutes, viewer on | 17,546 of about 17,588 (0.24 % dropped), longest gap 371 ms |
| Wait between decoder and loop, 60 s: `tov` minus the buffer PTS, above the run's minimum | p50 14 ms, p95 32 ms, p99 47 ms, max 100 ms |
| Loop stalls over 10 minutes, synthetic stream, no viewer | 37 gaps of 100 to 578 ms, at the log's section seals |
| Live tracking, a person walking, hiding and leaving | 37 detection dropouts of 26 to 500 ms bridged; 9 losses, each about 1 s after the last detection, all re-acquired |
| Log | 12 to 14 GB per 10 minutes |
| Viewer memory | 1.8 to 2.2 GB, with its 2 GB limit |
| Clock drift | Copper's clock about 65 ppm against GStreamer's; the camera's about 470 ppm |

The clock, frame-count, wait, stall and log rows come from earlier builds of this port that
still ran the pose model it started from: the clock, frame-count and wait rows from the first
build, before the 120 Hz rate target. The viewer and CPU-detector rows come from the first
build with the detector. The CUDA detector, tracker and live-tracking rows are this build.
There is no camera-to-track latency figure: frames are stamped when the source pops them, so
the time the camera took each one isn't known. There is no comparison with the Python
SKYLOCK yet.

## What isn't done

- **SKYLOCK's own model.** The detector is stock YOLOv8n with the COCO classes, which have no
  drone, so the bench target is a person.
- **The gimbal.** Nothing turns a track into pan and tilt.
- **Replay.** There is no resim binary, so the tracker's determinism under replay is untested,
  although it freezes its whole state into keyframes and a test checks that a thawed tracker
  goes on bit for bit.
- **Reconnect.** `cu_gstreamer` neither watches GStreamer's bus nor reconnects.

Known problems:

- A lost RTSP session never comes back, and nothing reports it: after a 10 s cable pull the
  app ran on without frames. In the live tracking test the stream also stopped twice
  mid-run, after 194 s and 25 s, with no error. The cause is unknown (a touched cable isn't
  ruled out for the first stop), and a third run with `GST_DEBUG=2` ran clean.
- While frames are logged, the loop stalls for 100 to 600 ms a few times a minute (37 times
  in 10 minutes), in bursts that follow the 1.6 s it takes to fill a log section: sealing a
  section flushes the whole slab, and waits for the disk, on the loop thread (seen on
  Windows). This is with Copper's default inline CopperList output, as the pose example
  uses; the `async-cl-io` feature, which moves the output to a worker thread, wasn't tried.
- A frame's `tov` is when the source pops it. The camera's start-up burst of several seconds
  of old frames is tracked as if it were live, with velocities that are too high, and frames
  that wait out a stall arrive bunched.
- Each frame is logged twice, by the camera source and by `gst_to_image`.
- The root justfile's recipes fail when the clone's path holds a space, and `just fmt`
  overflows the Windows command line.

`BREAKLOG.md` lists every problem the port met, each marked upstream or ours.
