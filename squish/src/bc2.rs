use crate::{
    alpha, colourblock, compress_bc1_bc2_bc3_colour_block, private, Decoder, Encoder, Params,
};

pub struct BC2 {}

impl private::Format for BC2 {
    fn block_size() -> usize {
        16
    }
}

impl private::Decoder for BC2 {
    fn decompress_block(block: &[u8]) -> [[u8; 4]; 16] {
        use private::Format;
        assert_eq!(block.len(), Self::block_size());
        // decompress colour block
        let mut rgba = colourblock::decompress(&block[8..16], true);
        // decompress alpha block(s)
        alpha::decompress_bc2(&mut rgba, &block[..8]);
        rgba
    }
}

impl Decoder for BC2 {}

impl private::Encoder for BC2 {
    fn compress_block_masked(rgba: [[u8; 4]; 16], mask: u32, params: Params, output: &mut [u8]) {
        compress_bc1_bc2_bc3_colour_block(rgba, mask, params, output, false);

        // compress alpha block(s)
        alpha::compress_bc2(&rgba, mask, &mut output[..8]);
    }
}

impl Encoder for BC2 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_storage_requirements() {
        assert_eq!(BC2::compressed_size(16, 32), 512);
        assert_eq!(BC2::compressed_size(15, 32), 512);
    }

    // Same RGB colours as DECODED_BLOCK_COLOUR_4X4, with additional alpha channel.
    // Alpha starts at 0x00, then increases by 0x11 for every pixel.
    static DECODED_BLOCK_RGBA_4X4: &[u8] = &[
        0xFF, 0x96, 0x4A, 0x00, 0xFF, 0x96, 0x4A, 0x11, // row 0, left half
        0xFF, 0x96, 0x4A, 0x22, 0xFF, 0x96, 0x4A, 0x33, // row 0, right half
        0xFF, 0x78, 0x34, 0x44, 0xFF, 0x78, 0x34, 0x55, // row 1, left half
        0xFF, 0x78, 0x34, 0x66, 0xFF, 0x78, 0x34, 0x77, // row 1, right half
        0xFF, 0x69, 0x29, 0x88, 0xFF, 0x69, 0x29, 0x99, // row 2, left half
        0xFF, 0x69, 0x29, 0xAA, 0xFF, 0x69, 0x29, 0xBB, // row 2, right half
        0xFF, 0x69, 0x29, 0xCC, 0xFF, 0x69, 0x29, 0xDD, // row 3, left half
        0xFF, 0x69, 0x29, 0xEE, 0xFF, 0x69, 0x29, 0xFF, // row 3, right half
    ];

    // Combine the same alpha channel (BC2-compressed) with RGB from ENCODED_BLOCK_COLOUR_4X4.
    // Alpha BC2 data created with GIMP DDS export.
    static ENCODED_BC2_BLOCK_ALPHA_4X4: [u8; 16] = [
        0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE, // Alpha
        0xA9, 0xFC, 0x45, 0xFB, 0x00, 0xFF, 0x55, 0x55, // RGB block
    ];

    #[test]
    fn test_bc2_decompression_colour() {
        let encoded: [u8; 16] = ENCODED_BC2_BLOCK_ALPHA_4X4;
        let mut output_actual = [0u8; 4 * 4 * 4];
        BC2::decompress(&encoded, 4, 4, &mut output_actual);
        let output_expected = DECODED_BLOCK_RGBA_4X4;
        assert_eq!(output_actual, output_expected);
    }

    #[test]
    fn test_bc2_compression_colour() {
        fn test(algorithm: Algorithm) {
            let mut output_actual = [0u8; 16];
            BC2::compress(
                &DECODED_BLOCK_RGBA_4X4,
                4,
                4,
                Params {
                    algorithm,
                    weights: COLOUR_WEIGHTS_UNIFORM,
                    weigh_colour_by_alpha: false,
                },
                &mut output_actual,
            );
            let output_expected = ENCODED_BC2_BLOCK_ALPHA_4X4;
            assert_eq!(output_actual, output_expected);
        }

        // all algorithms should result in the same expected output
        test(Algorithm::ClusterFit);
        test(Algorithm::RangeFit);
        test(Algorithm::IterativeClusterFit);
    }
}
