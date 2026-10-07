use std::io::Cursor;

use bulb_dither::{
    Method,
    adjust::{self, Adjust},
    diffusion::Kernel,
    ordered::Matrix,
    palette::{self, ExtractOptions, Preset},
    quantize::{KnollLut, Levels},
};
use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Rgba, RgbaImage};
use minicbor::data::Type;
use minicbor::{Decode, Decoder, decode::Error};
use wasm_minimal_protocol::*;
use zune_png::PngDecoder;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::zune_core::options::DecoderOptions;

initiate_protocol!();

/// Options sent by `typst/bulb.typ` as a CBOR array, in field order.
#[derive(Decode)]
struct Options {
    #[n(0)]
    colors: Colors,
    #[n(1)]
    levels: u32,
    #[cbor(n(2), decode_with = "decode_method")]
    method: Method,
    #[cbor(n(3), decode_with = "decode_filter")]
    filter: ResizeAlg,
    #[n(4)]
    size: Option<u32>,
    #[n(5)]
    transparent: bool,
    #[n(6)]
    gamma: f64,
    #[n(7)]
    contrast: f64,
    #[n(8)]
    brightness: f64,
    #[n(9)]
    hull_weight: f64,
}

/// The `colors` argument of `dither` in `typst/bulb.typ`.
enum Colors {
    Bw,
    Rgb,
    Extract(usize),
    Preset(Preset),
    Custom(Vec<[u8; 3]>),
}

impl<'b, C> Decode<'b, C> for Colors {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, Error> {
        match d.datatype()? {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => Ok(Self::Extract(d.u32()? as usize)),
            Type::String => Ok(match d.str()? {
                "bw" => Self::Bw,
                "rgb" => Self::Rgb,
                "gameboy" => Self::Preset(Preset::GameBoy),
                "nes" => Self::Preset(Preset::Nes),
                "cga" => Self::Preset(Preset::Cga),
                "pico8" => Self::Preset(Preset::Pico8),
                "mac" => Self::Preset(Preset::Mac),
                "c64" => Self::Preset(Preset::C64),
                other => {
                    return Err(Error::message(format!(
                        "unknown colors: {other:?}, expected \"bw\", \"rgb\" or a preset (\"gameboy\", \"nes\", \"cga\", \"pico8\", \"mac\", \"c64\")"
                    )));
                }
            }),
            Type::Array | Type::ArrayIndef => d.decode_with(ctx).map(Self::Custom),
            t => {
                Err(Error::type_mismatch(t)
                    .with_message("colors must be a string, integer or array"))
            }
        }
    }
}

fn decode_method<Ctx>(d: &mut Decoder<'_>, _: &mut Ctx) -> Result<Method, Error> {
    Ok(match d.str()? {
        "bayer2" | "bayer2x2" => Method::Ordered(Matrix::Bayer2),
        "bayer4" | "bayer4x4" => Method::Ordered(Matrix::Bayer4),
        "bayer8" | "bayer8x8" => Method::Ordered(Matrix::Bayer8),
        "cluster4" => Method::Ordered(Matrix::Cluster4),
        "cluster6" => Method::Ordered(Matrix::Cluster6),
        "cluster8" => Method::Ordered(Matrix::Cluster8),
        "floyd-steinberg" | "floyd" => Method::Diffusion(Kernel::FloydSteinberg),
        "atkinson" => Method::Diffusion(Kernel::Atkinson),
        "jarvis" => Method::Diffusion(Kernel::Jarvis),
        "stucki" => Method::Diffusion(Kernel::Stucki),
        "burkes" => Method::Diffusion(Kernel::Burkes),
        "sierra" => Method::Diffusion(Kernel::Sierra),
        "sierra-two-row" => Method::Diffusion(Kernel::SierraTwoRow),
        "sierra-lite" => Method::Diffusion(Kernel::SierraLite),
        "simple" => Method::Diffusion(Kernel::Simple),
        other => return Err(Error::message(format!("unknown method: {other:?}"))),
    })
}

fn decode_filter<Ctx>(d: &mut Decoder<'_>, _: &mut Ctx) -> Result<ResizeAlg, Error> {
    Ok(match d.str()? {
        "nearest" => ResizeAlg::Nearest,
        "triangle" => ResizeAlg::Convolution(FilterType::Bilinear),
        "catmull-rom" => ResizeAlg::Convolution(FilterType::CatmullRom),
        "gaussian" => ResizeAlg::Convolution(FilterType::Gaussian),
        "lanczos3" => ResizeAlg::Convolution(FilterType::Lanczos3),
        other => return Err(Error::message(format!("unknown filter: {other:?}"))),
    })
}

fn load_image(bytes: &[u8]) -> Result<DynamicImage, String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return decode_png_fast(bytes);
    }
    image::load_from_memory(bytes).map_err(|e| format!("failed to decode image: {e}"))
}

