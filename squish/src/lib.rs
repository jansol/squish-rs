// Copyright (c) 2006 Simon Brown <si@sjbrown.co.uk>
// Copyright (c) 2018-2021 Jan Solanti <jhs@psonet.com>
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to	deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! A pure Rust BC1/2/3 compressor and decompressor based on Simon Brown's
//! **libsquish**.
//!
//! BCn formats are laid out in 8-byte blocks of the following types:
//! * BC1: colour with optional 1-bit alpha
//! * BC2: paletted alpha, colour
//! * BC3: gradient alpha, colour
//! * BC4: gradient alpha
//! * BC5: gradient alpha, gradient alpha
//!
//! BC4 and BC5 reuse the alpha compression scheme for arbitrary one- and two-channel images.
//! Graphics APIs commonly refer to them as "grayscale", "luminance" or simply "red" for BC4 and
//! "rg" or "luminance + alpha" for BC5 respectively.

#![no_std]

mod alpha;
mod bc1;
mod bc2;
mod bc3;
mod bc4;
mod bc5;
mod colourblock;
mod colourfit;
mod colourset;
mod math;

use crate::colourfit::{ClusterFit, ColourFit, RangeFit, SingleColourFit};
use crate::colourset::ColourSet;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

// re-export the BC formats
pub use bc1::BC1;
pub use bc2::BC2;
pub use bc3::BC3;
pub use bc4::BC4;
pub use bc5::BC5;

/// Defines a compression algorithm
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// Fast, low quality
    RangeFit,

    /// Slow, high quality
    ClusterFit,

    /// Very slow, very high quality
    IterativeClusterFit,
}

impl Default for Algorithm {
    fn default() -> Self {
        Algorithm::ClusterFit
    }
}

/// RGB colour channel weights for use in block fitting
pub type ColourWeights = [f32; 3];

/// Uniform weights for each colour channel
pub const COLOUR_WEIGHTS_UNIFORM: ColourWeights = [1.0, 1.0, 1.0];

/// Weights based on the perceived brightness of each colour channel
pub const COLOUR_WEIGHTS_PERCEPTUAL: ColourWeights = [0.2126, 0.7152, 0.0722];

#[derive(Clone, Copy)]
pub struct Params {
    /// The compression algorithm to be used
    pub algorithm: Algorithm,

    /// Weigh the relative importance of each colour channel when fitting
    /// (defaults to perceptual weights)
    pub weights: ColourWeights,

    /// Weigh colour by alpha during cluster fit (defaults to false)
    ///
    /// This can significantly increase perceived quality for images that are rendered
    /// using alpha blending.
    pub weigh_colour_by_alpha: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            algorithm: Algorithm::default(),
            weights: COLOUR_WEIGHTS_PERCEPTUAL,
            weigh_colour_by_alpha: false,
        }
    }
}

/// Returns number of blocks needed for an image of given dimension
fn num_blocks(size: usize) -> usize {
    (size + 3) / 4
}

/// This module is used for sealing traits.
/// See <https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed>
mod private {
    use crate::*;

    pub trait Format {
        /// Returns how many bytes a 4x4 block of pixels will compress into.
        fn block_size() -> usize;
    }

    pub trait Decoder: Format {
        /// Decompresses a 4x4 block of pixels
        ///
        /// * `block`  - The compressed block of pixels
        /// * `output` - Storage for the decompressed block of pixels
        fn decompress_block(block: &[u8]) -> [[u8; 4]; 16];
    }

    pub trait Encoder: Format {
        /// Compresses a 4x4 block of pixels, masking out some pixels e.g. for padding the
        /// image to a multiple of the block size.
        ///
        /// * `rgba`   - The uncompressed block of pixels
        /// * `mask`   - The valid pixel mask
        /// * `params` - Additional compressor parameters
        /// * `output` - Storage for the compressed block
        fn compress_block_masked(rgba: [[u8; 4]; 16], mask: u32, params: Params, output: &mut [u8]);
    }
}

/// Abstraction over any decoder for any format.
/// Note that this trait is sealed, i.e. it can not be implemented outside of this crate.
pub trait Decoder: private::Decoder {
    /// Decompresses an image in memory
    ///
    /// * `data`   - The compressed image data
    /// * `width`  - The width of the source image
    /// * `height` - The height of the source image
    /// * `output` - Space to store the decompressed image
    fn decompress(data: &[u8], width: usize, height: usize, output: &mut [u8]) {
        let blocks_wide = num_blocks(width);
        let block_size = Self::block_size();

        #[cfg(feature = "rayon")]
        let output_rows = output.par_chunks_mut(width * 4 * 4);
        #[cfg(not(feature = "rayon"))]
        let output_rows = output.chunks_mut(width * 4 * 4);

        // loop over blocks
        output_rows.enumerate().for_each(|(y, output_row)| {
            for x in 0..blocks_wide {
                // decompress the block
                let bidx = (x + y * blocks_wide) * block_size;
                let rgba = Self::decompress_block(&data[bidx..bidx + block_size]);

                // write the decompressed pixels to the correct image location
                for py in 0..4 {
                    for px in 0..4 {
                        // get target location
                        let sx = 4 * x + px;
                        let sy = py;

                        if sx < width && sy < height {
                            for i in 0..4 {
                                output_row[4 * (sx + sy * width) + i] = rgba[px + py * 4][i];
                            }
                        }
                    }
                }
            }
        });
    }
}

