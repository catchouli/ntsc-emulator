use std::{io::Cursor, error::Error, f32::consts::PI};

use pixels::{SurfaceTexture, Pixels};
use winit::{event_loop::EventLoop, window::WindowBuilder, dpi::PhysicalSize, event::Event};

const OUTPUT_WIDTH: u32 = 320;
const OUTPUT_HEIGHT: u32 = 240;

const IMAGE_DATA: &[u8] = include_bytes!("../yamato.png");
//const IMAGE_DATA: &[u8] = include_bytes!("../colour_test_card.png");

// NTSC signal constants

/// The frequency of the color carrier wave in hz.
const NTSC_COLOR_CARRIER_FREQ: f32 = 3.579545e7;

/// The period of the color carrier sine wave.
const NTSC_COLOR_CARRIER_PERIOD: f32 = 2.0 * PI / NTSC_COLOR_CARRIER_FREQ;

/// The length of time for each scanline in seconds.
/// http://www.hpcc.ecs.soton.ac.uk/dan/pic/video_PIC.htm
const SCANLINE_TIME: f32 = 64e-6;

/// The length of time for the entire output image.
const OUTPUT_IMAGE_TIME: f32 = SCANLINE_TIME * OUTPUT_HEIGHT as f32;

// Decoder constants.

/// The number of samples to use per period to get accurate results. We need to average a sine wave
/// over its period and obtain a value as close to 0 as possible to minimize error when decoding
/// the signal luma.
const DECODER_SAMPLES_PER_PERIOD: usize = 10;

// TODO: note the sine wave might not align with the start of a scanline.
const DECODER_TIME_PER_SAMPLE: f32 = NTSC_COLOR_CARRIER_PERIOD / DECODER_SAMPLES_PER_PERIOD as f32;

/// The type for a pixel sample.
type PixelSample = (u8, u8, u8, u8);

/// The type for a YIQ (NTSC color space) sample.
type YiqSample = (f32, f32, f32);

/// The NTSC encoder, allows you to sample the NTSC signal at a given time, generated from an
/// internal pixel buffer.
struct NtscEncoder {
    width: u32,
    height: u32,
    pixel_buffer: Vec<u8>,
}

impl NtscEncoder {
    /// Create a new NTSC encoder with a pixel buffer initialized to all 0s.
    fn new(width: u32, height: u32) -> Self {
        let pixel_buffer = vec![0; (width * height * 4) as usize];

        Self {
            width,
            height,
            pixel_buffer,
        }
    }

    /// Initialize the NTSC decoder so that it contains an image file from a buffer (e.g. obtained
    /// from include_dir!).
    fn from_image_buf(buf: &[u8]) -> Result<Self, Box<dyn Error>> {
        // Load image and convert to rgba8 pixel buffer.
        let img = image::io::Reader::new(Cursor::new(buf))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        let mut encoder = NtscEncoder::new(img.width(), img.height());
        encoder.pixel_buffer = img.into_raw();

        Ok(encoder)
    }

    /// Sample the NTSC signal at a given time.
    fn sample(&self, time: f32) -> f32 {
        // Convert time back to a pixel coordinate. We round back to the last pixel before the
        // given time, as if the signal changes instantly whenever there's a new pixel.
        // TODO: a bit wrong semantically, I think the number of scanlines supported by the NTSC
        // decoder shouldn't depend on the output image size, but the other way around.
        let time = time % OUTPUT_IMAGE_TIME;
        let y = (time / SCANLINE_TIME) as usize as f32 / OUTPUT_HEIGHT as f32;
        let x = (time % SCANLINE_TIME) / SCANLINE_TIME;

        // Sample pixel buffer.
        let pixel_sample = self.sample_pixel(x, y);

        // Output pixel luma.
        let (y, i, q) = Self::rgb_to_yiq(pixel_sample);

        y + i * f32::sin(time * NTSC_COLOR_CARRIER_FREQ) + q * f32::cos(time * NTSC_COLOR_CARRIER_FREQ)
    }

