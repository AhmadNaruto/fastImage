package io.github.fastimage

import android.graphics.Bitmap

object FastImageResizer {
    
    init {
        // Loads the compiled .so library (libfast_image_resize_jni.so)
        System.loadLibrary("fast_image_resize_jni")
    }

    enum class Algorithm(val value: Int) {
        NEAREST(0),
        BOX(1),
        BILINEAR(2),
        BICUBIC(3),
        LANCZOS3(4)
    }

    /**
     * Resizes a raw RGBA byte array.
     * Returns a new byte array containing the resized RGBA image.
     */
    external fun resizeRgba(
        src: ByteArray,
        srcWidth: Int,
        srcHeight: Int,
        dstWidth: Int,
        dstHeight: Int,
        algorithm: Int
    ): ByteArray?

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
     * Splits an Android Bitmap into multiple Bitmaps by height (zero-copy).
     * The source bitmap MUST use Config.ARGB_8888.
     * Returns an array of Bitmaps, or null if the operation failed.
     */
    external fun splitBitmap(
        srcBitmap: Bitmap,
        numParts: Int
    ): Array<Bitmap>?

    /**
     * Splits an Android Bitmap into multiple Bitmaps by width (zero-copy).
     * The source bitmap MUST use Config.ARGB_8888.
     * Returns an array of Bitmaps, or null if the operation failed.
     */
    external fun splitBitmapByWidth(
        srcBitmap: Bitmap,
        numParts: Int
    ): Array<Bitmap>?

    // Kotlin friendly helper methods
    fun resize(src: ByteArray, srcW: Int, srcH: Int, dstW: Int, dstH: Int, alg: Algorithm): ByteArray? {
        return resizeRgba(src, srcW, srcH, dstW, dstH, alg.value)
    }

    fun resize(src: Bitmap, dst: Bitmap, alg: Algorithm): Boolean {
        require(src.config == Bitmap.Config.ARGB_8888) { "Source bitmap must be ARGB_8888" }
        require(dst.config == Bitmap.Config.ARGB_8888) { "Destination bitmap must be ARGB_8888" }
        return resizeBitmap(src, dst, alg.value)
    }

    /**
     * Resizes an Android Bitmap to a target width, automatically maintaining the original aspect ratio.
     * The source bitmap MUST use Config.ARGB_8888.
     * Returns the resized Bitmap, or null if the operation failed.
     */
    fun resizeByWidth(src: Bitmap, targetWidth: Int, alg: Algorithm): Bitmap? {
        require(src.config == Bitmap.Config.ARGB_8888) { "Source bitmap must be ARGB_8888" }
        if (targetWidth <= 0) return null
        
        // Calculate height preserving the aspect ratio
        val targetHeight = (src.height.toLong() * targetWidth / src.width).toInt()
        if (targetHeight <= 0) return null

        val dst = Bitmap.createBitmap(targetWidth, targetHeight, Bitmap.Config.ARGB_8888)
        val success = resizeBitmap(src, dst, alg.value)
        return if (success) {
            dst
        } else {
            dst.recycle()
            null
        }
    }

    @Deprecated("Use splitByHeight instead", ReplaceWith("splitByHeight(src, numParts)"))
    fun split(src: Bitmap, numParts: Int): Array<Bitmap>? {
        return splitByHeight(src, numParts)
    }

    fun splitByHeight(src: Bitmap, numParts: Int): Array<Bitmap>? {
        require(src.config == Bitmap.Config.ARGB_8888) { "Source bitmap must be ARGB_8888" }
        return splitBitmap(src, numParts)
    }

    fun splitByWidth(src: Bitmap, numParts: Int): Array<Bitmap>? {
        require(src.config == Bitmap.Config.ARGB_8888) { "Source bitmap must be ARGB_8888" }
        return splitBitmapByWidth(src, numParts)
    }
}
