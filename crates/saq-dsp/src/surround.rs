use crate::fft::{Complex, FourierTransform};

const BLOCK_SIZE: usize = 512;
const FFT_SIZE: usize = BLOCK_SIZE * 2;
const IMPULSE_LENGTH: usize = 16_384;
const PARTITIONS: usize = IMPULSE_LENGTH.div_ceil(BLOCK_SIZE);

const DIRECT_LEFT: &[u8] = include_bytes!("../assets/surround/direct-left.wav");
const RIGHT_TO_LEFT: &[u8] = include_bytes!("../assets/surround/right-to-left.wav");
const LEFT_TO_RIGHT: &[u8] = include_bytes!("../assets/surround/left-to-right.wav");
const DIRECT_RIGHT: &[u8] = include_bytes!("../assets/surround/direct-right.wav");

// Partitioned convolution applying long impulse response with
// bounded block latency by combining the convolution and overlap add
pub struct SurroundEngine {
    transform: FourierTransform,
    impulse_spectra: Box<[[[Vec<Complex>; PARTITIONS]; 2]; 2]>,
    input_history: Box<[[Vec<Complex>; PARTITIONS]; 2]>,
    input_block: [[f32; BLOCK_SIZE]; 2],
    output_block: [[f32; BLOCK_SIZE]; 2],
    output_spectra: [Vec<Complex>; 2],
    overlap: [[f32; BLOCK_SIZE]; 2],
    block_position: usize,
    history_position: usize,
}

impl SurroundEngine {
    pub fn new() -> Box<Self> {
        let impulses = [
            [read_float_wave(DIRECT_LEFT), read_float_wave(RIGHT_TO_LEFT)],
            [
                read_float_wave(LEFT_TO_RIGHT),
                read_float_wave(DIRECT_RIGHT),
            ],
        ];

        let mut transform = FourierTransform::new(FFT_SIZE);
        let mut impulse_spectra = Box::new(std::array::from_fn(|_| {
            std::array::from_fn(|_| std::array::from_fn(|_| vec![Complex::default(); FFT_SIZE]))
        }));
        for output in 0..2 {
            for input in 0..2 {
                for (partition, spectrum) in impulse_spectra[output][input].iter_mut().enumerate() {
                    let start = partition * BLOCK_SIZE;
                    let end = (start + BLOCK_SIZE).min(impulses[output][input].len());
                    for (bin, sample) in spectrum
                        .iter_mut()
                        .zip(&impulses[output][input][start..end])
                    {
                        *bin = Complex::new(*sample, 0.0);
                    }
                    transform.forward(spectrum);
                }
            }
        }

        Box::new(Self {
            transform,
            impulse_spectra,
            input_history: Box::new(std::array::from_fn(|_| {
                std::array::from_fn(|_| vec![Complex::default(); FFT_SIZE])
            })),
            input_block: [[0.0; BLOCK_SIZE]; 2],
            output_block: [[0.0; BLOCK_SIZE]; 2],
            output_spectra: std::array::from_fn(|_| vec![Complex::default(); FFT_SIZE]),
            overlap: [[0.0; BLOCK_SIZE]; 2],
            block_position: 0,
            history_position: 0,
        })
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
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
                for partition in 0..PARTITIONS {
                    let history = (self.history_position + PARTITIONS - partition) % PARTITIONS;
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

        self.history_position = (self.history_position + 1) % PARTITIONS;
    }
}

fn read_float_wave(bytes: &[u8]) -> Vec<f32> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_impulse_matches_embedded_response() {
        let expected_left = read_float_wave(DIRECT_LEFT);
        let expected_right = read_float_wave(LEFT_TO_RIGHT);
        let mut engine = SurroundEngine::new();
        let mut output = Vec::with_capacity(BLOCK_SIZE + IMPULSE_LENGTH);

        for index in 0..BLOCK_SIZE + IMPULSE_LENGTH {
            output.push(engine.process((index == 0) as u8 as f32, 0.0));
        }

        for index in 0..IMPULSE_LENGTH {
            let actual = output[index + BLOCK_SIZE];
            assert!((actual.0 - expected_left[index]).abs() < 2.0e-4);
            assert!((actual.1 - expected_right[index]).abs() < 2.0e-4);
        }
    }
}