/// Abstraction over any encoder for any format.
/// Note that this trait is sealed, i.e. it can not be implemented outside of this crate.
pub trait Encoder: private::Encoder {
    /// Computes the amount of space in bytes needed for an image of given size,
    /// accounting for padding to a multiple of 4x4 pixels
    ///
    /// * `width`  - Width of the uncompressed image
    /// * `height` - Height of the uncompressed image
    fn compressed_size(width: usize, height: usize) -> usize {
        // Number of blocks required for image of given dimensions
        let blocks = num_blocks(width) * num_blocks(height);
        blocks * Self::block_size()
    }

    /// Compresses an image in memory
    ///
    /// * `rgba`   - The uncompressed pixel data
    /// * `width`  - The width of the source image
    /// * `height` - The height of the source image
    /// * `params` - Additional compressor parameters
    /// * `output` - Output buffer for the compressed image. Ensure that this has
    /// at least as much space available as `compute_compressed_size` suggests.
    fn compress(rgba: &[u8], width: usize, height: usize, params: Params, output: &mut [u8]) {
        assert!(output.len() >= Self::compressed_size(width, height));

        let block_size = Self::block_size();
        let blocks_wide = num_blocks(width);

        #[cfg(feature = "rayon")]
        let output_rows = output.par_chunks_mut(blocks_wide * block_size);
        #[cfg(not(feature = "rayon"))]
        let output_rows = output.chunks_mut(blocks_wide * block_size);

        output_rows.enumerate().for_each(|(y, output_row)| {
            let mut source_rgba = [[0u8; 4]; 16];
            let output_blocks = output_row.chunks_mut(block_size);

            output_blocks.enumerate().for_each(|(x, output_block)| {
                // build the 4x4 block of pixels
                let mut mask = 0u32;
                for py in 0..4 {
                    for px in 0..4 {
                        let index = 4 * py + px;

                        // get position in source image
                        let sx = 4 * x + px;
                        let sy = 4 * y + py;

                        // enable pixel if within bounds
                        if sx < width && sy < height {
                            // copy pixel value
                            let src_index = 4 * (width * sy + sx);
                            source_rgba[index].copy_from_slice(&rgba[src_index..src_index + 4]);

                            // enable pixel
                            mask |= 1 << index;
                        }
                    }
                }

                Self::compress_block_masked(source_rgba, mask, params, output_block);
            });
        });
    }
}

fn compress_bc1_bc2_bc3_colour_block(
    rgba: [[u8; 4]; 16],
    mask: u32,
    params: Params,
    output: &mut [u8],
    is_bc1: bool,
) {
    // create the minimal point set
    let colours = ColourSet::new(&rgba, mask, is_bc1, params.weigh_colour_by_alpha);

    let colour_offset = if is_bc1 { 0 } else { 8 };
    let colour_block = &mut output[colour_offset..colour_offset + 8];

    // compress with appropriate compression algorithm
    if colours.count() == 1 {
        // Single colour fit can't handle fully transparent blocks, hence the
        // set has to contain at least 1 colour. It's also not very useful for
        // anything more complex so we only use it for blocks of uniform colour.
        let mut fit = SingleColourFit::new(&colours, is_bc1);
        fit.compress(colour_block);
    } else if (params.algorithm == Algorithm::RangeFit) || (colours.count() == 0) {
        let mut fit = RangeFit::new(&colours, is_bc1, params.weights);
        fit.compress(colour_block);
    } else {
        let iterate = params.algorithm == Algorithm::IterativeClusterFit;
        let mut fit = ClusterFit::new(&colours, is_bc1, params.weights, iterate);
        fit.compress(colour_block);
    }
}

//--------------------------------------------------------------------------------
// Unit tests
//--------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_num_blocks() {
        assert_eq!(num_blocks(0), 0);
        assert_eq!(num_blocks(1), 1);
        assert_eq!(num_blocks(2), 1);
        assert_eq!(num_blocks(3), 1);
        assert_eq!(num_blocks(4), 1);
        assert_eq!(num_blocks(5), 2);
        assert_eq!(num_blocks(6), 2);
    }
}
