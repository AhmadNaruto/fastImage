package io.github.fastimage

import android.graphics.Bitmap

object FastImageResizer {
    
    init {
        // Loads the compiled .so library (libimagefast.so)
        System.loadLibrary("imagefast")
    }

    enum class Algorithm(val value: Int) {
        NEAREST(0),
        BOX(1),
        BILINEAR(2),
        BICUBIC(3),
        LANCZOS3(4)
    }

    enum class CompressFormat(val value: Int) {
        WEBP(0),
        JPEG(1)
    }

    /**
     * Resizes an Android Bitmap into another existing Bitmap in-place.
     * Both bitmaps MUST use Config.ARGB_8888.
     * Returns true if the operation succeeded, false otherwise.
     */
    external fun resizeBitmap(
        srcBitmap: Bitmap,
        dstBitmap: Bitmap,
        algorithm: Int
    ): Boolean

    /**
     * Compresses an Android Bitmap directly into a WebP or JPEG byte array (zero-copy).
     * The source bitmap MUST use Config.ARGB_8888.
     * Returns the compressed byte array, or null if the operation failed.
     */
    external fun compressBitmap(
        srcBitmap: Bitmap,
        format: Int,
        quality: Int
    ): ByteArray?

    /**
     * Splits an Android Bitmap into multiple Bitmaps by height (zero-copy).
     * The source bitmap MUST use Config.ARGB_8888.
     * Returns an array of Bitmaps, or null if the operation failed.
     */
    external fun splitBitmap(
        srcBitmap: Bitmap,
        numParts: Int
    ): Array<Bitmap>?

    /**
     * Resizes an Android Bitmap to a target width, automatically maintaining the original aspect ratio.
     * Automatically converts the source Bitmap to ARGB_8888 if it's in another format, and recycles
     * the temporary bitmap afterwards to avoid memory leaks.
     * Returns the resized Bitmap, or null if the operation failed.
     */
    fun resizeByWidth(src: Bitmap, targetWidth: Int, alg: Algorithm = Algorithm.LANCZOS3): Bitmap? {
        if (targetWidth <= 0) return null
        
        // Auto-convert to ARGB_8888 if config is different
        val argbSrc = if (src.config != Bitmap.Config.ARGB_8888) {
            src.copy(Bitmap.Config.ARGB_8888, false) ?: return null
        } else {
            src
        }

        // Calculate height preserving the aspect ratio
        val targetHeight = (argbSrc.height.toLong() * targetWidth / argbSrc.width).toInt()
        if (targetHeight <= 0) {
            if (argbSrc !== src) argbSrc.recycle()
            return null
        }

        val dst = Bitmap.createBitmap(targetWidth, targetHeight, Bitmap.Config.ARGB_8888)
        val success = resizeBitmap(argbSrc, dst, alg.value)
        
        // Clean up temporary converted bitmap
        if (argbSrc !== src) {
            argbSrc.recycle()
        }

        return if (success) {
            dst
        } else {
            dst.recycle()
            null
        }
    }

    /**
     * Splits an Android Bitmap into multiple Bitmaps by height (zero-copy).
     * Automatically converts the source Bitmap to ARGB_8888 if it's in another format, and recycles
     * the temporary bitmap afterwards to avoid memory leaks.
     * Returns an array of Bitmaps, or null if the operation failed.
     */
    fun splitByHeight(src: Bitmap, numParts: Int): Array<Bitmap>? {
        if (numParts <= 0) return null

        // Auto-convert to ARGB_8888 if config is different
        val argbSrc = if (src.config != Bitmap.Config.ARGB_8888) {
            src.copy(Bitmap.Config.ARGB_8888, false) ?: return null
        } else {
            src
        }

        val result = splitBitmap(argbSrc, numParts)

        // Clean up temporary converted bitmap
        if (argbSrc !== src) {
            argbSrc.recycle()
        }

        return result
    }

    /**
     * Compresses an Android Bitmap directly into a WebP or JPEG byte array.
     * Automatically converts the source Bitmap to ARGB_8888 if it's in another format, and recycles
     * the temporary bitmap afterwards to avoid memory leaks.
     * Quality must be between 1 and 100.
     * Returns the compressed byte array, or null if the operation failed.
     */
    fun compress(src: Bitmap, format: CompressFormat, quality: Int): ByteArray? {
        if (quality < 1 || quality > 100) return null

        // Auto-convert to ARGB_8888 if config is different
        val argbSrc = if (src.config != Bitmap.Config.ARGB_8888) {
            src.copy(Bitmap.Config.ARGB_8888, false) ?: return null
        } else {
            src
        }

        val result = compressBitmap(argbSrc, format.value, quality)

        // Clean up temporary converted bitmap
        if (argbSrc !== src) {
            argbSrc.recycle()
        }

        return result
    }
}
