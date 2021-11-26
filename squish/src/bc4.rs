use crate::{alpha, private, Decoder, Encoder, Params};

pub struct BC4 {}

impl private::Format for BC4 {
    fn block_size() -> usize {
        8
    }
}

impl private::Decoder for BC4 {
    fn decompress_block(block: &[u8]) -> [[u8; 4]; 16] {
        use private::Format;
        assert_eq!(block.len(), Self::block_size());
        // decompress alpha
        let mut rgba = [[0u8; 4]; 16];
        alpha::decompress_bc3(&mut rgba, 0, &block[..8]);
        // splat decompressed value into g and b channels
        for ref mut pixel in rgba {
            pixel[1] = pixel[0];
            pixel[2] = pixel[0];
        }
        rgba
    }
}

impl Decoder for BC4 {}

impl private::Encoder for BC4 {
    fn compress_block_masked(rgba: [[u8; 4]; 16], mask: u32, _params: Params, output: &mut [u8]) {
        // compress alpha block(s)
        alpha::compress_bc3(&rgba, 0, mask, &mut output[..8]);
    }
}

impl Encoder for BC4 {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_requirements() {
        assert_eq!(BC4::compressed_size(16, 32), 256);
        assert_eq!(BC4::compressed_size(15, 32), 256);
    }
}
