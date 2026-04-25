use rustfft::{num_complex::Complex32, Fft, FftPlanner};
use std::sync::Arc;

const BLOCK_SIZE: usize = 128;
const FFT_SIZE: usize = BLOCK_SIZE * 2;
const SOURCE_RATE: f32 = 44_100.0;

// MIT KEMAR at 0 degrees elevation and -30 degrees azimuth.
const LEFT_SOURCE_LEFT_EAR: &[u8] = include_bytes!("../full/elev0/L0e330a.wav");
const LEFT_SOURCE_RIGHT_EAR: &[u8] = include_bytes!("../full/elev0/R0e330a.wav");
// MIT KEMAR at 0 degrees elevation and +30 degrees azimuth.
const RIGHT_SOURCE_LEFT_EAR: &[u8] = include_bytes!("../full/elev0/L0e030a.wav");
const RIGHT_SOURCE_RIGHT_EAR: &[u8] = include_bytes!("../full/elev0/R0e030a.wav");

pub struct HrtfRenderer {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    forward_scratch: Vec<Complex32>,
    inverse_scratch: Vec<Complex32>,
    filters: [[Vec<Vec<Complex32>>; 2]; 2],
    history: [Vec<Vec<Complex32>>; 2],
    history_pos: usize,
    input: [[f32; BLOCK_SIZE]; 2],
    output: [[f32; BLOCK_SIZE]; 2],
    overlap: [[f32; BLOCK_SIZE]; 2],
    block_pos: usize,
    fft_buffer: Vec<Complex32>,
    spectrum: Vec<Complex32>,
}

impl HrtfRenderer {
    pub fn new(sample_rate: f32) -> Self {
        let mut responses = [
            [
                resample(
                    &decode_pcm16_wav(LEFT_SOURCE_LEFT_EAR),
                    SOURCE_RATE,
                    sample_rate,
                ),
                resample(
                    &decode_pcm16_wav(LEFT_SOURCE_RIGHT_EAR),
                    SOURCE_RATE,
                    sample_rate,
                ),
            ],
            [
                resample(
                    &decode_pcm16_wav(RIGHT_SOURCE_LEFT_EAR),
                    SOURCE_RATE,
                    sample_rate,
                ),
                resample(
                    &decode_pcm16_wav(RIGHT_SOURCE_RIGHT_EAR),
                    SOURCE_RATE,
                    sample_rate,
                ),
            ],
        ];

        // Preserve each response pair's interaural level difference.
        for source in &mut responses {
            let energy: f32 = source.iter().flatten().map(|sample| sample * sample).sum();
            let gain = energy.max(f32::EPSILON).sqrt().recip();
            for response in source {
                for sample in response {
                    *sample *= gain;
                }
            }
        }

        let partitions = responses
            .iter()
            .flatten()
            .map(|response| response.len().div_ceil(BLOCK_SIZE))
            .max()
            .unwrap_or(1);

        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(FFT_SIZE);
        let inverse = planner.plan_fft_inverse(FFT_SIZE);
        let filters = std::array::from_fn(|source| {
            std::array::from_fn(|ear| {
                partition_response(&responses[source][ear], partitions, &forward)
            })
        });
        let history = std::array::from_fn(|_| vec![vec![Complex32::ZERO; FFT_SIZE]; partitions]);

        Self {
            forward_scratch: vec![Complex32::ZERO; forward.get_inplace_scratch_len()],
            inverse_scratch: vec![Complex32::ZERO; inverse.get_inplace_scratch_len()],
            forward,
            inverse,
            filters,
            history,
            history_pos: 0,
            input: [[0.0; BLOCK_SIZE]; 2],
            output: [[0.0; BLOCK_SIZE]; 2],
            overlap: [[0.0; BLOCK_SIZE]; 2],
            block_pos: 0,
            fft_buffer: vec![Complex32::ZERO; FFT_SIZE],
            spectrum: vec![Complex32::ZERO; FFT_SIZE],
        }
    }

    #[inline]
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let out = (
            self.output[0][self.block_pos],
            self.output[1][self.block_pos],
        );
        self.input[0][self.block_pos] = left;
        self.input[1][self.block_pos] = right;
        self.block_pos += 1;

