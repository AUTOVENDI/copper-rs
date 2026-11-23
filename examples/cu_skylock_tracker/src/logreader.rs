//! Log reader for `cu-skylock-tracker`: Copper's export CLI, built for this app's messages.
//!
//! Its output can include the camera URL, because the camera source logs its pipeline and
//! the log stores the effective config. Redact it before it leaves this machine.

use cu29::prelude::*;
use cu29_export::run_cli;
// The RON config names its messages through this module.
use cu_skylock_tracker::payloads;

gen_cumsgs!("copperconfig.ron");

fn main() {
    run_cli::<CuMsgs>().expect("Failed to run the export CLI");
}
