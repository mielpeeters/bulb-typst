use wasm_minimal_protocol::*;

use bulb::dither::{
    DitherMethod, adjust, custom,
    ordered::{self, OrderedOptions},
    palette::{self, DitherOptions, PaletteMethod},
    presets::Preset,
};
use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Rgba, RgbaImage};

initiate_protocol!();

const HEADER_LEN: usize = 38;

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

fn decode_preset(id: u8) -> Result<Preset, String> {
    match id {
        0 => Ok(Preset::GameBoy),
        1 => Ok(Preset::Nes),
        2 => Ok(Preset::Cga),
        3 => Ok(Preset::Pico8),
        4 => Ok(Preset::Mac),
        5 => Ok(Preset::C64),
        _ => Err(format!("unknown preset: {id}")),
    }
}

fn load_image(bytes: &[u8]) -> Result<DynamicImage, String> {
    image::load_from_memory(bytes).map_err(|e| format!("failed to decode image: {e}"))
}

fn decode_filter(id: u8) -> Result<fast_image_resize::ResizeAlg, String> {
    use fast_image_resize::{FilterType, ResizeAlg};
    match id {
        0 => Ok(ResizeAlg::Nearest),
        1 => Ok(ResizeAlg::Convolution(FilterType::Bilinear)),
        2 => Ok(ResizeAlg::Convolution(FilterType::CatmullRom)),
        3 => Ok(ResizeAlg::Convolution(FilterType::Gaussian)),
        4 => Ok(ResizeAlg::Convolution(FilterType::Lanczos3)),
        _ => Err(format!("unknown resize filter: {id}")),
    }
}

fn resize(
    img: DynamicImage,
    max_size: u32,
    alg: fast_image_resize::ResizeAlg,
) -> Result<DynamicImage, String> {
    use fast_image_resize::images::Image;
    use fast_image_resize::{PixelType, ResizeOptions, Resizer};

    let (w, h) = (img.width(), img.height());
    if max_size == 0 || (w <= max_size && h <= max_size) {
        return Ok(img);
    }
    let (nw, nh) = if w >= h {
        (
            max_size,
            (max_size as f64 * h as f64 / w as f64).round() as u32,
        )
    } else {
        (
            (max_size as f64 * w as f64 / h as f64).round() as u32,
            max_size,
        )
    };

    let rgba = img.into_rgba8();
    let src = Image::from_vec_u8(w, h, rgba.into_raw(), PixelType::U8x4)
        .map_err(|e| format!("failed to build resize source: {e}"))?;
    let mut dst = Image::new(nw, nh, PixelType::U8x4);

    let mut resizer = Resizer::new();
    resizer
        .resize(&src, &mut dst, &ResizeOptions::new().resize_alg(alg))
        .map_err(|e| format!("resize failed: {e}"))?;

    let buf = RgbaImage::from_raw(nw, nh, dst.into_vec())
        .ok_or_else(|| "resize produced unexpected buffer size".to_string())?;
    Ok(DynamicImage::ImageRgba8(buf))
}

fn gray_to_rgba(gray: &GrayImage) -> RgbaImage {
    let (w, h) = gray.dimensions();
    let mut rgba = RgbaImage::new(w, h);
    for (x, y, Luma([l])) in gray.enumerate_pixels() {
        rgba.put_pixel(x, y, Rgba([*l, *l, *l, 255]));
    }
    rgba
}

fn rgba_to_luma(rgba: &RgbaImage) -> GrayImage {
    let (w, h) = rgba.dimensions();
    let mut out = GrayImage::new(w, h);
    for (src, dst) in rgba.pixels().zip(out.pixels_mut()) {
        dst.0[0] = src.0[0];
    }
    out
}

fn encode_png_rgba(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>, String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut buf = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut buf, CompressionType::Uncompressed, FilterType::Sub);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf)
}

fn encode_png_luma(img: &GrayImage) -> Result<Vec<u8>, String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut buf = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut buf, CompressionType::Uncompressed, FilterType::Sub);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf)
}

fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