        if self.block_pos == BLOCK_SIZE {
            self.process_block();
            self.block_pos = 0;
        }
        out
    }

    fn process_block(&mut self) {
        for source in 0..2 {
            self.fft_buffer.fill(Complex32::ZERO);
            for (destination, sample) in self.fft_buffer[..BLOCK_SIZE]
                .iter_mut()
                .zip(self.input[source])
            {
                destination.re = sample;
            }
            self.forward
                .process_with_scratch(&mut self.fft_buffer, &mut self.forward_scratch);
            self.history[source][self.history_pos].copy_from_slice(&self.fft_buffer);
        }

        let partitions = self.history[0].len();
        let scale = (FFT_SIZE as f32).recip();
        for ear in 0..2 {
            self.spectrum.fill(Complex32::ZERO);
            for source in 0..2 {
                for partition in 0..partitions {
                    let history_index = (self.history_pos + partitions - partition) % partitions;
                    for bin in 0..FFT_SIZE {
                        self.spectrum[bin] += self.history[source][history_index][bin]
                            * self.filters[source][ear][partition][bin];
                    }
                }
            }

            self.inverse
                .process_with_scratch(&mut self.spectrum, &mut self.inverse_scratch);
            for sample in 0..BLOCK_SIZE {
                self.output[ear][sample] =
                    self.spectrum[sample].re * scale + self.overlap[ear][sample];
                self.overlap[ear][sample] = self.spectrum[sample + BLOCK_SIZE].re * scale;
            }
        }
        self.history_pos = (self.history_pos + 1) % partitions;
    }
}

fn partition_response(
    response: &[f32],
    partitions: usize,
    fft: &Arc<dyn Fft<f32>>,
) -> Vec<Vec<Complex32>> {
    (0..partitions)
        .map(|partition| {
            let mut block = vec![Complex32::ZERO; FFT_SIZE];
            let offset = partition * BLOCK_SIZE;
            let count = BLOCK_SIZE.min(response.len().saturating_sub(offset));
            for (destination, sample) in block[..count]
                .iter_mut()
                .zip(&response[offset..offset + count])
            {
                destination.re = *sample;
            }
            fft.process(&mut block);
            block
        })
        .collect()
}

fn decode_pcm16_wav(bytes: &[u8]) -> Vec<f32> {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");

    let mut cursor = 12;
    while cursor + 8 <= bytes.len() {
        let chunk_size =
            u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let data_start = cursor + 8;
        let data_end = data_start + chunk_size;
        assert!(data_end <= bytes.len(), "invalid embedded HRTF WAV");
        if &bytes[cursor..cursor + 4] == b"data" {
            return bytes[data_start..data_end]
                .chunks_exact(2)
                .map(|sample| i16::from_le_bytes(sample.try_into().unwrap()) as f32 / 32768.0)
                .collect();
        }
        cursor = data_end + (chunk_size & 1);
    }
    panic!("embedded HRTF WAV has no data chunk");
}

fn resample(input: &[f32], source_rate: f32, target_rate: f32) -> Vec<f32> {
    if (source_rate - target_rate).abs() < f32::EPSILON {
        return input.to_vec();
    }
    let output_len = (((input.len() - 1) as f32 * target_rate / source_rate).round() as usize) + 1;
    (0..output_len)
        .map(|index| {
            let source_position = index as f32 * source_rate / target_rate;
            let lower = (source_position.floor() as usize).min(input.len() - 1);
            let upper = (lower + 1).min(input.len() - 1);
            let fraction = source_position - lower as f32;
            input[lower] * (1.0 - fraction) + input[upper] * fraction
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_direction() {
        let mut renderer = HrtfRenderer::new(48_000.0);
        let (mut left, mut right) = (0.0, 0.0);
        for index in 0..1024 {
            let output = renderer.process(0.0, (index == 0) as u8 as f32);
            left += output.0 * output.0;
            right += output.1 * output.1;
        }
        assert!(right > left * 3.0);
    }
}
