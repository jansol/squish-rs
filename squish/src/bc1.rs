use crate::{colourblock, compress_bc1_bc2_bc3_colour_block, private, Decoder, Encoder, Params};

pub struct BC1 {}

impl private::Format for BC1 {
    fn block_size() -> usize {
        8
    }
}

impl private::Decoder for BC1 {
    fn decompress_block(block: &[u8]) -> [[u8; 4]; 16] {
        use private::Format;
        assert_eq!(block.len(), Self::block_size());
        // decompress colour block
        colourblock::decompress(block, true)
    }
}

impl Decoder for BC1 {}

impl private::Encoder for BC1 {
    fn compress_block_masked(rgba: [[u8; 4]; 16], mask: u32, params: Params, output: &mut [u8]) {
        compress_bc1_bc2_bc3_colour_block(rgba, mask, params, output, true)
    }
}

impl Encoder for BC1 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_storage_requirements() {
        assert_eq!(BC1::compressed_size(16, 32), 256);
        assert_eq!(BC1::compressed_size(15, 32), 256);
    }

    // The test-pattern is a gray-scale checkerboard of size 4x4 with 0xFF in the top-left.
    // On top of that, the four middle pixels are set to 0x7F.
    static DECODED_BLOCK_GRAY_4X4: &[u8] = &[
        0xFF, 0x00, 0xFF, 0x00, // row 0
        0x00, 0x7F, 0x7F, 0xFF, // row 1
        0xFF, 0x7F, 0x7F, 0x00, // row 2
        0x00, 0xFF, 0x00, 0xFF, // row 3
    ];

    fn decoded_block_gray_4x4_as_rgba() -> [u8; 4 * 4 * 4] {
        let mut output = [0u8; 4 * 4 * 4];
        for i in 0..DECODED_BLOCK_GRAY_4X4.len() {
            output[i * 4 + 0] = DECODED_BLOCK_GRAY_4X4[i]; // R
            output[i * 4 + 1] = DECODED_BLOCK_GRAY_4X4[i]; // G
            output[i * 4 + 2] = DECODED_BLOCK_GRAY_4X4[i]; // B
            output[i * 4 + 3] = 0xFF; //A
        }
        output
    }

    #[test]
    fn test_bc1_decompression_gray() {
        // BC1 data created with AMD Compressonator v4.1.5083
        let encoded: [u8; 8] = [0x00, 0x00, 0xFF, 0xFF, 0x11, 0x68, 0x29, 0x44];
        let mut output_actual = [0u8; 4 * 4 * 4];
        BC1::decompress(&encoded, 4, 4, &mut output_actual);
        assert_eq!(output_actual, decoded_block_gray_4x4_as_rgba());
    }

    #[test]
    fn test_bc1_compression_gray() {
        fn test(algorithm: Algorithm) {
            let mut output_actual = [0u8; 8];
            BC1::compress(
                &decoded_block_gray_4x4_as_rgba(),
                4,
                4,
                Params {
                    algorithm,
                    weights: COLOUR_WEIGHTS_UNIFORM,
                    weigh_colour_by_alpha: false,
                },
                &mut output_actual,
            );
            // BC1 data created with AMD Compressonator v4.1.5083
            let output_expected = [0x00, 0x00, 0xFF, 0xFF, 0x11, 0x68, 0x29, 0x44];
            assert_eq!(output_actual, output_expected);
        }

        // all algorithms should result in the same expected output
        test(Algorithm::ClusterFit);
        test(Algorithm::RangeFit);
        test(Algorithm::IterativeClusterFit);
    }

    // A colour test-pattern (RGB) with the first row in one colour,
    // the second in another and the third and last row in a third colour.
    static DECODED_BLOCK_COLOUR_4X4: &[u8] = &[
        255, 150, 74, 255, 150, 74, 255, 150, 74, 255, 150, 74, // row 0
        255, 120, 52, 255, 120, 52, 255, 120, 52, 255, 120, 52, // row 1
        255, 105, 41, 255, 105, 41, 255, 105, 41, 255, 105, 41, // row 2
        255, 105, 41, 255, 105, 41, 255, 105, 41, 255, 105, 41, // row 3
    ];

    // BC1 data created with AMD Compressonator v4.1.5083 and is the same as libsquish
    static ENCODED_BLOCK_COLOUR_4X4: [u8; 8] = [0xA9, 0xFC, 0x45, 0xFB, 0x00, 0xFF, 0x55, 0x55];

    fn decoded_block_colour_4x4_as_rgba() -> [u8; 4 * 4 * 4] {
        let mut output = [0u8; 4 * 4 * 4];
        for i in 0..4 * 4 {
            output[i * 4 + 0] = DECODED_BLOCK_COLOUR_4X4[i * 3 + 0]; // R
            output[i * 4 + 1] = DECODED_BLOCK_COLOUR_4X4[i * 3 + 1]; // G
            output[i * 4 + 2] = DECODED_BLOCK_COLOUR_4X4[i * 3 + 2]; // B
            output[i * 4 + 3] = 0xFF; //A
        }
        output
    }

    #[test]
    fn test_bc1_decompression_colour() {
        let encoded: [u8; 8] = ENCODED_BLOCK_COLOUR_4X4;
        let mut output_actual = [0u8; 4 * 4 * 4];
        BC1::decompress(&encoded, 4, 4, &mut output_actual);
        assert_eq!(output_actual, decoded_block_colour_4x4_as_rgba());
    }

    #[test]
    fn test_bc1_compression_colour() {
        fn test(algorithm: Algorithm) {
            let mut output_actual = [0u8; 8];
            BC1::compress(
                &decoded_block_colour_4x4_as_rgba(),
                4,
                4,
                Params {
                    algorithm,
                    weights: COLOUR_WEIGHTS_UNIFORM,
                    weigh_colour_by_alpha: false,
                },
                &mut output_actual,
            );
            let output_expected = ENCODED_BLOCK_COLOUR_4X4;
            assert_eq!(output_actual, output_expected);
        }

        // all algorithms should result in the same expected output
        test(Algorithm::ClusterFit);
        test(Algorithm::RangeFit);
        test(Algorithm::IterativeClusterFit);
    }
}
