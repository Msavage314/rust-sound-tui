//! Deals with the fourier transform of a signal
//!
//! # Information on fourier transform (start here)
//! We want to convert the signal, a stream of amplitude values, into a stream of
//! different frequency values, which when added together produce a wave matching
//! what we had originally
//!
//! `fourier_transform` does this slowly, taking O(n^2) time to do so.
//! More useful, is the FFT, or fast fourier transform which does it in O(nlog(n)
//!
//! a *bin* is a single frequency output. e.g. there is a bin showing how closly 1Hz matches the signal, and another for 2Hz etc
//!
//!
//! The microphone gives us a set of amplitudes
//!
//! e.g. $$ x = \left[x_0,x_1,x_2,...x_{N-1}\right] $$
//!
//! Each is the sound amplitude at a particular moment in time
//! For example, if the signal contains a 440Hz tone, we want to see whether
//! that was present in the original sound, only using the samples
//!
//! One way we can do this is generate a new hypothetical 440Hz wave, and see how
//! closely it matches the amplitudes. We can repeat this for 441Hz, 441Hz, etc
//!
//! For each frequency we therefore calculate
//!
//! $$ \[C(f)=\sum_{n=0}^{N-1}x_n \begin{pmatrix} \cos(2\pi fn/f_s)\\ \sin(2\pi fn/f_s)  \end{pmatrix} \] $$
//!
//! which tels how closely the signal correlates with the sine and cosine components at that frequency
//! This is equivalent to wrapping the frequency around polar coordinates and finding the centre
//! <div class="warning"> check out https://www.desmos.com/calculator/rbvzt8sckq </div>
//!
//! FFT calculates the same frequency information but cleverly reuses intermediate results instead of starting from scratch every time
//!
//!
//! the DFT produces N complex numbers:
//! $$
//! \[X[k]=\sum_{n=0}^{N-1}x[n]e^{-i2\pi kn/N}\]
//! $$
//! Lets explain that equation
//! - X[n] - one of our audio samples
//! - N the number of samples in this FFT block
//! - X[k] frequency bin at index k
//! - \(e^{-i2\pi kn/N}\) A rotating vector used to test how much frequency is present. It is analogous to the sin and cos term above
//!
//! The frequency associated with bin k is
//! $$
//! f_k = \frac{k f_s}{N}
//! $$
//! where f_s is the sample rate.
//! For example, with 48000 samples per second, and 1024 samples per FFT (we will tak about that in a second):
//! Bin 1 corresponds to 46.875Hz, bin 2 to 93.75Hz etc.
//! Each bin measures how strongly the signal correlates with a particular complex sinusoid, storing two important pieces of information
//! - Magnitude - how strongly it is represented
//! - Phase - how it is aligned
//!
//! ### FFT
//! Calculating FFT for 8 samples requires 8 output bins, because we can prove (using "we" figuratively) that there is only one wave
//! that will fit... It is just important to know you only need to test 8 frequencies and the rest can be found from there
//! And each bin requires us to project those points onto the circle, like in the desmos, then add them up, leading to O(n^2) operations.
//!
//! But we can be clever, and split the DFT into two groups, the even frequencies adn the odd frequencies.
//! e.g.
//! x0,x2,x4,x6
//! x1,x3,x5,x7
//! Then we calculate a DFT for each group, giving us two sets of complex results
//! E[k]: the even samples transform
//! O[k]: the odd samples transform
//!
//! These are build using the same process, dividing by two (we will get to the base case later)
//!
//! We can then derive the 8 point DFT from those two four point DFTs
//!
//! for even samples, we replace the index with 2m, and so the exponent becomes
//! $$
//! e^{-2\pi i k m / (N/2)}
//! $$
//! The for odd samples we do 2m+1
//! $$
//! \[e^{-i2\pi k(2m+1)/N}=e^{-i2\pi km/(N/2)}\cdot e^{-i2\pi k/N}\]
//! $$
//!
//! The second factor for odd samples goes outside the sum, so we can factor it out to give
//! $$
//! \boxed{X[k]=E[k]+e^{-i2\pi k/N}O[k]}
//! $$
//! This is called a twiddle factor, and multiplying our function by that factor results in it being rotated different amounts
//! So we rotate the odd factor by an amount and call the output T
//! Then
//! $$ \[\boxed{X[k]=E[k]+T}\]$$
//! $$ \[\boxed{X[k+N/2]=E[k]-T}\]$$
//! These are the two outputs. This operation is called a butterfly, between the even and odd groups. It is how we combine
//! the sets.
//! The twiddle factor is like reshuffling the phase of the odd sample, so that they line up properly
//! The output gives the index of the lower frequency bin X[k] and the higher index output \(X[k + N/2]\)
//!
//!
//! TODO
//! - Clean up above
//! - Optimise more
use crate::{Signal, maths::utils::Complex};
use core::f64::consts::TAU;
use rayon::prelude::*;
use std::vec;

