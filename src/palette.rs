use bincode;
use std::path::PathBuf;

use image::{imageops::ColorMap, Rgba};

// This is based off of perceptual color, but it's results have been ok so far
fn bad_color_distance(pixel_a: &Rgba<f32>, pixel_b: &Rgba<f32>, q: u64, qfactor: f32) -> u64 {
    let c1 = pixel_a;
    let c2 = pixel_b;
    let dc = [c2[0] - c1[0], c2[1] - c1[1], c2[2] - c1[2], c2[3] - c1[3]];
    let r = (c1[0] + c2[0]) / 2.0;
    let dr = (2.0 + (r / 256.0)) * dc[0] * dc[0];
    let dg = 4.0 * dc[1] * dc[1];
    let db = (2.0 + ((255.0 - r) / 256.0)) * dc[2] * dc[2];
    let da = 255.0 - dc[3] / 256.0;
    if qfactor > 0.0 {
        ((dr + dg + db + da) * 1024.0 + ((q as f32) / qfactor)) as u64
    } else {
        ((dr + dg + db + da) * 1024.0) as u64
    }
}

#[derive(PartialEq, Debug)]
pub struct Palette {
    items: Vec<(PathBuf, Rgba<f32>)>,
}

impl bincode::Decode for Palette {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let items: Vec<(PathBuf, [f32; 4])> = bincode::Decode::decode(decoder)?;
        Ok(Palette {
            items: items
                .into_iter()
                .map(|(path, color)| (path, Rgba::<f32>::from(color)))
                .collect(),
        })
    }
}

impl bincode::Encode for Palette {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        let items: Vec<_> = self
            .items
            .iter()
            .map(|(path, rgba)| (path, rgba.0))
            .collect();
        bincode::Encode::encode(&items, encoder)?;
        Ok(())
    }
}

impl Palette {
    pub fn nearest_color(&self, color: &Rgba<f32>) -> (PathBuf, Rgba<f32>) {
        self.items
            .iter()
            .min_by_key(|(_, palette_color)| bad_color_distance(palette_color, color, 0, 64.0))
            .unwrap()
            .clone()
    }
}

impl<'a> From<Vec<(PathBuf, Rgba<f32>)>> for Palette {
    fn from(items: Vec<(PathBuf, Rgba<f32>)>) -> Self {
        Palette { items }
    }
}

fn into_f32(color: &Rgba<u8>) -> Rgba<f32> {
    Rgba::from(color.0.map(|a| (a as f32) / 255.0))
}

fn into_u8(color: &Rgba<f32>) -> Rgba<u8> {
    Rgba::from(color.0.map(|v| (v * 255.0) as u8))
}

impl ColorMap for Palette {
    type Color = Rgba<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        let (index, _) = self
            .items
            .iter()
            .enumerate()
            .min_by_key(|(_, (_, palette_color))| {
                bad_color_distance(palette_color, &into_f32(color), 0, 64.0)
            })
            .unwrap();

        index
    }

    fn map_color(&self, color: &mut Self::Color) {
        *color = into_u8(&self.items[self.index_of(&color)].1)
    }
}
