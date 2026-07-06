# Panduan Dokumentasi & API JNI Kotlin - fast_image_resize

Dokumen ini menjelaskan API, contoh penggunaan, optimasi ukuran binary, serta langkah integrasi pustaka native `fast_image_resize` ke dalam aplikasi Android menggunakan Kotlin.

---

## 1. Optimasi Ukuran File (.so)
Secara default, pustaka ini mengompilasi semua format piksel (seperti U8, U16, Grayscale, dan F32). Namun, karena Android Bitmap default menggunakan format **`ARGB_8888`** (diwakili format `U8x4` pada Rust), biner native telah dioptimalkan dengan mengaktifkan flag feature `only_u8x4`.

Ditambah dengan konfigurasi compiler `opt-level = "z"`, `lto = true`, dan `panic = "abort"`, ukuran pustaka native `.so` berhasil dipangkas dari **2,5 MB menjadi hanya ~340 KB** tanpa mengurangi performa atau fungsi yang Anda butuhkan (sangat hemat memori & RAM saat memproses file gambar besar seperti Manga/Manhwa).

---

## 2. Referensi API (Kotlin)

Pustaka diakses melalui objek `FastImageResizer` yang didefinisikan pada package `io.github.fastimage`.

### A. Enum `Algorithm`
Menentukan algoritma interpolasi gambar yang digunakan untuk proses resizing.

| Konstanta Enum | Nilai Native | Deskripsi |
| :--- | :--- | :--- |
| `NEAREST` | `0` | Sangat cepat. Kualitas rendah (pixelated). Cocok untuk pixel art. |
| `BOX` | `1` | Rata-rata area piksel tetangga. Cepat untuk downscaling. |
| `BILINEAR` | `2` | Interpolasi linear 2x2 piksel. Menghasilkan gambar cukup halus dengan performa seimbang. |
| `BICUBIC` | `3` | Interpolasi kubik 4x4 piksel (Catmull-Rom). Lebih tajam dan detail dari Bilinear. |
| `LANCZOS3` | `4` | Interpolasi sinc 6x6 piksel. **Kualitas terbaik untuk foto/Manga**, gambar tetap tajam (tulisan teks balon dialog terbaca jelas). |

---

### B. Metode API Utama

#### 1. `resizeBitmap` (Direct Zero-Copy Resize)
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

#### 2. `splitByHeight` & `splitByWidth` (Direct Zero-Copy Splitting)

* **`splitByHeight`**: Membagi sebuah `Bitmap` vertikal yang panjang (seperti strip manhwa/webtoon) secara horizontal (berdasarkan tinggi) menjadi beberapa objek `Bitmap` yang lebih kecil secara rata.
  ```kotlin
  fun splitByHeight(src: Bitmap, numParts: Int): Array<Bitmap>?
  ```
* **`splitByWidth`**: Membagi sebuah `Bitmap` lanskap yang lebar (seperti halaman ganda manga / double-spread) secara vertikal (berdasarkan lebar) menjadi beberapa objek `Bitmap` yang lebih kecil secara rata (misalnya menjadi halaman kiri dan kanan).
  ```kotlin
  fun splitByWidth(src: Bitmap, numParts: Int): Array<Bitmap>?
  ```
* **Parameters:**
  * `src`: Objek `Bitmap` asal yang ingin dipotong (harus menggunakan `Bitmap.Config.ARGB_8888`).
  * `numParts`: Jumlah potongan gambar yang diinginkan (harus > 0).
* **Returns:** `Array<Bitmap>` berisi objek-objek Bitmap hasil potongan, atau `null` jika proses gagal.

---

#### 3. `resizeRgba` (Kopling Byte Array)
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

---

## 3. Contoh Penggunaan (Kotlin)

### Contoh A: Memotong Halaman / Strip Manga & Manhwa
Berikut adalah cara memotong strip manhwa vertikal menjadi 4 bagian secara rata, serta membagi halaman ganda (double-page spread) manga menjadi 2 halaman tunggal.

```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

// Kasus 1: Membagi strip manhwa vertikal panjang menjadi 4 bagian secara rata
fun sliceManhwaStrip(manhwaStrip: Bitmap): Array<Bitmap>? {
    val srcBitmap = if (manhwaStrip.config != Bitmap.Config.ARGB_8888) {
        manhwaStrip.copy(Bitmap.Config.ARGB_8888, false)
    } else {
        manhwaStrip
    }
    return FastImageResizer.splitByHeight(srcBitmap, numParts = 4)
}

// Kasus 2: Membagi halaman ganda (double-spread) manga menjadi halaman kiri dan kanan
fun splitMangaDoublePage(doublePage: Bitmap): Array<Bitmap>? {
    val srcBitmap = if (doublePage.config != Bitmap.Config.ARGB_8888) {
        doublePage.copy(Bitmap.Config.ARGB_8888, false)
    } else {
        doublePage
    }
    return FastImageResizer.splitByWidth(srcBitmap, numParts = 2)
}
```

---

### Contoh B: Mengubah Ukuran Android `Bitmap` (`resize`)
Mengubah ukuran halaman manga sebelum ditampilkan ke UI agar rendering berjalan mulus.

```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun resizeMangaPage(source: Bitmap, targetWidth: Int, targetHeight: Int): Bitmap? {
    val srcBitmap = if (source.config != Bitmap.Config.ARGB_8888) {
        source.copy(Bitmap.Config.ARGB_8888, false)
    } else {
        source
    }

    val dstBitmap = Bitmap.createBitmap(targetWidth, targetHeight, Bitmap.Config.ARGB_8888)

    // Resize menggunakan Lanczos3 agar teks komik tetap tajam
    val success = FastImageResizer.resize(
        src = srcBitmap,
        dst = dstBitmap,
        alg = FastImageResizer.Algorithm.LANCZOS3
    )

    return if (success) dstBitmap else null
}
```

---

## 4. Langkah Integrasi ke Android Studio

### Langkah 1: Salin File `.so`
Letakkan file pustaka `.so` yang telah dikompilasi ke direktori jniLibs proyek Anda:
```text
[YourAndroidProject]/app/src/main/jniLibs/
  └── arm64-v8a/
        └── libfast_image_resize_jni.so
```

### Langkah 2: Salin File Kotlin
Buat berkas dengan nama `FastImageResizer.kt` di dalam proyek Android Anda dengan struktur path package berikut:
`app/src/main/java/io/github/fastimage/FastImageResizer.kt`

### Langkah 3: Konfigurasi ProGuard / R8
Agar optimizer Android tidak menghapus atau mengubah nama class JNI Kotlin saat membuild versi *Release*, tambahkan aturan berikut ke berkas `proguard-rules.pro` aplikasi Anda:

```proguard
# Mencegah obfuscation pada kelas pembungkus JNI
-keep class io.github.fastimage.FastImageResizer {
    native <methods>;
}
```