fn read_i32_le(buf: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

const FIXED_SCALE: f32 = 10_000.0;

fn read_fixed(buf: &[u8], offset: usize) -> f32 {
    read_i32_le(buf, offset) as f32 / FIXED_SCALE
}

// Negative encoded value signals None; valid threshold is >= 0.
fn read_optional_fixed(buf: &[u8], offset: usize) -> Option<f32> {
    let raw = read_i32_le(buf, offset);
    if raw < 0 {
        None
    } else {
        Some(raw as f32 / FIXED_SCALE)
    }
}

/// Unified dither function.
///
/// Header (38 bytes):
///   [0]:      mode (0=bw, 1=rgb, 2=palette)
///   [1]:      dither_method_id
///   [2..6]:   max_size u32 LE (0 = no resize)
///   [6..10]:  param1 u32 LE (rgb: levels, palette: k)
///   [10..14]: param2 u32 LE (palette: n_accent)
///   [14]:     palette_method_id
///   [15]:     flags (bit 0 = linear_light, bit 1 = perceptual_cap, bits 4..7 = resize filter id)
///   All four floats below are i32 LE fixed-point: stored = round(value * FIXED_SCALE).
///   [16..20]: gamma
///   [20..24]: contrast
///   [24..28]: brightness
///   [28..32]: edge_threshold (negative = None, >= 0 = Some)
///   [32]:     palette source (0=extract, 1=preset, 2=custom)
///   [33]:     preset id (0..=5, matches Preset enum order)
///   [34..38]: custom_palette_len u32 LE (number of RGB triples)
///   [38..38+3*N]: custom palette bytes (R,G,B u8 triples), only when palette source = 2
///   [...]:    image bytes
///
/// Returns PNG bytes.
#[wasm_func]
fn dither(args: &[u8]) -> Result<Vec<u8>, String> {
    if args.len() < HEADER_LEN + 1 {
        return Err(format!(
            "input too short: need {HEADER_LEN}-byte header + image data"
        ));
    }

    let mode = args[0];
    let method = decode_method(args[1])?;
    let max_size = read_u32_le(args, 2);
    let flags = args[15];
    let filter = decode_filter((flags >> 4) & 0b0111)?;
    let adjust_opts = adjust::Adjust {
        gamma: read_fixed(args, 16),
        contrast: read_fixed(args, 20),
        brightness: read_fixed(args, 24),
    };
    let edge_threshold = read_optional_fixed(args, 28);

    let palette_source = args[32];
    let custom_palette_len = read_u32_le(args, 34) as usize;
    let custom_palette_bytes = 3 * custom_palette_len;
    let image_offset = HEADER_LEN + custom_palette_bytes;
    if args.len() < image_offset + 1 {
        return Err(format!(
            "input too short: header + {custom_palette_bytes}-byte palette block + image data expected"
        ));
    }

    let img = load_image(&args[image_offset..])?;
    let img = resize(img, max_size, filter)?;

    match mode {
        // BW: grayscale + 2 levels, output as Luma8 PNG
        0 => {
            let gray = img.into_luma8();
            let mut rgba = gray_to_rgba(&gray);
            adjust::apply(&mut rgba, adjust_opts);
            ordered::dither_cpu(
                &mut rgba,
                OrderedOptions {
                    method,
                    levels: 2,
                    edge_threshold,
                },
            );
            let luma = rgba_to_luma(&rgba);
            encode_png_luma(&luma)
        }
        // RGB: configurable levels per channel
        1 => {
            let mut rgba = img.into_rgba8();
            adjust::apply(&mut rgba, adjust_opts);
            let levels = read_u32_le(args, 6);
            ordered::dither_cpu(
                &mut rgba,
                OrderedOptions {
                    method,
                    levels,
                    edge_threshold,
                },
            );
            encode_png_rgba(&rgba)
        }
        // Palette
        2 => {
            let mut rgba = img.into_rgba8();
            adjust::apply(&mut rgba, adjust_opts);
            let pal = match palette_source {
                0 => {
                    let k = read_u32_le(args, 6) as usize;
                    let n_accent = read_u32_le(args, 10) as usize;
                    let pal_method = decode_palette_method(args[14])?;
                    let linear_light = flags & 1 != 0;
                    let perceptual_cap = flags & 2 != 0;
                    palette::extract_palette(
                        &rgba,
                        k,
                        n_accent,
                        10_000,
                        pal_method,
                        linear_light,
                        perceptual_cap,
                    )
                }
                1 => decode_preset(args[33])?.colors(),
                2 => {
                    let bytes = &args[HEADER_LEN..HEADER_LEN + custom_palette_bytes];
                    let triples: Vec<[u8; 3]> =
                        bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                    custom::palette_from_rgb(&triples)
                }
                _ => return Err(format!("unknown palette source: {palette_source}")),
            };
            if pal.len() < 2 {
                return Err(format!(
                    "palette too small ({} colours, need >= 2)",
                    pal.len()
                ));
            }
            let _ = palette::dither_palette(
                &mut rgba,
                &pal,
                DitherOptions {
                    method,
                    edge_threshold,
                },
            );
            encode_png_rgba(&rgba)
        }
        _ => Err(format!("unknown mode: {mode}")),
    }
}
