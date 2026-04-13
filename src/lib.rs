use wasm_minimal_protocol::*;

use bulb::dither::{
    ordered,
    palette::{self, PaletteMethod},
    DitherMethod,
};
use image::{DynamicImage, ImageBuffer, Luma, Rgba, RgbaImage};

initiate_protocol!();

const HEADER_LEN: usize = 16;

fn decode_method(id: u8) -> Result<DitherMethod, String> {
    match id {
        0 => Ok(DitherMethod::Bayer2x2),
        1 => Ok(DitherMethod::Bayer4x4),
        2 => Ok(DitherMethod::Bayer8x8),
        3 => Ok(DitherMethod::Cluster4),
        4 => Ok(DitherMethod::Cluster6),
        5 => Ok(DitherMethod::Cluster8),
        6 => Ok(DitherMethod::Noise),
        _ => Err(format!("unknown dither method: {id}")),
    }
}

fn decode_palette_method(id: u8) -> Result<PaletteMethod, String> {
    match id {
        0 => Ok(PaletteMethod::Hybrid),
        1 => Ok(PaletteMethod::Fps),
        2 => Ok(PaletteMethod::Kmeans),
        _ => Err(format!("unknown palette method: {id}")),
    }
}

fn load_image(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).map_err(|e| format!("failed to decode image: {e}"))
}

fn resize(img: DynamicImage, max_size: u32) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if max_size == 0 || (w <= max_size && h <= max_size) {
        return img;
    }
    let (nw, nh) = if w >= h {
        (max_size, (max_size as f64 * h as f64 / w as f64).round() as u32)
    } else {
        ((max_size as f64 * w as f64 / h as f64).round() as u32, max_size)
    };
    img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
}

fn to_grayscale_rgba(img: &DynamicImage) -> RgbaImage {
    let gray = img.to_luma8();
    let (w, h) = gray.dimensions();
    let mut rgba = RgbaImage::new(w, h);
    for (x, y, Luma([l])) in gray.enumerate_pixels() {
        rgba.put_pixel(x, y, Rgba([*l, *l, *l, 255]));
    }
    rgba
}

fn encode_png(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>, String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut buf = Vec::new();
    let encoder = PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::Sub);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf)
}

fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

/// Unified dither function.
///
/// Header (16 bytes):
///   [0]:     mode (0=bw, 1=rgb, 2=palette)
///   [1]:     dither_method_id
///   [2..6]:  max_size u32 LE (0 = no resize)
///   [6..10]: param1 u32 LE (rgb: levels, palette: k)
///   [10..14]: param2 u32 LE (palette: n_accent)
///   [14]:    palette_method_id
///   [15]:    flags (bit 0 = linear_light, bit 1 = perceptual_cap)
///   [16..]:  image bytes
///
/// Returns PNG bytes.
#[wasm_func]
fn dither(args: &[u8]) -> Result<Vec<u8>, String> {
    if args.len() < HEADER_LEN + 1 {
        return Err(format!("input too short: need {HEADER_LEN}-byte header + image data"));
    }

    let mode = args[0];
    let method = decode_method(args[1])?;
    let max_size = read_u32_le(args, 2);

    let img = load_image(&args[HEADER_LEN..])?;
    let img = resize(img, max_size);

    let mut rgba = match mode {
        0 => to_grayscale_rgba(&img),
        _ => img.to_rgba8(),
    };

    match mode {
        // BW: grayscale + 2 levels
        0 => ordered::dither_cpu(&mut rgba, method, 2),
        // RGB: configurable levels per channel
        1 => {
            let levels = read_u32_le(args, 6);
            ordered::dither_cpu(&mut rgba, method, levels);
        }
        // Palette
        2 => {
            let k = read_u32_le(args, 6) as usize;
            let n_accent = read_u32_le(args, 10) as usize;
            let pal_method = decode_palette_method(args[14])?;
            let flags = args[15];
            let linear_light = flags & 1 != 0;
            let perceptual_cap = flags & 2 != 0;

            let pal = palette::extract_palette(
                &rgba, k, n_accent, 10_000, pal_method, linear_light, perceptual_cap,
            );
            if pal.len() < 2 {
                return Err(format!(
                    "extracted palette too small ({} colours, need >= 2)",
                    pal.len()
                ));
            }
            palette::dither_palette(&mut rgba, &pal, method);
        }
        _ => return Err(format!("unknown mode: {mode}")),
    }

    encode_png(&rgba)
}
