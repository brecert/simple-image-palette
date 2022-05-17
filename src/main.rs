#![feature(array_zip)]

mod palette;

use bincode::{decode_from_std_read, encode_into_std_write, Decode};
use palette::Palette;

use std::borrow::Borrow;
use std::error::Error;
use std::fs;
use std::io::stdout;
use std::path::{Path, PathBuf};

use argh::FromArgs;
use image::{imageops, GenericImageView, ImageResult, Pixel, Rgba, RgbaImage};

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

    /// the input palette path
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

    /// the cache for palettes path
    #[argh(option)]
    cache: Option<PathBuf>,

    /// the scale of the image in relation to the emoji size
    #[argh(option, default = "1.0")]
    scale: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    let source = image::open(args.input)?;
    let width = (source.width() as f32 * args.scale) as u32;
    let height = (source.height() as f32 * args.scale) as u32;
    let mut source = source.resize_exact(
        width / args.palette_width,
        height / args.palette_height,
        imageops::FilterType::Gaussian,
    );

    let palette_path = args.palette.as_path();
    let palette = match args.cache {
        Some(ref path) => {
            let mut file = fs::File::open(path)?;
            decode_from_std_read(&mut file, bincode::config::standard())?
        }
        None => {
            let files = files_in_dir(palette_path)?;
            let len = files.len() as f64;
            Palette::from(
                files
                    .iter()
                    .enumerate()
                    .map(|(i, path)| {
                        println!("generating palette: {:.2}%", (i as f64 / len) * 100f64);
                        let mut color = Rgba([0f32; 4]);
                        let image = image::open(path)?.into_rgba32f();
                        for pixel in image.pixels() {
                            for (i, val) in pixel.0.iter().enumerate() {
                                color[i] += val
                            }
                        }
                        let color = color.map(|v| v / (image.width() * image.height()) as f32);
                        Ok((path.clone(), color))
                    })
                    .collect::<ImageResult<Vec<_>>>()?,
            )
        }
    };

    if let Some(path) = args.cache {
        let mut file = fs::File::create(&path)?;
        encode_into_std_write(&palette, &mut file, bincode::config::standard())?;
    }

    // imageops::dither(image, color_map)
    let mut new_image = RgbaImage::new(width, height);

    if args.dither {
        imageops::dither(&mut source.as_mut_rgba8().unwrap(), &palette);
    }

    let width = source.width();
    let len = width * source.height();

    source
        .into_rgba32f()
        .enumerate_pixels()
        .map(|(x, y, pixel)| (x, y, palette.nearest_color(&pixel)))
        .try_for_each(|(x, y, (path, _color))| -> ImageResult<()> {
            println!(
                "mapping pixels: {:.2}%",
                ((y * width + x) as f64 / len as f64) * 100f64
            );
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
