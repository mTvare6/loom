// SPDX-License-Identifier: MPL-2.0

use crate::fft::{Complex, FourierTransform};

const BLOCK_SIZE: usize = 512;
const FFT_SIZE: usize = BLOCK_SIZE * 2;

pub(crate) struct StereoFir {
    transform: FourierTransform,
    impulse_spectra: Box<[[Vec<Vec<Complex>>; 2]; 2]>,
    input_history: Box<[Vec<Vec<Complex>>; 2]>,
    input_block: [[f32; BLOCK_SIZE]; 2],
    output_block: [[f32; BLOCK_SIZE]; 2],
    output_spectra: [Vec<Complex>; 2],
    overlap: [[f32; BLOCK_SIZE]; 2],
    block_position: usize,
    history_position: usize,
    partitions: usize,
}

pub(crate) struct InterpolatedStereoFir {
    transform: FourierTransform,
    impulse_spectra: Vec<Box<[[Vec<Vec<Complex>>; 2]; 2]>>,
    input_history: Box<[Vec<Vec<Complex>>; 2]>,
    input_block: [[f32; BLOCK_SIZE]; 2],
    output_block: [[f32; BLOCK_SIZE]; 2],
    output_spectra: [Vec<Complex>; 2],
    overlap: [[f32; BLOCK_SIZE]; 2],
    block_position: usize,
    history_position: usize,
    partitions: usize,

    gain_knots: [[f32; 6]; 2],
    current_position: f32,
    target_position: f32,
}

impl StereoFir {
    pub(crate) fn new(impulses: [[Vec<f32>; 2]; 2]) -> Box<Self> {
        let impulse_length = impulses.iter().flatten().map(Vec::len).max().unwrap_or(1);
        let partitions = impulse_length.div_ceil(BLOCK_SIZE);
        let mut transform = FourierTransform::new(FFT_SIZE);
        let mut impulse_spectra = Box::new(std::array::from_fn(|output| {
            std::array::from_fn(|input| {
                (0..partitions)
                    .map(|partition| {
                        let mut spectrum = vec![Complex::default(); FFT_SIZE];
                        let start = partition * BLOCK_SIZE;
                        let end = (start + BLOCK_SIZE).min(impulses[output][input].len());
                        for (bin, sample) in spectrum
                            .iter_mut()
                            .zip(&impulses[output][input][start..end])
                        {
                            *bin = Complex::new(*sample, 0.0);
                        }
                        transform.forward(&mut spectrum);
                        spectrum
                    })
                    .collect::<Vec<_>>()
            })
        }));

        // Explicit allocation before process callback
        for paths in impulse_spectra.iter_mut() {
            for path in paths.iter_mut() {
                path.shrink_to_fit();
            }
        }

        Box::new(Self {
            transform,
            impulse_spectra,
            input_history: Box::new(std::array::from_fn(|_| {
                (0..partitions)
                    .map(|_| vec![Complex::default(); FFT_SIZE])
                    .collect::<Vec<_>>()
            })),
            input_block: [[0.0; BLOCK_SIZE]; 2],
            output_block: [[0.0; BLOCK_SIZE]; 2],
            output_spectra: std::array::from_fn(|_| vec![Complex::default(); FFT_SIZE]),
            overlap: [[0.0; BLOCK_SIZE]; 2],
            block_position: 0,
            history_position: 0,
            partitions,
        })
    }

