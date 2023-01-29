mod encoder;
mod decoder;

pub use encoder::*;
pub use decoder::*;

use crate::types::SignalFloat;

/// The frequency of the color carrier wave in hz.
pub const NTSC_COLOR_CARRIER_FREQ: SignalFloat = 3.579545e6;

/// The period of the color carrier sine wave.
pub const NTSC_COLOR_CARRIER_PERIOD: SignalFloat = 1.0 / NTSC_COLOR_CARRIER_FREQ;

/// The number of scanlines in an NTSC broadcast.
pub const NTSC_SCANLINE_COUNT: u32 = 525;

/// The length of time for each scanline in seconds. This includes hsync normally, but we don't
/// emulate it since we can calculate the correct time range for each pixel being drawn.
/// http://www.hpcc.ecs.soton.ac.uk/dan/pic/video_PIC.htm
pub const NTSC_SCANLINE_PERIOD: SignalFloat = 64e-6;

/// The length of time for a full image in an NTSC signal, which is the period for each scanline
/// times the number of scanlines.
pub const NTSC_IMAGE_PERIOD: SignalFloat = NTSC_SCANLINE_PERIOD * NTSC_SCANLINE_COUNT as SignalFloat;