    /// Sample a pixel at the given pixel index.
    fn sample_index(&self, idx: usize) -> PixelSample {
        let idx = idx * 4;
        let pixels = &self.pixel_buffer[idx..idx+4];
        //println!("buffer idx: {idx}");
        (pixels[0], pixels[1], pixels[2], pixels[3])
    }

    /// Sample a pixel by coordinate.
    fn sample_pixel(&self, x: f32, y: f32) -> PixelSample {
        // Convert to image coordinates and clamp.
        let x = u32::clamp((x * self.width as f32) as u32, 0, self.width);
        let y = u32::clamp((y * self.height as f32) as u32, 0, self.height);

        let idx = (y * self.width + x) as usize;
        self.sample_index(idx)
    }

    /// Convert from rgb to yiq.
    fn rgb_to_yiq((r, g, b, _): PixelSample) -> YiqSample {
        // Convert colors to float.
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;

        // Calculate luma.
        // https://en.wikipedia.org/wiki/YIQ
        let y = 0.3 * r + 0.59 * g + 0.11 * b;
        let i = -0.27 * (b - y) + 0.74 * (r - y);
        let q = 0.41 * (b - y) + 0.48 * (r - y);

        (y, i, q)
    }
}

/// The NTSC decoder, converts NTSC signals back to pixel data.
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
        let size = PhysicalSize::new(OUTPUT_WIDTH * 2, OUTPUT_HEIGHT * 2);

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
        const TIMING_NOISE: f32 = SCANLINE_TIME * 0.0;
        const SIGNAL_NOISE: f32 = 1.0;

        //if let Event::RedrawRequested(_) = event {
            println!("Redraw requested");
            let buf = pixels.get_frame_mut();
            let mut time_offset = 0.0;
            for (idx, pixel) in buf.chunks_exact_mut(4).enumerate() {
                if idx % OUTPUT_WIDTH as usize == 0 {
                    time_offset = rand::random::<f32>() * TIMING_NOISE;
                }

                // Calculate pixel time (start) in signal.
                let idx_nrm = idx as f32 / (OUTPUT_WIDTH * OUTPUT_HEIGHT) as f32;
                let pixel_time = idx_nrm * OUTPUT_IMAGE_TIME;

                // Calculate luma by averaging samples across the pixel's timespan in the signal.
                let mut luma = 0.0;
                let mut i = 0.0;
                let mut q = 0.0;
                let mut sample_time = pixel_time;
                let noise = rand::random::<f32>();
                for _ in 0..DECODER_SAMPLES_PER_PERIOD {
                    let sample = encoder.sample(sample_time + time_offset) * (1.0 - SIGNAL_NOISE)
                        + noise * SIGNAL_NOISE;
                    luma += sample;
                    i += sample * f32::sin(sample_time * NTSC_COLOR_CARRIER_FREQ);
                    q += sample * f32::cos(sample_time * NTSC_COLOR_CARRIER_FREQ);
                    sample_time += DECODER_TIME_PER_SAMPLE;
                }
                luma = luma / DECODER_SAMPLES_PER_PERIOD as f32;
                i = i / DECODER_SAMPLES_PER_PERIOD as f32 * 4.0;
                q = q / DECODER_SAMPLES_PER_PERIOD as f32 * 4.0;

                // yiq back to rgb
                let r = luma + 0.9469 * i + 0.6236 * q;
                let g = luma - 0.2748 * i - 0.6357 * q;
                let b = luma - 1.1 * i + 1.7 * q;

                pixel[0] = (r * 255.0) as u8;
                pixel[1] = (g * 255.0) as u8;
                pixel[2] = (b * 255.0) as u8;
                pixel[3] = 0xFF;
            }
            pixels.render().unwrap();
        //}
    });
}
