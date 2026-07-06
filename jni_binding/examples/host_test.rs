use std::fs::File;
use std::io::{BufReader, Write};
use fast_image_resize::{Resizer, ResizeAlg, PixelType, FilterType, ImageView};
use fast_image_resize::images::{Image, ImageRef};
use jpeg_decoder::Decoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading test_image.jpg...");
    let file = File::open("test_image.jpg")?;
    let mut decoder = Decoder::new(BufReader::new(file));
    let rgb_pixels = decoder.decode()?;
    let info = decoder.info().ok_or("Failed to get image info")?;
    let width = info.width as u32;
    let height = info.height as u32;

    println!("Loaded image: {}x{} (RGB)", width, height);

    // Convert RGB to RGBA since our Android JNI bindings process RGBA_8888 (U8x4)
    let mut rgba_pixels = Vec::with_capacity((width * height * 4) as usize);
    for chunk in rgb_pixels.chunks_exact(3) {
        rgba_pixels.push(chunk[0]); // R
        rgba_pixels.push(chunk[1]); // G
        rgba_pixels.push(chunk[2]); // B
        rgba_pixels.push(255);      // A
    }

    // 1. Test Resizing (resizeByWidth equivalent)
    println!("Testing Resize to width 300 (Lanczos3)...");
    let target_width = 300u32;
    let target_height = (height as u64 * target_width as u64 / width as u64) as u32;

    let src_image = ImageRef::new(width, height, &rgba_pixels, PixelType::U8x4)?;
    let mut dst_vec = vec![0u8; (target_width * target_height * 4) as usize];
    let mut dst_image = Image::from_slice_u8(target_width, target_height, &mut dst_vec, PixelType::U8x4)?;

    let mut resizer = Resizer::new();
    let options = fast_image_resize::ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer.resize(&src_image, &mut dst_image, Some(&options))?;
    println!("Resize completed: {}x{}", target_width, target_height);

    // 2. Test Splitting (splitByHeight equivalent)
    println!("Testing Split into 2 parts by height...");
    let src_typed = src_image.typed_image::<fast_image_resize::pixels::U8x4>().ok_or("Invalid type")?;
    let height_nz = core::num::NonZeroU32::new(height).ok_or("Height is zero")?;
    let parts_nz = core::num::NonZeroU32::new(2).ok_or("Parts is zero")?;
    let parts = src_typed.split_by_height(0, height_nz, parts_nz).ok_or("Split failed")?;
    println!("Split completed. Part 1 height: {}, Part 2 height: {}", parts[0].height(), parts[1].height());

    // 3. Test WebP Compression (compress format WEBP equivalent)
    println!("Testing WebP compression (quality 80)...");
    use zenwebp::{EncodeRequest, LossyConfig, PixelLayout};
    let mut webp_config = LossyConfig::new();
    webp_config.quality = 80.0;
    webp_config.method = 4;

    let req = EncodeRequest::lossy(&webp_config, &rgba_pixels, PixelLayout::Rgba8, width, height);
    let webp_bytes = req.encode().map_err(|_| "WebP encoding failed")?;

    let mut webp_file = File::create("output_test.webp")?;
    webp_file.write_all(&webp_bytes)?;
    println!("WebP compression completed. Saved to output_test.webp ({} bytes)", webp_bytes.len());

    // 4. Test JPEG Compression (compress format JPEG equivalent)
    println!("Testing JPEG compression (quality 85)...");
    use jpeg_encoder::{Encoder, ColorType};
    let mut jpeg_buffer = Vec::new();
    {
        let encoder = Encoder::new(&mut jpeg_buffer, 85);
        encoder.encode(&rgb_pixels, width as u16, height as u16, ColorType::Rgb)
            .map_err(|_| "JPEG encoding failed")?;
    }
    
    let mut jpeg_file = File::create("output_test.jpg")?;
    jpeg_file.write_all(&jpeg_buffer)?;
    println!("JPEG compression completed. Saved to output_test.jpg ({} bytes)", jpeg_buffer.len());

    println!("All tests passed successfully!");
    Ok(())
}