/// A *bin* storing frequency data about a waveform sinusoid

#[derive(Debug, Clone, Copy)]
pub struct Bin {
    pub frequency: f64,
    pub phase: f64,
    pub amplitude: f64,
}

/// Computes the Fast Fourier Transform (FFT) from a set of complex numbers, and returns a set
/// of complex numbers giving sinusoids that can sum to make the input
/// The length of the data must be a power of two
/// https://www.desmos.com/calculator/rbvzt8sckq
fn fft(data: &[Complex]) -> Vec<Complex> {
    let n = data.len();

    // Base case. a one element DFT is already complete.
    if n <= 1 {
        return data.to_vec();
    }
    assert!(n.is_power_of_two(), "FFT length must be a power of two");

    // Here we split the data into even and odd indexed samples
    let even: Vec<Complex> = data.iter().step_by(2).copied().collect();
    let odd: Vec<Complex> = data.iter().skip(1).step_by(2).copied().collect();

    // Next we recursively transform each half of the fft
    let even_fft = fft(&even);
    let odd_fft = fft(&odd);
    let mut result = vec![Complex::new(0.0, 0.0); n];

    // combine the two smaller FFTs using butterflies
    for k in 0..n / 2 {
        let angle = -TAU * k as f64 / n as f64;
        let twiddle = Complex::new(angle.cos(), angle.sin());
        let rotated_odd = twiddle * odd_fft[k];

        result[k] = even_fft[k] + rotated_odd;
        result[k + n / 2] = even_fft[k] - rotated_odd;
    }
    return result;
}
/// See module doc comment and also https://www.desmos.com/calculator/rbvzt8sckq
pub fn fft_signal(signal: &Signal) -> Vec<Bin> {
    let sample_count = signal.samples.len();
    if sample_count == 0 {
        return Vec::new();
    }

    let n = sample_count.next_power_of_two();
    let mut data: Vec<Complex> = signal
        .samples
        .iter()
        .map(|&sample| Complex::new(sample as f64, 0.0))
        .collect();

    data.resize(n, Complex::zero());

    let spectrum = fft(&data);

    // For real-valued inputs, only the first half is unique; This is because a sin wave of frequency -3 is
    // the same as a sin wave of frequency 3, but not for complex numbers
    (0..=n / 2)
        .map(|k| {
            let value = spectrum[k];

            let frequency = k as f64 * signal.sample_rate as f64 / n as f64;

            let mut amplitude = value.abs() / n as f64;
            // Double non-DC, non-Nyquist bins to get
            // single-sided peak amplitudes.
            if k != 0 && !(n % 2 == 0 && k == n / 2) {
                amplitude *= 2.0;
            }

            let phase = value.phase();

            Bin {
                frequency,
                phase,
                amplitude,
            }
        })
        .collect()
}

/// Performs the fourier transform.
/// Do not use this function, apart from for testing / demonstration, as it is \(O(n^2))\ time complexity,
/// so for standard sample rates of 10,000+ Hz and a few second long clips this will take forever.
///
/// Instead use `fft_signal` below, which uses the fast-fourier-transform
///
/// Check out this desmos graph
/// https://www.desmos.com/calculator/rbvzt8sckq
///
#[deprecated(note = "Deprecated as it is extremely slow. Use `fft_signal` instead")]
pub fn fourier_transform(signal: &Signal) -> Vec<Bin> {
    // We want to test loads of different frequencies and see which ones line up with
    // the signal.
    const MAX_FREQUENCY: f64 = 20000.0;
    let duration = signal.duration();
    let step = 1.0 / duration;
    let tests = (MAX_FREQUENCY / step) as usize;

    (0..tests)
        .into_par_iter()
        .map(|test_index| {
            let test_frequency = test_index as f64 * step;
            let mut total = Complex::new(0.0, 0.0);
            for (index, sample) in signal.samples.iter().enumerate() {
                // T is how far we are around a imaginary circle.
                // It has to be from 0 to 1
                let t = index as f64 / signal.sample_rate as f64;
                // Then we want an entire revolution of t going from 0 to 1
                // to go fully around the circle, so this is the angle
                let angle = TAU * t * test_frequency as f64;
                // Find the point and add it to the total.
                let p = Complex::new(angle.cos(), angle.sin());
                total += p * *sample;
            }
            // Divide by total to find centre of mass (approximation of integration)
            let sample_centre = total / signal.samples.len() as f64;
            // distance to centre *2 as
            // O -- c -- a
            // if c is the centre, a is twice as far from O

            let amplitude = sample_centre.abs() * 2.0;
            let phase = -sample_centre.phase();

            Bin {
                frequency: test_frequency as f64,
                phase,
                amplitude,
            }
        })
        .collect()
}
