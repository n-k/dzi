use std::io::Write;
use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView, ImageError, RgbImage};

#[derive(thiserror::Error, Debug)]
pub enum TilingError {
    #[error("Unsupported source image: {0}")]
    UnsupportedSourceImage(String),
    #[error("Unexpected error")]
    UnexpectedError,
    #[error("Unsupported source image: {0}")]
    ImageError(#[from] ImageError),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Input dimensions do not match input rgb data length")]
    IncorrectRGBInputDimensions,
}

pub type DZIResult<T, E = TilingError> = Result<T, E>;

/// A tile creator, this struct and associated functions
/// implement the DZI tiler
pub struct TileCreator {
    /// path of destination directory where tiles will be stored
    pub dest_path: PathBuf,
    /// path of .dzi descriptor file to be created
    pub dzi_file_path: PathBuf,
    /// source image
    pub image: DynamicImage,
    /// size of individual tiles in pixels
    pub tile_size: u32,
    /// number of pixels neighboring tiles overlap
    pub tile_overlap: u32,
    /// total number of levels of tiles
    pub levels: u32,
}

impl TileCreator {
    pub fn new_from_image_path(
        image_path: &Path,
        tile_size: u32,
        tile_overlap: u32,
    ) -> DZIResult<Self> {
        let im = image::io::Reader::open(image_path)?
            .with_guessed_format()?
            .decode()?;
        let (width, height) = im.dimensions();
        let levels: u32 = Self::calculate_levels(width, height);

        let parent_dir = image_path.parent();
        if parent_dir.is_none() {
            return Err(TilingError::UnsupportedSourceImage(
                "Could not find parent dir of image".into(),
            ));
        }
        let parent_dir = parent_dir.unwrap();
        let stem = Path::file_stem(image_path);
        if stem.is_none() {
            return Err(TilingError::UnsupportedSourceImage(
                "Could not find base name of image".into(),
            ));
        }
        let stem = stem.unwrap().to_str();
        if stem.is_none() {
            return Err(TilingError::UnsupportedSourceImage(
                "Could not find base name of image".into(),
            ));
        }
        let stem = stem.unwrap();
        let dest_path = parent_dir.join(format!("{}_files", stem));
        let dzi_file_path = parent_dir.join(format!("{}.dzi", stem));

        Ok(Self {
            image: im,
            levels,
            tile_size,
            tile_overlap,
            dest_path,
            dzi_file_path,
        })
    }

    pub fn new_from_rgb(
        rgb_data: &[u8],
        width: u32,
        height: u32,
        tile_size: u32,
        tile_overlap: u32,
        dest_path: PathBuf,
        dzi_file_path: PathBuf,
    ) -> DZIResult<Self> {
        use TilingError::*;
        let rgb = RgbImage::from_raw(width, height, rgb_data.to_vec())
            .ok_or(IncorrectRGBInputDimensions)?;
        let dyn_image = DynamicImage::from(rgb);
        let (width, height) = dyn_image.dimensions();
        let levels = Self::calculate_levels(width, height);
        Ok(Self {
            dest_path,
            dzi_file_path,
            image: dyn_image,
            levels,
            tile_size,
            tile_overlap,
        })
    }

    fn calculate_levels(src_image_width: u32, src_image_height: u32) -> u32 {
        let levels: u32 = (src_image_height.max(src_image_width) as f64).log2().ceil() as u32 + 1;
        return levels;
    }

    /// Create DZI tiles
    pub fn create_tiles(&self) -> DZIResult<(PathBuf, PathBuf)> {
        for l in 0..self.levels {
            self.create_level(l)?;
        }
        let (w, h) = self.image.dimensions();
        let dzi = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Image xmlns="http://schemas.microsoft.com/deepzoom/2008"
    TileSize="{}"
    Overlap="{}"
    Format="jpg">
    <Size Width="{}" Height="{}"/>
</Image>"#,
            self.tile_size, self.tile_overlap, w, h
        );
        let mut f = std::fs::File::create(self.dzi_file_path.as_path())?;
        f.write_all(dzi.as_bytes())?;
        Ok((self.dzi_file_path.clone(), self.dest_path.clone()))
    }

    /// Check if level is valid
    fn check_level(&self, l: u32) -> DZIResult<()> {
        if l >= self.levels {
            return Err(TilingError::UnexpectedError);
        }
        Ok(())
    }

    /// Create tiles for a level
    fn create_level(&self, level: u32) -> DZIResult<()> {
        let p = self.dest_path.join(format!("{}", level));
        std::fs::create_dir_all(&p)?;
        let mut li = self.get_level_image(level)?;
        let (c, r) = self.get_tile_count(level)?;
        for col in 0..c {
            for row in 0..r {
                let (x, y, x2, y2) = self.get_tile_bounds(level, col, row)?;
                let tile_image = li.crop(x, y, x2 - x, y2 - y);
                let tile_path = p.join(format!("{}_{}.jpg", col, row));
                tile_image.save(tile_path)?;
            }
        }
        Ok(())
    }