    #[inline]
    pub(crate) fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let output = (
            self.output_block[0][self.block_position],
            self.output_block[1][self.block_position],
        );
        self.input_block[0][self.block_position] = left;
        self.input_block[1][self.block_position] = right;
        self.block_position += 1;
        if self.block_position == BLOCK_SIZE {
            self.process_block();
            self.block_position = 0;
        }
        output
    }

    pub(crate) fn reset(&mut self) {
        for channel in self.input_history.iter_mut() {
            for partition in channel {
                partition.fill(Complex::default());
            }
        }
        self.input_block.fill([0.0; BLOCK_SIZE]);
        self.output_block.fill([0.0; BLOCK_SIZE]);
        for spectrum in &mut self.output_spectra {
            spectrum.fill(Complex::default());
        }
        self.overlap.fill([0.0; BLOCK_SIZE]);
        self.block_position = 0;
        self.history_position = 0;
    }

    fn process_block(&mut self) {
        for input in 0..2 {
            let spectrum = &mut self.input_history[input][self.history_position];
            spectrum.fill(Complex::default());
            for (bin, sample) in spectrum.iter_mut().zip(self.input_block[input]) {
                *bin = Complex::new(sample, 0.0);
            }
            self.transform.forward(spectrum);
        }

        for output in 0..2 {
            let spectrum = &mut self.output_spectra[output];
            spectrum.fill(Complex::default());
            for input in 0..2 {
                for partition in 0..self.partitions {
                    let history =
                        (self.history_position + self.partitions - partition) % self.partitions;
                    for (sum, (input_bin, impulse_bin)) in spectrum.iter_mut().zip(
                        self.input_history[input][history]
                            .iter()
                            .zip(&self.impulse_spectra[output][input][partition]),
                    ) {
                        *sum += *input_bin * *impulse_bin;
                    }
                }
            }
            self.transform.inverse(spectrum);
            for index in 0..BLOCK_SIZE {
                self.output_block[output][index] = spectrum[index].re + self.overlap[output][index];
                self.overlap[output][index] = spectrum[index + BLOCK_SIZE].re;
            }
        }
        self.history_position = (self.history_position + 1) % self.partitions;
    }
}

// H(s) = H_0 + b_1(s)*(H_100 - H_0) + b_2(s)*C
// TODO: Remove limiting to 6. Allow working across multiple subdivision
// for different slider type so everything doesn't seem long analyses.
impl InterpolatedStereoFir {
    pub(crate) fn new(
        base: [[Vec<f32>; 2]; 2],
        branches: [([[Vec<f32>; 2]; 2], [f32; 6]); 2],
        position: f32,
    ) -> Box<Self> {
        let [(primary, primary_gains), (correction, correction_gains)] = branches;
        let impulses = [base, primary, correction];
        let impulse_length = impulses
            .iter()
            .flat_map(|matrix| matrix.iter().flatten())
            .map(Vec::len)
            .max()
            .unwrap_or(1);
        let partitions = impulse_length.div_ceil(BLOCK_SIZE);
        let mut transform = FourierTransform::new(FFT_SIZE);
        let impulse_spectra = impulses
            .into_iter()
            .map(|matrix| {
                Box::new(std::array::from_fn(|output| {
                    std::array::from_fn(|input| {
                        (0..partitions)
                            .map(|partition| {
                                let mut spectrum = vec![Complex::default(); FFT_SIZE];
                                let start = partition * BLOCK_SIZE;
                                let end = (start + BLOCK_SIZE).min(matrix[output][input].len());
                                for (bin, sample) in
                                    spectrum.iter_mut().zip(&matrix[output][input][start..end])
                                {
                                    *bin = Complex::new(*sample, 0.0);
                                }
                                transform.forward(&mut spectrum);
                                spectrum
                            })
                            .collect()
                    })
                }))
            })
            .collect();
        let position = position.clamp(0.0, 1.0);
        Box::new(Self {
            transform,
            impulse_spectra,
            gain_knots: [primary_gains, correction_gains],
            input_history: Box::new(std::array::from_fn(|_| {
                (0..partitions)
                    .map(|_| vec![Complex::default(); FFT_SIZE])
                    .collect()
            })),
            input_block: [[0.0; BLOCK_SIZE]; 2],
            output_block: [[0.0; BLOCK_SIZE]; 2],
            output_spectra: std::array::from_fn(|_| vec![Complex::default(); FFT_SIZE]),
            overlap: [[0.0; BLOCK_SIZE]; 2],
            block_position: 0,
            history_position: 0,
            partitions,
            current_position: position,
            target_position: position,
        })
    }

