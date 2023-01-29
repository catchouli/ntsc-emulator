mod ntsc;
mod types;

use std::error::Error;
use pixels::{SurfaceTexture, PixelsBuilder};
use rand::{SeedableRng, Rng};
use rand::rngs::StdRng;
use winit::event::Event;
use winit::{event_loop::EventLoop, window::WindowBuilder, dpi::PhysicalSize};
use crate::ntsc::*;
use crate::types::SignalFloat;

/// The output image width.
const OUTPUT_WIDTH: u32 = 640;

/// The output image height.
const OUTPUT_HEIGHT: u32 = NTSC_SCANLINE_COUNT;

/// The test image data.
const IMAGE_DATA: &[u8] = include_bytes!("../yamato.png");

/// The amount of timing jitter to add to each scanline, in order to add a little analog 'jiggle'.
const TIMING_JITTER: SignalFloat = NTSC_SCANLINE_PERIOD * 0.000;

/// The amount of noise to add to the encoded signal before decoding it. The signal is attenuated
/// by the noise, and 1.0 noise leaves none of the original signal and just colorful snow.
const SIGNAL_NOISE: SignalFloat = 0.00;

/// The length of time for the entire output image.
const OUTPUT_IMAGE_TIME: SignalFloat = NTSC_SCANLINE_PERIOD * OUTPUT_HEIGHT as SignalFloat;

/// The number of samples to use per period to get accurate results. We need to average a sine wave
/// over its period and obtain a value as close to 0 as possible to minimize error when decoding
/// the signal luma. In practice I found that 3 samples was enough to produce an aesthetic result
/// and avoid visible error, which is an interesting result, but may be down to the fact that we
/// have pretty precise/deterministic timing compared to a real TV.
const SAMPLES_PER_PERIOD: usize = 5;

// TODO: note the sine wave might not align with the start of a scanline.
const TIME_PER_SAMPLE: SignalFloat = NTSC_COLOR_CARRIER_PERIOD / SAMPLES_PER_PERIOD as SignalFloat;

/// Generate timing jitter.
fn generate_timing_jitter(rng: &mut impl Rng) -> SignalFloat {
    if TIMING_JITTER > 0.0 {
        rng.gen_range(0.0..TIMING_JITTER)
    }
    else {
        0.0
    }
}

/// Generate signal noise.
fn generate_signal_noise(rng: &mut impl Rng) -> SignalFloat {
    if SIGNAL_NOISE > 0.0 {
        rng.gen_range(0.0..SIGNAL_NOISE)
    }
    else {
        0.0
    }
}

/// The main test program for the NTSC encoder/decoder - creates an NtscEncoder with the image from
/// `IMAGE_DATA` loaded in, and then encodes a signal using it, adds noise and timing jitter, and
/// finally decodes the signal using the NtscDecoder before displaying it on the screen.
///
/// Limitations:
/// * Doesn't support vsync or hsync - the timespan of each pixel is simply calculated.
fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging.
    env_logger::init();

    // Create event loop.
    let event_loop = EventLoop::new();

    // Create window.
    let window = {
        let size = PhysicalSize::new(1024.0, 768.0);

        WindowBuilder::new()
            .with_title("NTSC test")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)?
    };
    window.set_resizable(false);

    // Create pixel buffer.
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(OUTPUT_WIDTH, OUTPUT_HEIGHT, surface_texture)
            .build()?
    };

    // Create NTSC encoder and decoder and load image.
    let encoder = NtscEncoder::from_image_buf(IMAGE_DATA)?;
    let mut decoder = NtscDecoder::new(SAMPLES_PER_PERIOD);

    // Create random number generator.
    let mut rng = StdRng::from_entropy();

    // Main loop.
    event_loop.run(move |event, _, _| {
        match event {
            // Fill the buffer with pixel data whenever a redraw is requested.
            Event::RedrawRequested(_) => {
                let buf = pixels.get_frame_mut();
                let mut time_offset = 0.0;
                for (idx, pixel) in buf.chunks_exact_mut(4).enumerate() {
                    if (idx as u32) % OUTPUT_WIDTH == 0 {
                        time_offset = generate_timing_jitter(&mut rng);
                    }

                    // Calculate pixel time (start) in signal.
                    let idx_nrm = idx as SignalFloat / (OUTPUT_WIDTH * OUTPUT_HEIGHT+10) as SignalFloat;
                    let pixel_time = idx_nrm * OUTPUT_IMAGE_TIME;

                    // Generate `SAMPLES_PER_PERIOD` samples across the color carrier within the
                    // pixel, and push them to the NtscDecoder.
                    let mut sample_time = pixel_time + time_offset;
                    for _ in 0..SAMPLES_PER_PERIOD {
                        let noise = generate_signal_noise(&mut rng);
                        let sample = encoder.sample(sample_time) * (1.0 - SIGNAL_NOISE) + noise;
                        decoder.push_sample(sample_time, sample);
                        sample_time += TIME_PER_SAMPLE;
                    }

                    // Decode new samples.
                    let (r, g, b) = decoder.decode(true);

                    pixel[0] = SignalFloat::clamp(r * 255.0, 0.0, 255.0) as u8;
                    pixel[1] = SignalFloat::clamp(g * 255.0, 0.0, 255.0) as u8;
                    pixel[2] = SignalFloat::clamp(b * 255.0, 0.0, 255.0) as u8;
                    pixel[3] = 0xFF;
                }
                pixels.render().expect("Failed to render pixel buffer to screen");
            },
            // When we're out of events to process, request a redraw.
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
