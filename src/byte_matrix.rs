//! Bounded, exact byte histograms for presentation. No type or code inference.
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_WINDOWS: usize = 64;

#[derive(Debug)]
pub struct ByteWindow {
    pub offset: u64,
    pub length: usize,
    pub counts: [u32; 256],
    pub entropy: f64,
    pub preview: Vec<u8>,
}

#[derive(Debug)]
pub struct ByteMatrix {
    pub length: usize,
    pub windows: Vec<ByteWindow>,
}

impl ByteMatrix {
    pub fn from_bytes(bytes: &[u8], base: u64) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES || base.checked_add(bytes.len() as u64).is_none() {
            return Err("Byte matrix requires a bounded, non-overflowing source span".into());
        }
        let size = bytes.len().div_ceil(MAX_WINDOWS).max(1);
        let windows = bytes
            .chunks(size)
            .enumerate()
            .map(|(index, chunk)| {
                let mut counts = [0u32; 256];
                for byte in chunk {
                    counts[usize::from(*byte)] += 1;
                }
                let entropy = counts
                    .iter()
                    .filter(|&&count| count != 0)
                    .map(|&count| {
                        let p = f64::from(count) / chunk.len() as f64;
                        -p * p.log2()
                    })
                    .sum::<f64>()
                    .max(0.0);
                ByteWindow {
                    offset: base + (index * size) as u64,
                    length: chunk.len(),
                    counts,
                    entropy,
                    preview: chunk[..chunk.len().min(32)].to_vec(),
                }
            })
            .collect();
        Ok(Self {
            length: bytes.len(),
            windows,
        })
    }

    pub fn max_frequency(&self) -> f32 {
        self.windows
            .iter()
            .flat_map(|window| {
                window
                    .counts
                    .iter()
                    .map(move |&count| count as f32 / window.length as f32)
            })
            .fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_windows_have_eight_bits_and_exact_counts() {
        let bytes: Vec<_> = (0..64).flat_map(|_| 0..=255u8).collect();
        let data = ByteMatrix::from_bytes(&bytes, 0x4000).unwrap();
        assert_eq!(data.windows.len(), 64);
        for (index, window) in data.windows.iter().enumerate() {
            assert_eq!(window.offset, 0x4000 + (index * 256) as u64);
            assert_eq!(window.counts, [1; 256]);
            assert!((window.entropy - 8.0).abs() < 1e-10);
        }
    }

    #[test]
    fn padding_and_structured_regions_remain_distinct() {
        let bytes: Vec<_> = std::iter::repeat_n(0u8, 256 * 32)
            .chain((0..32).flat_map(|_| 0..=255u8))
            .collect();
        let data = ByteMatrix::from_bytes(&bytes, 0).unwrap();
        assert_eq!(data.windows[0].entropy, 0.0);
        assert_eq!(data.windows[0].counts[0], 256);
        assert_eq!(data.windows[32].entropy, 8.0);
        assert_eq!(data.max_frequency(), 1.0);
    }

    #[test]
    fn uneven_final_window_preserves_all_bytes_and_normalizes_its_own_length() {
        let data = ByteMatrix::from_bytes(&[0xff; 129], 9).unwrap();
        assert!(data.windows.len() <= MAX_WINDOWS);
        assert_eq!(data.windows.iter().map(|w| w.length).sum::<usize>(), 129);
        assert_eq!(
            data.windows.last().unwrap().offset + data.windows.last().unwrap().length as u64,
            138
        );
        assert_eq!(data.max_frequency(), 1.0);
        assert!(
            data.windows
                .iter()
                .all(|w| w.counts.iter().sum::<u32>() as usize == w.length)
        );
    }

    #[test]
    fn empty_and_overflow_are_explicit() {
        let data = ByteMatrix::from_bytes(&[], 0).unwrap();
        assert!(data.windows.is_empty());
        assert_eq!(data.max_frequency(), 0.0);
        assert!(ByteMatrix::from_bytes(&[1, 2], u64::MAX).is_err());
    }
}
