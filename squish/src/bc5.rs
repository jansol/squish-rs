use crate::{alpha, private, Decoder, Encoder, Params};

pub struct BC5 {}

impl private::Format for BC5 {
    fn block_size() -> usize {
        16
    }
}

impl private::Decoder for BC5 {
    fn decompress_block(block: &[u8]) -> [[u8; 4]; 16] {
        use private::Format;
        assert_eq!(block.len(), Self::block_size());
        // decompress alpha
        let mut rgba = [[0u8; 4]; 16];
        alpha::decompress_bc3(&mut rgba, 0, &block[..8]);
        alpha::decompress_bc3(&mut rgba, 1, &block[8..16]);
        rgba
    }
}

impl Decoder for BC5 {}

impl private::Encoder for BC5 {
    fn compress_block_masked(rgba: [[u8; 4]; 16], mask: u32, _params: Params, output: &mut [u8]) {
        // compress alpha block(s)
        alpha::compress_bc3(&rgba, 0, mask, &mut output[0..8]);
        alpha::compress_bc3(&rgba, 1, mask, &mut output[8..16]);
    }
}

impl Encoder for BC5 {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_requirements() {
        assert_eq!(BC5::compressed_size(16, 32), 512);
        assert_eq!(BC5::compressed_size(15, 32), 512);
    }
}
