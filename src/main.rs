#![feature(array_zip)]

mod palette;

use palette::Palette;

use std::borrow::Borrow;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use argh::FromArgs;
use image::{imageops, ImageResult, Pixel, Rgba, RgbaImage};

fn files_in_dir<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<PathBuf>> {
    let mut image_list = vec![];
    for entry in fs::read_dir(path.borrow())? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            image_list.push(entry.path())
        }
    }
    Ok(image_list)
}

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// the input file
    #[argh(positional, short = 'i')]
    input: PathBuf,

    /// the input palette directory
    #[argh(option)]
    palette: PathBuf,

    /// the width of the palette items
    #[argh(option)]
    palette_width: u32,

    /// the height of the palette items
    #[argh(option)]
    palette_height: u32,

    /// the output file
    #[argh(option, short = 'o')]
    output: PathBuf,

    /// whether or not to enable dithering
    #[argh(switch, short = 'd')]
    dither: bool,

    /// the scale of the image in relation to the emoji size
    #[argh(option, default = "1.0")]
    scale: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    let files = files_in_dir(args.palette)?;
    let source = image::open(args.input)?;
    let width = (source.width() as f32 * args.scale) as u32;
    let height = (source.height() as f32 * args.scale) as u32;
    let mut source = source.resize_exact(
        width / args.palette_width,
        height / args.palette_height,
        imageops::FilterType::Gaussian,
    );

    let palette = Palette::from(
        files
            .iter()
            .map(|path| {
                let mut color = Rgba([0f32; 4]);
                let image = image::open(path)?.into_rgba32f();
                for pixel in image.pixels() {
                    for (i, val) in pixel.0.iter().enumerate() {
                        color[i] += val
                    }
                }
                let color = color.map(|v| v / (image.width() * image.height()) as f32);
                Ok((path, color))
            })
            .collect::<ImageResult<Vec<_>>>()?,
    );

    // imageops::dither(image, color_map)
    let mut new_image = RgbaImage::new(width, height);

    if args.dither {
        imageops::dither(&mut source.as_mut_rgba8().unwrap(), &palette);
    }

    source
        .into_rgba32f()
        .enumerate_pixels()
        .map(|(x, y, pixel)| (x, y, palette.nearest_color(&pixel)))
        .try_for_each(|(x, y, (path, _color))| -> ImageResult<()> {
            imageops::replace(
                &mut new_image,
                &image::open(path)?.resize(
                    args.palette_width,
                    args.palette_height,
                    imageops::FilterType::Gaussian,
                ),
                x as i64 * args.palette_width as i64,
                y as i64 * args.palette_height as i64,
            );
            Ok(())
        })?;

    new_image.save(args.output)?;

    Ok(())
}