    /// Get image for a level
    fn get_level_image(&self, level: u32) -> DZIResult<DynamicImage> {
        self.check_level(level)?;
        let (w, h) = self.get_dimensions(level)?;
        Ok(self
            .image
            .resize(w, h, image::imageops::FilterType::Nearest))
    }

    /// Get scale factor at level
    fn get_scale(&self, level: u32) -> DZIResult<f64> {
        self.check_level(level)?;
        Ok(0.5f64.powi((self.levels - 1 - level) as i32))
    }

    /// Get dimensions (width, height) in pixels of image for level
    fn get_dimensions(&self, level: u32) -> DZIResult<(u32, u32)> {
        self.check_level(level)?;
        let s = self.get_scale(level)?;
        let (w, h) = self.image.dimensions();
        let h = (h as f64 * s).ceil() as u32;
        let w = (w as f64 * s).ceil() as u32;
        Ok((w, h))
    }

    /// Get (number of columns, number of rows) for a level
    fn get_tile_count(&self, l: u32) -> DZIResult<(u32, u32)> {
        let (w, h) = self.get_dimensions(l)?;
        let cols = (w as f64 / self.tile_size as f64).ceil() as u32;
        let rows = (h as f64 / self.tile_size as f64).ceil() as u32;
        Ok((cols, rows))
    }

    fn get_tile_bounds(&self, level: u32, col: u32, row: u32) -> DZIResult<(u32, u32, u32, u32)> {
        let offset_x = if col == 0 { 0 } else { self.tile_overlap };
        let offset_y = if row == 0 { 0 } else { self.tile_overlap };
        let x = col * self.tile_size - offset_x;
        let y = row * self.tile_size - offset_y;

        let (lw, lh) = self.get_dimensions(level)?;

        let w = self.tile_size + (if col == 0 { 1 } else { 2 }) * self.tile_overlap;
        let h = self.tile_size + (if row == 0 { 1 } else { 2 }) * self.tile_overlap;

        let w = w.min(lw - x);
        let h = h.min(lh - y);
        Ok((x, y, x + w, y + h))
    }
}

#[cfg(test)]
mod tests {
    use crate::TileCreator;
    use image::open;
    use std::fs::{read, read_dir};
    use std::path::PathBuf;
    use temp_dir::TempDir;

    #[test]
    fn test_info() {
        let path = PathBuf::from(format!("{}/test_data/test.jpg", env!("CARGO_MANIFEST_DIR")));
        let ic = TileCreator::new_from_image_path(path.as_path(), 254, 1);
        assert!(ic.is_ok());
        let ic = ic.unwrap();
        assert_eq!(ic.levels, 14);
        let (w, h) = ic.get_dimensions(ic.levels - 1).unwrap();
        assert_eq!(w, 5184);
        assert_eq!(h, 3456);

        let (w, h) = ic.get_dimensions(1).unwrap();
        assert_eq!(w, 2);
        assert_eq!(h, 1);

        let (c, r) = ic.get_tile_count(13).unwrap();
        assert_eq!(c, 21);
        assert_eq!(r, 14);
    }

    #[test]
    fn test_artifact_creation_from_rgb_bytes() {
        // Arrange
        let test_image_path =
            PathBuf::from(format!("{}/test_data/test.jpg", env!("CARGO_MANIFEST_DIR")));
        let tmp = TempDir::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();
        let dest_tiles_dir = tmp_path.clone().join("test_files");
        let dest_dzi_path = tmp_path.clone().join("test.dzi");

        let test_image = open(test_image_path).unwrap();
        let width = test_image.width();
        let height = test_image.height();
        let test_image_rgb = test_image.into_rgb8().to_vec();

        let tile_creator = TileCreator::new_from_rgb(
            &test_image_rgb,
            width,
            height,
            254,
            1,
            dest_tiles_dir.clone(),
            dest_dzi_path.clone(),
        )
        .unwrap();

        // Act
        tile_creator.create_tiles().unwrap();

        // Assert dzi file is as expected
        let expected_dzi = include_bytes!("../test_data/expected/test.dzi").to_vec();
        let result_dzi = read(&dest_dzi_path).unwrap();
        assert_eq!(
            result_dzi, expected_dzi,
            ".dzi file should be created as expected"
        );

        // Assert all output tiles are as expected
        let expected_artifact_folder = PathBuf::from(format!(
            "{}/test_data/expected/test_files",
            env!("CARGO_MANIFEST_DIR")
        ));
        for item in read_dir(expected_artifact_folder).unwrap() {
            let level_dir = item.unwrap();
            let level_dir_name = level_dir.file_name().into_string().unwrap();
            for child_file in read_dir(level_dir.path()).unwrap() {
                let tile_file = child_file.unwrap();
                let expected_tile_bytes = read(tile_file.path()).unwrap();
                let result_tile_location = dest_tiles_dir
                    .clone()
                    .join(level_dir_name.as_str())
                    .join(tile_file.file_name());
                let result_tile_bytes = read(result_tile_location).unwrap();
                assert_eq!(result_tile_bytes, expected_tile_bytes);
            }
        }
    }
}
