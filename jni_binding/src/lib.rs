use std::slice;
use jni::JNIEnv;
use jni::objects::{JClass, JObject};
use jni::sys::{jint, jboolean, JNI_TRUE, JNI_FALSE};
use fast_image_resize::{Resizer, ResizeAlg, PixelType, FilterType, ImageView};
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

/// 1. Direct Bitmap Resize: Resizes directly in Bitmap pixel buffers (Zero-copy).
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

/// 2. Split Bitmap by height: Splits a Bitmap into multiple Bitmaps by height (zero-copy).
/// Kotlin: external fun splitBitmap(srcBitmap: Bitmap, numParts: Int): Array<Bitmap>?
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "system" fn Java_io_github_fastimage_FastImageResizer_splitBitmap(
    mut env: JNIEnv,
    _class: JClass,
    src_bitmap: JObject,
    num_parts: jint,
) -> jni::sys::jobjectArray {
    if num_parts <= 0 {
        return std::ptr::null_mut();
    }

    let raw_env = env.get_native_interface();
    let raw_src = src_bitmap.as_raw();

    // Get Bitmap Info
    let mut src_info = ndk_sys::AndroidBitmapInfo {
        width: 0,
        height: 0,
        stride: 0,
        format: 0,
        flags: 0,
    };
    if ndk_sys::AndroidBitmap_getInfo(raw_env, raw_src, &mut src_info) < 0 {
        return std::ptr::null_mut();
    }

    if src_info.format != ANDROID_BITMAP_FORMAT_RGBA_8888 {
        return std::ptr::null_mut();
    }

    // Lock Src Pixels
    let mut src_pixels: *mut std::ffi::c_void = std::ptr::null_mut();
    if ndk_sys::AndroidBitmap_lockPixels(raw_env, raw_src, &mut src_pixels) < 0 {
        return std::ptr::null_mut();
    }

    let src_slice = slice::from_raw_parts(
        src_pixels as *const u8,
        (src_info.stride * src_info.height) as usize,
    );

    // Call inner split logic
    let res = (|| {
        let src_image = ImageRef::new(
            src_info.width,
            src_info.height,
            src_slice,
            PixelType::U8x4,
        ).map_err(|_| "Invalid source buffer")?;

        let src_typed = src_image.typed_image::<fast_image_resize::pixels::U8x4>().ok_or("Invalid type")?;
        
        let height_nz = core::num::NonZeroU32::new(src_info.height).ok_or("Height is zero")?;
        let parts_nz = core::num::NonZeroU32::new(num_parts as u32).ok_or("Parts is zero")?;
        
        let parts = src_typed.split_by_height(0, height_nz, parts_nz).ok_or("Split failed")?;

        // Get Config class and ARGB_8888 object
        let config_class = env.find_class("android/graphics/Bitmap$Config").map_err(|_| "Config class not found")?;
        let config_field = env.get_static_field(&config_class, "ARGB_8888", "Landroid/graphics/Bitmap$Config;").map_err(|_| "ARGB_8888 field not found")?;
        let config_obj = config_field.l().map_err(|_| "Config object not found")?;

        let bitmap_class = env.find_class("android/graphics/Bitmap").map_err(|_| "Bitmap class not found")?;
        let array = env.new_object_array(num_parts as jint, &bitmap_class, JObject::null()).map_err(|_| "Object array creation failed")?;

        for (i, part) in parts.iter().enumerate() {
            let part_height = part.height();
            let new_bitmap = env.call_static_method(
                &bitmap_class,
                "createBitmap",
                "(IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;",
                &[
                    (src_info.width as jint).into(),
                    (part_height as jint).into(),
                    (&config_obj).into(),
                ],
            ).map_err(|_| "createBitmap failed")?.l().map_err(|_| "createBitmap result is not an object")?;

            // Lock Dest Pixels
            let mut dst_pixels: *mut std::ffi::c_void = std::ptr::null_mut();
            if ndk_sys::AndroidBitmap_lockPixels(raw_env, new_bitmap.as_raw(), &mut dst_pixels) < 0 {
                return Err("Failed to lock dest pixels");
            }

            // Copy rows
            let dst_stride = src_info.width as usize * 4; // ARGB_8888 is 4 bytes per pixel
            let dst_slice = slice::from_raw_parts_mut(
                dst_pixels as *mut u8,
                dst_stride * part_height as usize,
            );

            // Copy row by row
            for (row_idx, row_pixels) in part.iter_rows(0).enumerate() {
                let row_bytes = slice::from_raw_parts(row_pixels.as_ptr() as *const u8, dst_stride);
                let start = row_idx * dst_stride;
                dst_slice[start..start + dst_stride].copy_from_slice(row_bytes);
            }

            ndk_sys::AndroidBitmap_unlockPixels(raw_env, new_bitmap.as_raw());

            env.set_object_array_element(&array, i as jint, &new_bitmap).map_err(|_| "Failed to set object array element")?;
        }

        Ok::<jni::objects::JObjectArray, &'static str>(array)
    })();

    // Always unlock src
    ndk_sys::AndroidBitmap_unlockPixels(raw_env, raw_src);

    match res {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Fallback for non-android targets (to compile/test on host)
#[cfg(not(target_os = "android"))]
#[no_mangle]
pub unsafe extern "system" fn Java_io_github_fastimage_FastImageResizer_splitBitmap(
    _env: JNIEnv,
    _class: JClass,
    _src_bitmap: JObject,
    _num_parts: jint,
) -> jni::sys::jobjectArray {
    std::ptr::null_mut()
}
