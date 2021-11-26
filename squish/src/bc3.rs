use crate::{
    alpha, colourblock, compress_bc1_bc2_bc3_colour_block, private, Decoder, Encoder, Params,
};

pub struct BC3 {}

impl private::Format for BC3 {
    fn block_size() -> usize {
        16
    }
}

impl private::Decoder for BC3 {
    fn decompress_block(block: &[u8]) -> [[u8; 4]; 16] {
        use private::Format;
        assert_eq!(block.len(), Self::block_size());
        // decompress colour block
        let mut rgba = colourblock::decompress(&block[8..16], true);
        // decompress alpha block(s)
        alpha::decompress_bc3(&mut rgba, 3, &block[..8]);
        rgba
    }
}

impl Decoder for BC3 {}

impl private::Encoder for BC3 {
    fn compress_block_masked(rgba: [[u8; 4]; 16], mask: u32, params: Params, output: &mut [u8]) {
        compress_bc1_bc2_bc3_colour_block(rgba, mask, params, output, false);

        // compress alpha block(s)
        alpha::compress_bc3(&rgba, 3, mask, &mut output[..8]);
    }
}

impl Encoder for BC3 {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_requirements() {
        assert_eq!(BC3::compressed_size(16, 32), 512);
        assert_eq!(BC3::compressed_size(15, 32), 512);
    }
}
