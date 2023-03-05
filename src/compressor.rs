//! Module that contains things related with compressing a image.
//!
//! # Compressor
//!
//! The `compress_to_jpg` function resizes the given image and compresses it by a certain percentage.
//! # Examples
//! ```
//! use std::path::PathBuf;
//! use image_compressor::compressor::Compressor;
//! use image_compressor::Factor;
//!
//! let origin_dir = PathBuf::from("origin").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let compressor = Compressor::new(image::load_from_memory(include_bytes!("../tests/test.jpg")).unwrap());
//! compressor.compress_image().expect("panic");
//! ```

use std::error::Error;

use image::imageops::FilterType;
use mozjpeg::{ColorSpace, Compress, ScanMode};

/// Factor struct that used for setting quality and resize ratio in the new image.
///
/// The [`Compressor`] and [`FolderCompressor`](super::FolderCompressor) need a function pointer that
/// calculate and return the `Factor` for compressing images.
///
/// So, to create a new `Compressor` or `FolderCompressor` instance
/// you need to define a new function or closure that calculates and returns a `Factor` instance
/// based on the size of image(width and height) and file size.
///
/// The recommended range of quality is 60 to 80.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Factor {
    /// Quality of the new compressed image.
    /// Values range from 0 to 100 in float.
    quality: f32,

    /// Ratio for resize the new compressed image.
    /// Values range from 0 to 1 in float.
    size_ratio: f32,
}

impl Factor {
    /// Create a new `Factor` instance.
    /// The `quality` range from 0 to 100 in float,
    /// and `size_ratio` range from 0 to 1 in float.
    ///
    /// # Panics
    ///
    /// - If the quality value is 0 or less.
    /// - If the quality value exceeds 100.
    /// - If the size ratio value is 0 or less.
    /// - If the size ratio value exceeds 1.
    pub fn new(quality: f32, size_ratio: f32) -> Self {
        if (quality > 0. && quality <= 100.) && (size_ratio > 0. && size_ratio <= 1.) {
            Self {
                quality,
                size_ratio,
            }
        } else {
            panic!("Wrong Factor argument!");
        }
    }

    /// Getter for `quality` of `Factor`.
    pub fn quality(&self) -> f32 {
        self.quality
    }

    /// Getter for `size_ratio` of `Factor`.
    pub fn size_ratio(&self) -> f32 {
        self.size_ratio
    }
}

impl Default for Factor {
    fn default() -> Self {
        Self {
            quality: 80.,
            size_ratio: 0.8,
        }
    }
}

/// Compressor struct.
///
pub struct Compressor {
    factor: Factor,
    image: image::DynamicImage,
}

impl Compressor {
    /// Create a new compressor.
    ///
    /// The new `Compressor` instance needs a function to calculate quality and scaling factor of the new compressed image.
    /// For more information of `cal_factor_func` parameter, please check the [`Factor`] struct.
    ///
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use image_compressor::compressor::Compressor;
    /// use image_compressor::Factor;
    ///
    /// let origin_dir = PathBuf::from("origin").join("file1.jpg");
    /// let dest_dir = PathBuf::from("dest");
    ///
    /// let compressor = Compressor::new(image::load_from_memory(include_bytes!("../tests/test.jpg")).unwrap());
    /// ```
    pub fn new(image: image::DynamicImage) -> Self {
        Compressor {
            factor: Factor::default(),
            image,
        }
    }

    /// Set factor for the new compressed image.
    pub fn set_factor(&mut self, factor: Factor) {
        self.factor = factor;
    }

    fn compress(
        &self,
        resized_img_data: Vec<u8>,
        target_width: usize,
        target_height: usize,
        quality: f32,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut comp = Compress::new(ColorSpace::JCS_RGB);
        comp.set_scan_optimization_mode(ScanMode::Auto);
        comp.set_quality(quality);

        comp.set_size(target_width, target_height);

        comp.set_mem_dest();
        comp.set_optimize_scans(true);
        comp.start_compress();

        let mut line = 0;
        loop {
            if line > target_height - 1 {
                break;
            }
            comp.write_scanlines(
                &resized_img_data[line * target_width * 3..(line + 1) * target_width * 3],
            );
            line += 1;
        }
        comp.finish_compress();

        let compressed = comp
            .data_to_vec()
            .map_err(|_| "data_to_vec failed".to_string())?;
        Ok(compressed)
    }

    fn resize(
        &self,
        resize_ratio: f32,
    ) -> Result<(Vec<u8>, usize, usize), Box<dyn Error>> {
        let img = &self.image;
        let width = img.width() as usize;
        let height = img.height() as usize;

        let width = width as f32 * resize_ratio;
        let height = height as f32 * resize_ratio;

        let resized_img = img.resize(width as u32, height as u32, FilterType::Triangle);

        let resized_width = resized_img.width() as usize;
        let resized_height = resized_img.height() as usize;

        Ok((
            resized_img.into_rgb8().into_vec(),
            resized_width,
            resized_height,
        ))
    }

    /// Compress a file.
    ///
    /// Compress the given image file and save it to target_dir.
    /// If the extension of the given image file is not jpg or jpeg, then convert the image to jpg file.
    /// If the module can not open the file, just copy it to target_dir.
    /// Compress quality and resize ratio calculate based on file size of the image.
    /// For a continuous multithreading process, every single error doesn't occur panic or exception and just print error message with return Ok.
    ///
    /// If the flag to delete the original is true, the function delete the original file.
    ///
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use image_compressor::compressor::Compressor;
    /// use image_compressor::Factor;
    ///
    /// let origin_dir = PathBuf::from("origin").join("file1.jpg");
    /// let dest_dir = PathBuf::from("dest");
    ///
    /// let compressor = Compressor::new(image::load_from_memory(include_bytes!("../tests/test.jpg")).unwrap());
    /// compressor.compress_image().expect("panic");
    /// ```
    pub fn compress_image(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let (resized_img_data, target_width, target_height) =
            self.resize(self.factor.size_ratio())?;
        let compressed_img_data = self.compress(
            resized_img_data,
            target_width,
            target_height,
            self.factor.quality(),
        )?;

        Ok(compressed_img_data)
    }
}