    #[inline]
    pub(crate) fn process(&mut self, left: f32, right: f32, position: f32) -> (f32, f32) {
        self.target_position = position.clamp(0.0, 1.0);
        let output = (
            self.output_block[0][self.block_position],
            self.output_block[1][self.block_position],
        );
        self.input_block[0][self.block_position] = left;
        self.input_block[1][self.block_position] = right;
        self.block_position += 1;
        if self.block_position == BLOCK_SIZE {
            // FIXME: Pass around target_position(?)
            self.process_block();
            self.block_position = 0;
        }
        output
    }

    pub(crate) fn reset(&mut self) {
        for channel in self.input_history.iter_mut() {
            for partition in channel {
                partition.fill(Complex::default());
            }
        }
        self.input_block.fill([0.0; BLOCK_SIZE]);
        self.output_block.fill([0.0; BLOCK_SIZE]);
        for spectrum in &mut self.output_spectra {
            spectrum.fill(Complex::default());
        }
        self.overlap.fill([0.0; BLOCK_SIZE]);
        self.block_position = 0;
        self.history_position = 0;
        self.current_position = self.target_position;
    }

    fn process_block(&mut self) {
        // EWMA based interpol to new position to avoid too much instant change 
        // over 30ms frame. c = 1 - exp(-dt/tau) and dt = BLOCK_SIZE / f
        // HACK: arbitrary constant, generalize needed response time to average
        const SMOOTHING: f32 = 0.299_226_46;
        self.current_position += (self.target_position - self.current_position) * SMOOTHING;
        if (self.target_position - self.current_position).abs() < 1e-4 {
            self.current_position = self.target_position;
        }
        let scaled = self.current_position * 5.0;
        let lower = (scaled.floor() as usize).min(5);
        let upper = (lower + 1).min(5);
        let fraction = scaled - lower as f32;
        let gains = self
            .gain_knots
            .map(|knots| knots[lower] + (knots[upper] - knots[lower]) * fraction);

        for input in 0..2 {
            let spectrum = &mut self.input_history[input][self.history_position];
            spectrum.fill(Complex::default());
            for (bin, sample) in spectrum.iter_mut().zip(self.input_block[input]) {
                *bin = Complex::new(sample, 0.0);
            }
            self.transform.forward(spectrum);
        }

        for output in 0..2 {
            let spectrum = &mut self.output_spectra[output];
            spectrum.fill(Complex::default());
            for input in 0..2 {
                for partition in 0..self.partitions {
                    let history =
                        (self.history_position + self.partitions - partition) % self.partitions;
                    for index in 0..FFT_SIZE {
                        let impulse = self.impulse_spectra[0][output][input][partition][index]
                            + self.impulse_spectra[1][output][input][partition][index] * gains[0]
                            + self.impulse_spectra[2][output][input][partition][index] * gains[1];
                        spectrum[index] += self.input_history[input][history][index] * impulse;
                    }
                }
            }
            self.transform.inverse(spectrum);
            for index in 0..BLOCK_SIZE {
                self.output_block[output][index] = spectrum[index].re + self.overlap[output][index];
                self.overlap[output][index] = spectrum[index + BLOCK_SIZE].re;
            }
        }
        self.history_position = (self.history_position + 1) % self.partitions;
    }
}

pub(crate) fn read_float_wave(bytes: &[u8]) -> Vec<f32> {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let chunk_size =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let data_start = offset + 8;
        let data_end = data_start + chunk_size;
        if &bytes[offset..offset + 4] == b"data" {
            let (samples, remainder) = bytes[data_start..data_end].as_chunks::<4>();
            assert!(remainder.is_empty());
            return samples
                .iter()
                .map(|sample| f32::from_le_bytes(*sample))
                .collect();
        }
        offset = data_end + (chunk_size & 1);
    }
    panic!("embedded impulse response has no data chunk");
}
