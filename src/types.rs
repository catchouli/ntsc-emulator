/// The floating point type to use for signal calculations. This is here so it's easy to test whether
/// f64 produces the same results as f32, and having tested that, it seems like we're right on the
/// edge of 32-bit floating point imprecision due to the high frequency of the color carrier wave.
/// However, f32 seems to work perfectly fine for the given values, thankfully, so it should be
/// possible to port this to a fast shader easily.
pub type SignalFloat = f32;

/// Not really a type, but the PI constant to use with SignalFloat.
pub const PI: SignalFloat = std::f64::consts::PI as SignalFloat;

/// The type for an rgba pixel sample.
pub type PixelSample = (u8, u8, u8, u8);

/// The type for an RGB color sample, in floating point format.
pub type RgbSample = (SignalFloat, SignalFloat, SignalFloat);

/// The type for an sRGB color sample, in floating point format.
pub type SrgbSample = (SignalFloat, SignalFloat, SignalFloat);

/// The type for a YIQ (NTSC color space) sample.
pub type YiqSample = (SignalFloat, SignalFloat, SignalFloat);

/// The instant a sample from the signal was taken.
pub type SampleTime = SignalFloat;

/// The value of a sample from the signal.
pub type SampleValue = SignalFloat;
