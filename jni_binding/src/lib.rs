use std::slice;
use jni::JNIEnv;
use jni::objects::{JClass, JObject, JByteArray};
use jni::sys::{jbyteArray, jint, jboolean, JNI_TRUE, JNI_FALSE};
use fast_image_resize::{Resizer, ResizeAlg, PixelType, FilterType};
use fast_image_resize::images::{Image, ImageRef};

const ANDROID_BITMAP_FORMAT_RGBA_8888: i32 = 1;

// Helper to map integer to ResizeAlg
fn get_resize_alg(alg: jint) -> ResizeAlg {
    match alg {
        0 => ResizeAlg::Nearest,
        1 => ResizeAlg::Convolution(FilterType::Box),
        2 => ResizeAlg::Convolution(FilterType::Bilinear),
        3 => ResizeAlg::Convolution(FilterType::CatmullRom),
        4 => ResizeAlg::Convolution(FilterType::Lanczos3),
        _ => ResizeAlg::Convolution(FilterType::Bilinear),
    }
}

/// 1. ByteArray Resize: Resizes raw RGBA bytes.
/// Kotlin: external fun resizeRgba(src: ByteArray, srcW: Int, srcH: Int, dstW: Int, dstH: Int, alg: Int): ByteArray
#[no_mangle]
pub unsafe extern "system" fn Java_io_github_fastimage_FastImageResizer_resizeRgba(
    env: JNIEnv,
    _class: JClass,
    src_array: JByteArray,
    src_width: jint,
    src_height: jint,
    dst_width: jint,
    dst_height: jint,
    alg: jint,
) -> jbyteArray {
    let src_len = env.get_array_length(&src_array).unwrap_or(0) as usize;
    let mut src_vec = vec![0u8; src_len];
    let src_slice = slice::from_raw_parts_mut(src_vec.as_mut_ptr() as *mut i8, src_len);
    if env.get_byte_array_region(&src_array, 0, src_slice).is_err() {
        return std::ptr::null_mut();
    }

    // Wrap source bytes in ImageRef
    let src_image = match ImageRef::new(
        src_width as u32,
        src_height as u32,
        &src_vec,
        PixelType::U8x4, // RGBA_8888 is U8x4
    ) {
        Ok(view) => view,
        Err(_) => return std::ptr::null_mut(),
    };

    // Prepare destination buffer
    let dst_len = (dst_width * dst_height * 4) as usize;
    let mut dst_vec = vec![0u8; dst_len];
    
    let mut dst_image = match Image::from_slice_u8(
        dst_width as u32,
        dst_height as u32,
        &mut dst_vec,
        PixelType::U8x4,
    ) {
        Ok(view) => view,
        Err(_) => return std::ptr::null_mut(),
    };

    // Perform Resize
    let mut resizer = Resizer::new();
    let options = fast_image_resize::ResizeOptions::new().resize_alg(get_resize_alg(alg));
    if resizer.resize(&src_image, &mut dst_image, Some(&options)).is_err() {
        return std::ptr::null_mut();
    }

    // Convert result vector back to jbyteArray
    let result_array = match env.new_byte_array(dst_len as jint) {
        Ok(arr) => arr,
        Err(_) => return std::ptr::null_mut(),
    };

    let dst_slice = slice::from_raw_parts(dst_vec.as_ptr() as *const i8, dst_len);
    if env.set_byte_array_region(&result_array, 0, dst_slice).is_err() {
        return std::ptr::null_mut();
    }

    result_array.into_raw()
}

/// 2. Direct Bitmap Resize: Resizes directly in Bitmap pixel buffers (Zero-copy).
/// Requires ndk-sys. Lock source and destination, resize, and unlock.
/// Kotlin: external fun resizeBitmap(src: Bitmap, dst: Bitmap, alg: Int): Boolean
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "system" fn Java_io_github_fastimage_FastImageResizer_resizeBitmap(
    env: JNIEnv,
    _class: JClass,
    src_bitmap: JObject,
    dst_bitmap: JObject,
    alg: jint,
) -> jboolean {
    let raw_env = env.get_native_interface();
    let raw_src = src_bitmap.as_raw();
    let raw_dst = dst_bitmap.as_raw();

    // Get Bitmap Info
    let mut src_info = ndk_sys::AndroidBitmapInfo {
        width: 0,
        height: 0,
        stride: 0,
        format: 0,
        flags: 0,
    };
    let mut dst_info = ndk_sys::AndroidBitmapInfo {
        width: 0,
        height: 0,
        stride: 0,
        format: 0,
        flags: 0,
    };

    if ndk_sys::AndroidBitmap_getInfo(raw_env, raw_src, &mut src_info) < 0 {
        return JNI_FALSE;
    }
    if ndk_sys::AndroidBitmap_getInfo(raw_env, raw_dst, &mut dst_info) < 0 {
        return JNI_FALSE;
    }

    // Ensure they are both RGBA_8888 (Fast Image Resize uses PixelType::U8x4)
    if src_info.format != ANDROID_BITMAP_FORMAT_RGBA_8888 
        || dst_info.format != ANDROID_BITMAP_FORMAT_RGBA_8888 {
        return JNI_FALSE;
    }

    // Lock Pixels
    let mut src_pixels: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut dst_pixels: *mut std::ffi::c_void = std::ptr::null_mut();

    if ndk_sys::AndroidBitmap_lockPixels(raw_env, raw_src, &mut src_pixels) < 0 {
        return JNI_FALSE;
    }
    if ndk_sys::AndroidBitmap_lockPixels(raw_env, raw_dst, &mut dst_pixels) < 0 {
        ndk_sys::AndroidBitmap_unlockPixels(raw_env, raw_src);
        return JNI_FALSE;
    }

    // Create slice views from locked pointers
    let src_slice = slice::from_raw_parts(
        src_pixels as *const u8,
        (src_info.stride * src_info.height) as usize,
    );
    let dst_slice = slice::from_raw_parts_mut(
        dst_pixels as *mut u8,
        (dst_info.stride * dst_info.height) as usize,
    );

    // Resize using fast_image_resize
    let res = (|| {
        let src_image = ImageRef::new(
            src_info.width,
            src_info.height,
            src_slice,
            PixelType::U8x4,
        ).map_err(|_| "Invalid source buffer")?;
        let mut dst_image = Image::from_slice_u8(
            dst_info.width,
            dst_info.height,
            dst_slice,
            PixelType::U8x4,
        ).map_err(|_| "Invalid destination buffer")?;
        let mut resizer = Resizer::new();
        let options = fast_image_resize::ResizeOptions::new().resize_alg(get_resize_alg(alg));
        resizer.resize(&src_image, &mut dst_image, Some(&options)).map_err(|_| "Resize failed")?;
        Ok::<(), &'static str>(())
    })();

    // Always unlock pixels
    ndk_sys::AndroidBitmap_unlockPixels(raw_env, raw_src);
    ndk_sys::AndroidBitmap_unlockPixels(raw_env, raw_dst);

    if res.is_ok() {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// Fallback for non-android targets (to compile/test on host)
#[cfg(not(target_os = "android"))]
#[no_mangle]
pub unsafe extern "system" fn Java_io_github_fastimage_FastImageResizer_resizeBitmap(
    _env: JNIEnv,
    _class: JClass,
    _src_bitmap: JObject,
    _dst_bitmap: JObject,
    _alg: jint,
) -> jboolean {
    JNI_FALSE
}
