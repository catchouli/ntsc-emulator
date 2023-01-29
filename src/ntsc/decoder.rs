use std::collections::VecDeque;
use crate::types::{SampleTime, SampleValue, SignalFloat, YiqSample, SrgbSample, RgbSample};
use crate::ntsc::generate_color_carrier;

/// The NTSC decoder, takes signal samples and converts them back to color information.
pub struct NtscDecoder {
    /// How many samples to integrate in order to retrieve the luma and color information.
    sample_count: usize,

    /// The sample queue, we store new samples here, and then integrate them to retrieve the color
    /// and luma information from the signal.
    sample_queue: VecDeque<(SampleTime, SampleValue)>,
}

impl NtscDecoder {
    /// Create a new NTSC Decoder which will integrate the given number of samples.
    pub fn new(sample_count: usize) -> Self {
        Self {
            sample_count,
            sample_queue: VecDeque::new(),
        }
    }

    /// Push a new sample into the decoder.
    pub fn push_sample(&mut self, time: SampleTime, value: SampleValue) {
        self.sample_queue.push_back((time, value));
        while self.sample_queue.len() > self.sample_count {
            self.sample_queue.pop_front();
        }
    }

    /// Decode the signal using the last `sample_count` samples.
    pub fn decode(&self, srgb: bool) -> RgbSample {
        if self.sample_queue.len() == self.sample_count {
            // Iterate our sample queue and:
            // * Average out the samples to obtain the luma (Y).
            // * Multiply each sample by the carrier wave (both in and out of phase), average, and
            //   multiply by four to obtain the chroma (I/Q).
            // https://codeandlife.com/2012/10/09/composite-video-decoding-theory-and-practice/
            let mut y = 0.0;
            let mut i = 0.0;
            let mut q = 0.0;

            for &(time, value) in &self.sample_queue {
                y += value;
                i += value * generate_color_carrier(time, true);
                q += value * generate_color_carrier(time, false);
            }

            let sample_count = self.sample_count as SignalFloat;
            let y = y / sample_count;
            let i = i / sample_count * 4.0;
            let q = q / sample_count * 4.0;

            let color = Self::yiq_to_rgb((y, i, q));

            if srgb {
                Self::srgb_to_rgb(color)
            }
            else {
                color
            }
        }
        else {
            // Just return black to be safe.
            (0.0, 0.0, 0.0)
        }
    }

    /// Convert from yiq to rgb. The output is in the sRGB color space.
    fn yiq_to_rgb((y, i, q): YiqSample) -> SrgbSample {
        // https://en.wikipedia.org/wiki/YIQ
        let r = y + 0.9469 * i + 0.6236 * q;
        let g = y - 0.2748 * i - 0.6357 * q;
        let b = y - 1.1 * i + 1.7 * q;

        (r, g, b)
    }

    /// Convert from sRGB to RGB.
    fn srgb_to_rgb((r, g, b): SrgbSample) -> RgbSample {
        (r.powf(2.2), g.powf(2.2), b.powf(2.2))
    }
}