fn decode_png_fast(bytes: &[u8]) -> Result<DynamicImage, String> {
    let opts = DecoderOptions::new_fast()
        .png_set_strip_to_8bit(true)
        .png_set_add_alpha_channel(true);
    let mut dec = PngDecoder::new_with_options(Cursor::new(bytes), opts);
    let pixels = dec
        .decode_raw()
        .map_err(|e| format!("png decode failed: {e:?}"))?;
    let (w, h) = dec
        .dimensions()
        .ok_or_else(|| "png missing dimensions".to_string())?;
    let cs = dec
        .colorspace()
        .ok_or_else(|| "png missing colorspace".to_string())?;
    let rgba = match cs {
        ColorSpace::RGBA => pixels,
        ColorSpace::LumaA => expand_luma_a_to_rgba(&pixels),
        other => return Err(format!("unsupported png colorspace: {other:?}")),
    };
    let buf = RgbaImage::from_raw(w as u32, h as u32, rgba)
        .ok_or_else(|| "png buffer length mismatch".to_string())?;
    Ok(DynamicImage::ImageRgba8(buf))
}

fn expand_luma_a_to_rgba(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 2);
    for chunk in src.as_chunks::<2>().0 {
        let l = chunk[0];
        let a = chunk[1];
        out.extend_from_slice(&[l, l, l, a]);
    }
    out
}

fn resize(
    img: DynamicImage,
    max_size: Option<u32>,
    alg: ResizeAlg,
) -> Result<DynamicImage, String> {
    let (w, h) = (img.width(), img.height());
    let Some(max_size) = max_size.filter(|&m| w > m || h > m) else {
        return Ok(img);
    };
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

fn gray_to_rgba(gray: &GrayImage, src: &Option<RgbaImage>) -> RgbaImage {
    let (w, h) = gray.dimensions();
    let mut rgba = RgbaImage::new(w, h);
    for (x, y, Luma([l])) in gray.enumerate_pixels() {
        let alpha = src.as_ref().map_or(255, |s| s.get_pixel(x, y).0[3]);
        rgba.put_pixel(x, y, Rgba([*l, *l, *l, alpha]));
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
    let mut buf = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut buf, CompressionType::Uncompressed, PngFilterType::Sub);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf)
}

fn encode_png_luma(img: &GrayImage) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut buf, CompressionType::Uncompressed, PngFilterType::Sub);
    img.write_with_encoder(encoder)
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf)
}

fn dither_palette(
    mut rgba: RgbaImage,
    pal: &[[f32; 3]],
    method: Method,
    dither_alpha: bool,
) -> Result<Vec<u8>, String> {
    if pal.len() < 2 {
        return Err(format!(
            "palette too small ({} colours, need >= 2)",
            pal.len()
        ));
    }
    match method {
        // Knoll mixes more than two colours per pattern, which only
        // ordered dithering can use.
        Method::Ordered(_) => {
            method.dither(&mut rgba, &KnollLut::<16>::new(pal), dither_alpha);
        }
        Method::Diffusion(_) => {
            method.dither(&mut rgba, pal, dither_alpha);
        }
    }
    encode_png_rgba(&rgba)
}

/// Dither `image` (PNG or JPEG bytes) with the CBOR-encoded [`Options`].
///
/// Returns PNG bytes.
#[wasm_func]
fn dither(options: &[u8], image: &[u8]) -> Result<Vec<u8>, String> {
    let opts: Options = minicbor::decode(options).map_err(|e| format!("invalid options: {e}"))?;
    let method = opts.method;
    let dither_alpha = opts.transparent;
    let adjust_opts = Adjust {
        gamma: opts.gamma as f32,
        contrast: opts.contrast as f32,
        brightness: opts.brightness as f32,
    };

    let img = resize(load_image(image)?, opts.size, opts.filter)?;

    // Grayscale keeps its own path to output a Luma8 PNG when alpha is dropped.
    if let Colors::Bw = opts.colors {
        let src = dither_alpha.then(|| img.to_rgba8());
        let mut rgba = gray_to_rgba(&img.into_luma8(), &src);
        adjust::apply(&mut rgba, adjust_opts);
        method.dither(&mut rgba, &Levels::new(2), dither_alpha);
        return if dither_alpha {
            encode_png_rgba(&rgba)
        } else {
            encode_png_luma(&rgba_to_luma(&rgba))
        };
    }

    let mut rgba = img.into_rgba8();
    adjust::apply(&mut rgba, adjust_opts);
    match opts.colors {
        Colors::Bw => unreachable!(),
        Colors::Rgb => {
            method.dither(&mut rgba, &Levels::new(opts.levels), dither_alpha);
            encode_png_rgba(&rgba)
        }
        Colors::Extract(colours) => {
            let extract_opts = ExtractOptions {
                colours,
                hull_weight: opts.hull_weight as f32,
                fast: true,
            };
            let pal = palette::extract(&rgba, &extract_opts);
            dither_palette(rgba, &pal, method, dither_alpha)
        }
        Colors::Preset(preset) => dither_palette(rgba, &preset.colours(), method, dither_alpha),
        Colors::Custom(rgb) => dither_palette(rgba, &palette::from_rgb(&rgb), method, dither_alpha),
    }
}
