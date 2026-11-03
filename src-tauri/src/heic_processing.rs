use anyhow::{Result, anyhow};
use heic::{DecoderConfig, PixelLayout};
use image::{DynamicImage, RgbImage};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Returns true when the buffer looks like an HEIF container with an HEVC
/// (HEIC) or generic HEIF brand, regardless of the file extension.
pub fn looks_like_heic(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    let box_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let end = box_len.clamp(12, bytes.len().min(256));
    const BRANDS: &[&[u8; 4]] = &[
        b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"hevm", b"hevs", b"mif1", b"msf1",
    ];
    let is_brand = |b: &[u8]| BRANDS.iter().any(|brand| b == &brand[..]);
    if is_brand(&bytes[8..12]) {
        return true;
    }
    // Compatible brands start after major_brand (4) + minor_version (4).
    let mut off = 16;
    while off + 4 <= end {
        if is_brand(&bytes[off..off + 4]) {
            return true;
        }
        off += 4;
    }
    false
}

pub fn decode_heic(
    bytes: &[u8],
    cancel_token: Option<(Arc<AtomicUsize>, usize)>,
) -> Result<DynamicImage> {
    let check_cancel = || -> Result<()> {
        if let Some((tracker, generation)) = &cancel_token
            && tracker.load(Ordering::SeqCst) != *generation
        {
            return Err(anyhow!("Load cancelled"));
        }
        Ok(())
    };

    check_cancel()?;

    // DecoderConfig::new() uses heic::recommended_backends(): the native OS
    // decoder first (VideoToolbox / Media Foundation / MediaCodec / VA-API)
    // and the pure-Rust decoder as fallback when the native one is missing
    // or rejects the bitstream.
    let output = DecoderConfig::new()
        .decode(bytes, PixelLayout::Rgb8)
        .map_err(|e| anyhow!("HEIC decode failed: {}", e))?;

    check_cancel()?;

    let image = RgbImage::from_raw(output.width, output.height, output.data)
        .ok_or_else(|| anyhow!("HEIC decode produced an invalid pixel buffer"))?;

    Ok(DynamicImage::ImageRgb32F(
        DynamicImage::ImageRgb8(image).to_rgb32f(),
    ))
}
