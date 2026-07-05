# Panduan Dokumentasi & API JNI Kotlin - fast_image_resize

Dokumen ini menjelaskan API, contoh penggunaan, serta langkah integrasi pustaka native `fast_image_resize` ke dalam aplikasi Android menggunakan Kotlin.

---

## 1. Referensi API (Kotlin)

Pustaka diakses melalui objek `FastImageResizer` yang didefinisikan pada package `io.github.fastimage`.

### A. Enum `Algorithm`
Menentukan algoritma interpolasi gambar yang digunakan untuk proses resizing.

| Konstanta Enum | Nilai Native | Deskripsi |
| :--- | :--- | :--- |
| `NEAREST` | `0` | Sangat cepat, memilih piksel terdekat. Kualitas rendah (pixelated). Cocok untuk pixel art. |
| `BOX` | `1` | Rata-rata area piksel tetangga. Cepat untuk downscaling. |
| `BILINEAR` | `2` | Interpolasi linear 2x2 piksel. Menghasilkan gambar cukup halus dengan performa seimbang. |
| `BICUBIC` | `3` | Interpolasi kubik 4x4 piksel (Catmull-Rom). Lebih tajam dan detail dari Bilinear. |
| `LANCZOS3` | `4` | Interpolasi sinc 6x6 piksel. Kualitas terbaik untuk foto, gambar tajam, namun membutuhkan komputasi lebih tinggi. |

---

### B. Metode API

#### 1. `resizeRgba` (Kopling Byte Array)
Menerima array byte mentah gambar berformat RGBA dan mengembalikan gambar hasil resize dengan format yang sama.

```kotlin
external fun resizeRgba(
    src: ByteArray,
    srcWidth: Int,
    srcHeight: Int,
    dstWidth: Int,
    dstHeight: Int,
    algorithm: Int
): ByteArray?
```
* **Parameters:**
  * `src`: Array byte gambar asal (harus memiliki panjang minimal `srcWidth * srcHeight * 4` byte).
  * `srcWidth` / `srcHeight`: Dimensi piksel gambar asal.
  * `dstWidth` / `dstHeight`: Dimensi target gambar hasil resize.
  * `algorithm`: Nilai integer dari algoritma interpolasi (0–4).
* **Returns:** `ByteArray` berisi data piksel RGBA hasil resize, atau `null` jika proses gagal.

---

#### 2. `resizeBitmap` (Direct Zero-Copy)
Melakukan operasi resize secara langsung di dalam buffer memori objek Android `Bitmap` tanpa penyalinan memori antara Kotlin/Java heap dan Rust heap.

```kotlin
external fun resizeBitmap(
    srcBitmap: Bitmap,
    dstBitmap: Bitmap,
    algorithm: Int
): Boolean
```
* **Parameters:**
  * `srcBitmap`: Objek `Bitmap` asal (harus memiliki konfigurasi `Bitmap.Config.ARGB_8888`).
  * `dstBitmap`: Objek `Bitmap` tujuan (harus diinisialisasi terlebih dahulu dengan ukuran target dan konfigurasi `Bitmap.Config.ARGB_8888`).
  * `algorithm`: Nilai integer dari algoritma interpolasi (0–4).
* **Returns:** `Boolean` bernilai `true` jika berhasil, `false` jika gagal.

---

## 2. Contoh Penggunaan (Kotlin)

### Contoh A: Mengubah Ukuran Android `Bitmap` (Metode Zero-Copy - Direkomendasikan)
Metode ini adalah opsi terbaik untuk memproses gambar pada UI thread atau background worker Android karena tidak memakan alokasi heap memori tambahan untuk byte array.

```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun resizeAndroidBitmap(source: Bitmap, targetWidth: Int, targetHeight: Int): Bitmap? {
    // 1. Pastikan bitmap asal menggunakan format ARGB_8888
    val srcBitmap = if (source.config != Bitmap.Config.ARGB_8888) {
        source.copy(Bitmap.Config.ARGB_8888, false)
    } else {
        source
    }

    // 2. Buat bitmap kosong sebagai penampung hasil dengan ukuran target
    val dstBitmap = Bitmap.createBitmap(targetWidth, targetHeight, Bitmap.Config.ARGB_8888)

    // 3. Panggil fungsi native JNI menggunakan Lanczos3 untuk kualitas terbaik
    val success = FastImageResizer.resize(
        src = srcBitmap,
        dst = dstBitmap,
        alg = FastImageResizer.Algorithm.LANCZOS3
    )

    return if (success) {
        dstBitmap
    } else {
        dstBitmap.recycle()
        null
    }
}
```

---

### Contoh B: Mengubah Ukuran Byte Array RGBA
Sangat berguna apabila Anda memproses frame video mentah atau input dari kamera Android (seperti output dari CameraX/Camera2) yang berbentuk byte stream.

```kotlin
import io.github.fastimage.FastImageResizer

fun processRawCameraFrame(
    rgbaData: ByteArray,
    width: Int,
    height: Int,
    targetWidth: Int,
    targetHeight: Int
): ByteArray? {
    // Panggil helper resize untuk memproses byte array
    val resizedData = FastImageResizer.resize(
        src = rgbaData,
        srcW = width,
        srcH = height,
        dstW = targetWidth,
        dstH = targetHeight,
        alg = FastImageResizer.Algorithm.BILINEAR // Bilinear lebih cepat untuk pemrosesan real-time
    )
    
    return resizedData
}
```

---

## 3. Langkah Integrasi ke Android Studio

### Langkah 1: Salin File `.so`
Setelah melakukan kompilasi di Rust, letakkan file `.so` pada direktori module aplikasi Anda:
```text
[YourAndroidProject]/app/src/main/jniLibs/
  └── arm64-v8a/
        └── libfast_image_resize_jni.so
```

### Langkah 2: Salin File Kotlin
Buat berkas dengan nama `FastImageResizer.kt` di dalam proyek Android Anda dengan struktur path package berikut:
`app/src/main/java/io/github/fastimage/FastImageResizer.kt`

> [!IMPORTANT]
> Nama JNI method pada kode Rust (`Java_io_github_fastimage_FastImageResizer_...`) terikat langsung dengan nama package dan class Kotlin. Jika Anda memindahkan atau mengganti nama package Kotlin (misalnya menjadi `com.mycompany.app`), Anda harus mengganti nama fungsi yang di-eksport pada berkas Rust `jni_binding/src/lib.rs` agar sesuai.

### Langkah 3: Konfigurasi ProGuard / R8
Agar optimizer Android tidak menghapus objek JNI `FastImageResizer` saat membuild APK mode *Release*, tambahkan aturan berikut ke dalam berkas `proguard-rules.pro` aplikasi Anda:

```proguard
# Mencegah obfuscation pada kelas pembungkus JNI
-keep class io.github.fastimage.FastImageResizer {
    native <methods>;
}
```
