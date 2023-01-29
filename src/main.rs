mod ntsc;
mod types;

use std::error::Error;
use pixels::{SurfaceTexture, Pixels};
use winit::event::Event;
use winit::{event_loop::EventLoop, window::WindowBuilder, dpi::PhysicalSize};
use crate::ntsc::*;
use crate::types::{SignalFloat, PI};

/// The output image width.
const OUTPUT_WIDTH: u32 = 640;

/// The output image height.
const OUTPUT_HEIGHT: u32 = NTSC_SCANLINE_COUNT;

/// The test image data.
const IMAGE_DATA: &[u8] = include_bytes!("../yamato.png");

/// The length of time for the entire output image.
const OUTPUT_IMAGE_TIME: SignalFloat = NTSC_SCANLINE_PERIOD * OUTPUT_HEIGHT as SignalFloat;

// Decoder constants.

/// The number of samples to use per period to get accurate results. We need to average a sine wave
/// over its period and obtain a value as close to 0 as possible to minimize error when decoding
/// the signal luma.
const DECODER_SAMPLES_PER_PERIOD: u32 = 5;

// TODO: note the sine wave might not align with the start of a scanline.
const DECODER_TIME_PER_SAMPLE: SignalFloat = NTSC_COLOR_CARRIER_PERIOD / DECODER_SAMPLES_PER_PERIOD as SignalFloat;

/// The NTSC decoder, takes signal samples and converts them back to color information.
struct NtscDecoder {
}

impl NtscDecoder {
}

/// Test program.
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
        Pixels::new(OUTPUT_WIDTH, OUTPUT_HEIGHT, surface_texture)?
    };

    // Create NTSC encoder and load image.
    let encoder = NtscEncoder::from_image_buf(IMAGE_DATA)?;

    event_loop.run(move |event, _, _| {
        const TIMING_NOISE: SignalFloat = NTSC_SCANLINE_PERIOD * 0.005;
        const SIGNAL_NOISE: SignalFloat = 0.05;

        // Round output height up to nearest scanline multiple.

        match event {
            Event::RedrawRequested(_) => {
                let buf = pixels.get_frame_mut();
                let mut time_offset = 0.0;
                for (idx, pixel) in buf.chunks_exact_mut(4).enumerate() {
                    if (idx as u32) % OUTPUT_WIDTH == 0 {
                        time_offset = rand::random::<SignalFloat>() * TIMING_NOISE;
                    }

                    // Calculate pixel time (start) in signal.
                    let idx_nrm = idx as SignalFloat / (OUTPUT_WIDTH * OUTPUT_HEIGHT+10) as SignalFloat;
                    let pixel_time = idx_nrm * OUTPUT_IMAGE_TIME;

                    // Calculate luma by averaging samples across the pixel's timespan in the signal.
                    let mut luma = 0.0;
                    let mut i = 0.0;
                    let mut q = 0.0;
                    let mut sample_time = pixel_time + time_offset;
                    let mut samples = 0.0;
                    for _ in 0..DECODER_SAMPLES_PER_PERIOD {
                        let noise = rand::random::<SignalFloat>();
                        let sample = encoder.sample(sample_time) * (1.0 - SIGNAL_NOISE)
                            + noise * SIGNAL_NOISE;
                        luma += sample;
                        i += sample * SignalFloat::sin(sample_time * 2.0 * PI * NTSC_COLOR_CARRIER_FREQ);
                        q += sample * SignalFloat::cos(sample_time * 2.0 * PI * NTSC_COLOR_CARRIER_FREQ);
                        sample_time += DECODER_TIME_PER_SAMPLE;
                        samples += 1.0;
                    }
                    luma = luma / samples;
                    i = i / samples * 4.0;
                    q = q / samples * 4.0;

                    // yiq back to rgb
                    let r = luma + 0.9469 * i + 0.6236 * q;
                    let g = luma - 0.2748 * i - 0.6357 * q;
                    let b = luma - 1.1 * i + 1.7 * q;

                    pixel[0] = SignalFloat::clamp(r * 256.0, 0.0, 255.9) as u8;
                    pixel[1] = SignalFloat::clamp(g * 256.0, 0.0, 255.9) as u8;
                    pixel[2] = SignalFloat::clamp(b * 256.0, 0.0, 255.9) as u8;
                    pixel[3] = 0xFF;
                }
                pixels.render().expect("Failed to render pixel buffer to screen");
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });
}
