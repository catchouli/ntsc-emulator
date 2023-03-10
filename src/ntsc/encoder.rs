use std::{error::Error, io::Cursor};
use crate::types::{PixelSample, YiqSample, PI, SignalFloat, SrgbSample, RgbSample};
use crate::ntsc::*;

/// The NTSC encoder, allows you to sample the NTSC signal at a given time, generated from an
/// internal pixel buffer.
pub struct NtscEncoder {
    width: u32,
    height: u32,
    pixel_buffer: Vec<u8>,
}

impl NtscEncoder {
    /// Create a new NTSC encoder with a pixel buffer initialized to all 0s.
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_buffer = vec![0; (width * height * 4) as usize];

        Self {
            width,
            height,
            pixel_buffer,
        }
    }

    /// Initialize the NTSC decoder so that it contains an image file from a buffer (e.g. obtained
    /// from include_dir!).
    pub fn from_image_buf(buf: &[u8]) -> Result<Self, Box<dyn Error>> {
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
    pub fn sample(&self, time: SignalFloat) -> SignalFloat {
        // Convert time back to a pixel coordinate. We round back to the last pixel before the
        // given time, as if the signal changes instantly whenever there's a new pixel.
        // TODO: a bit wrong semantically, I think the number of scanlines supported by the NTSC
        // decoder shouldn't depend on the output image size, but the other way around.
        let time = time % NTSC_IMAGE_PERIOD;
        let y = (time / NTSC_SCANLINE_PERIOD) as u32 as SignalFloat / NTSC_SCANLINE_COUNT as SignalFloat;
        let x = (time % NTSC_SCANLINE_PERIOD) / NTSC_SCANLINE_PERIOD;

        // Sample pixel buffer.
        let pixel_sample = self.sample_pixel(x, y);

        // Output pixel luma.
        let (y, i, q) = Self::srgb_to_yiq(pixel_sample);

        y + i * SignalFloat::sin(time * 2.0 * PI * NTSC_COLOR_CARRIER_FREQ)
          + q * SignalFloat::cos(time * 2.0 * PI * NTSC_COLOR_CARRIER_FREQ)
    }

    /// Sample a pixel at the given pixel index.
    fn sample_index(&self, idx: u32) -> PixelSample {
        let idx = ((idx * 4) as usize) % self.pixel_buffer.len();
        let pixels = &self.pixel_buffer[idx..idx+4];
        (pixels[0], pixels[1], pixels[2], pixels[3])
    }

    /// Sample a pixel by coordinate.
    pub fn sample_pixel(&self, x: SignalFloat, y: SignalFloat) -> SrgbSample {
        // Convert to image coordinates and clamp.
        let x = u32::clamp((x * self.width as SignalFloat) as u32, 0, self.width);
        let y = u32::clamp((y * self.height as SignalFloat) as u32, 0, self.height);

        let idx = y * self.width + x;
        let sample_u8 = self.sample_index(idx);

        Self::rgba8_to_rgbf(sample_u8)
    }

    /// Convert from rgb to yiq.
    fn srgb_to_yiq((r, g, b): SrgbSample) -> YiqSample {
        // Calculate luma.
        // https://en.wikipedia.org/wiki/YIQ
        let y = 0.3 * r + 0.59 * g + 0.11 * b;
        let i = -0.27 * (b - y) + 0.74 * (r - y);
        let q = 0.41 * (b - y) + 0.48 * (r - y);

        (y, i, q)
    }

    /// Convert from rgb8 pixel data to rgb float data.
    fn rgba8_to_rgbf((r, g, b, _): PixelSample) -> RgbSample {
        (r as SignalFloat / 255.0, g as SignalFloat / 255.0, b as SignalFloat / 255.0)
    }
}
