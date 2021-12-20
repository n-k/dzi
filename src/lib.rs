use std::path::{Path, PathBuf};

use anyhow::bail;
use image::{DynamicImage, GenericImageView};

/// A tile creator, this struct and associated functions
/// implement the DZI tiler
pub struct TileCreator {
    /// path of destination directory where tiles will be stored
    dest_path: PathBuf,
    /// source image
    image: DynamicImage,
    /// size of individual tiles in pixels
    tile_size: u32,
    /// number of pixels neighboring tiles overlap
    tile_overlap: u32,
    /// total number of levels of tiles
    levels: u32,
}

impl TileCreator {
    pub fn new_from_image_path(image_path: &Path) -> anyhow::Result<Self> {
        let im = image::open(image_path)?;
        let (h, w) = im.dimensions();
        let levels: u32 = (h.max(w) as f64).log2().ceil() as u32 + 1;
        Ok(Self {
            dest_path: image_path.join("_files"),
            image: im,
            tile_size: 254,
            tile_overlap: 1,
            levels,
        })
    }

    #[cfg(feature = "async-tokio")]
    /// Create DZI tiles
    pub async fn create_tiles(&self) -> anyhow::Result<()> {
        for l in 0..self.levels {
            self.create_level(l).await?;
        }
        Ok(())
    }

    #[cfg(not(feature = "async-tokio"))]
    /// Create DZI tiles
    pub fn create_tiles(&self) -> anyhow::Result<()> {
        for l in 0..self.levels {
            self.create_level(l)?;
        }
        Ok(())
    }

    /// Check if level is valid
    fn check_level(&self, l: u32) -> anyhow::Result<()> {
        if l >= self.levels {
            bail!("invalid level");
        }
        Ok(())
    }

    #[cfg(feature = "async-tokio")]
    /// Create tiles for a level
    async fn create_level(&self, level: u32) -> anyhow::Result<()> {
        let p = self.dest_path
            .join(format!("{}", level));
        std::fs::create_dir_all(p.clone())?;
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

    #[cfg(not(feature = "async-tokio"))]
    /// Create tiles for a level
    fn create_level(&self, level: u32) -> anyhow::Result<()> {
        let p = self.dest_path
            .join(format!("{}", level));
        std::fs::create_dir_all(p.clone())?;
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
    fn get_level_image(&self, level: u32) -> anyhow::Result<DynamicImage> {
        self.check_level(level)?;
        let (w, h) = self.get_dimensions(level)?;
        Ok(self.image
            .resize(
                w,
                h,
                image::imageops::FilterType::Nearest,
            )
        )
    }

    /// Get scale factor at level
    fn get_scale(&self, level: u32) -> anyhow::Result<f64> {
        self.check_level(level)?;
        Ok(0.5f64.powi((self.levels - 1 - level) as i32))
    }

    /// Get dimensions (width, height) in pixels of image for level
    fn get_dimensions(&self, level: u32) -> anyhow::Result<(u32, u32)> {
        self.check_level(level)?;
        let s = self.get_scale(level)?;
        let (w, h) = self.image.dimensions();
        let h = (h as f64 * s).ceil() as u32;
        let w = (w as f64 * s).ceil() as u32;
        Ok((w, h))
    }

    /// Get (number of columns, number of rows) for a level
    fn get_tile_count(&self, l: u32) -> anyhow::Result<(u32, u32)> {
        let (w, h) = self.get_dimensions(l)?;
        let cols = (w as f64 / self.tile_size as f64).ceil() as u32;
        let rows = (h as f64 / self.tile_size as f64).ceil() as u32;
        Ok((cols, rows))
    }

    fn get_tile_bounds(
        &self,
        level: u32,
        col: u32,
        row: u32,
    ) -> anyhow::Result<(u32, u32, u32, u32)> {
        let offset_x = if col == 0 {
            0
        } else {
            self.tile_overlap
        };
        let offset_y = if row == 0 {
            0
        } else {
            self.tile_overlap
        };
        let x = col * self.tile_size - offset_x;
        let y = row * self.tile_size - offset_y;

        let (lw, lh) = self.get_dimensions(level)?;

        let w = self.tile_size +
            (if col == 0 { 1 } else { 2 }) * self.tile_overlap;
        let h = self.tile_size +
            (if row == 0 { 1 } else { 2 }) * self.tile_overlap;

        let w = w.min(lw - x);
        let h = h.min(lh - y);
        Ok((x, y, x + w, y + h))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::TileCreator;

    #[test]
    fn test_info() {
        let path = PathBuf::from(format!("{}/test_data/test.jpg", env!("CARGO_MANIFEST_DIR")));
        let ic = TileCreator::new_from_image_path(path.as_path());
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

        ic.create_tiles().unwrap();
    }
}
