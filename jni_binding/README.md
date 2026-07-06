# Panduan Dokumentasi & API JNI Kotlin - fast_image_resize

Dokumen ini menjelaskan API, contoh penggunaan, optimasi ukuran binary, serta langkah integrasi pustaka native `fast_image_resize` ke dalam aplikasi Android menggunakan Kotlin.

---

## 1. Optimasi Ukuran File (.so)
Secara default, pustaka ini mengompilasi semua format piksel (seperti U8, U16, Grayscale, dan F32). Namun, karena Android Bitmap default menggunakan format **`ARGB_8888`** (diwakili format `U8x4` pada Rust), biner native telah dioptimalkan dengan mengaktifkan flag feature `only_u8x4`.

Untuk mendukung kompresi WebP dan JPEG secara efisien, kami menambahkan dependensi Rust murni **`zenwebp`** dan **`jpeg-encoder`**. Kami menonaktifkan semua modul yang tidak digunakan dari `zenwebp` (termasuk modul integrasi tipe piksel eksternal) untuk meminimalkan dampak biner.

Dengan optimasi rilis compiler (`opt-level = "z"`, `lto = true`, `panic = "abort"`, `strip = true`), total ukuran berkas pustaka `.so` rilis gabungan untuk seluruh fitur ini (resizing, vertikal splitting, kompresi WebP, kompresi JPEG) hanyalah **~733 KB**, yang sangat kecil dan hemat RAM ketika berjalan di ponsel Android Anda.

---

## 2. Referensi API (Kotlin)

Pustaka diakses melalui objek `FastImageResizer` yang didefinisikan pada package `io.github.fastimage`.

### A. Enums

#### 1. Enum `Algorithm`
Menentukan algoritma interpolasi gambar yang digunakan untuk proses resizing.

| Konstanta Enum | Nilai Native | Deskripsi |
| :--- | :--- | :--- |
| `NEAREST` | `0` | Sangat cepat. Kualitas rendah (pixelated). Cocok untuk pixel art. |
| `BOX` | `1` | Rata-rata area piksel tetangga. Cepat untuk downscaling. |
| `BILINEAR` | `2` | Interpolasi linear 2x2 piksel. Menghasilkan gambar cukup halus dengan performa seimbang. |
| `BICUBIC` | `3` | Interpolasi kubik 4x4 piksel (Catmull-Rom). Lebih tajam dan detail dari Bilinear. |
| `LANCZOS3` | `4` | Interpolasi sinc 6x6 piksel. **Kualitas terbaik untuk foto/Manga**, gambar tetap tajam (teks dialog terbaca sangat jelas). |

#### 2. Enum `CompressFormat`
Menentukan format kompresi target saat mengekspor bitmap ke bytes.

| Konstanta Enum | Nilai Native | Deskripsi |
| :--- | :--- | :--- |
| `WEBP` | `0` | Kompresi WebP lossy (menggunakan `zenwebp` Rust). |
| `JPEG` | `1` | Kompresi JPEG (menggunakan `jpeg-encoder` Rust). |

---

### B. Metode API Utama

> [!NOTE]
> Semua fungsi helper di bawah ini secara otomatis mendeteksi konfigurasi format Bitmap asal. Jika gambar masukan bukan `ARGB_8888`, wrapper Kotlin akan menyalin dan mengonversinya secara otomatis, melakukan operasi native, lalu segera membebaskan memori salinan sementara tersebut dengan `.recycle()` untuk mencegah kebocoran memori (Memory Leak).

#### 1. `splitByHeight` (Direct Zero-Copy Height Splitting)
Membagi sebuah `Bitmap` vertikal yang panjang (seperti strip manhwa/webtoon) secara horizontal (berdasarkan tinggi) menjadi beberapa objek `Bitmap` yang lebih kecil secara rata.

```kotlin
fun splitByHeight(src: Bitmap, numParts: Int): Array<Bitmap>?
```
* **Parameters:**
  * `src`: Objek `Bitmap` asal yang ingin dipotong.
  * `numParts`: Jumlah potongan gambar yang diinginkan (harus > 0).
* **Returns:** `Array<Bitmap>` berisi objek-objek Bitmap hasil potongan, atau `null` jika proses gagal.

---

#### 2. `resizeByWidth` (Direct Aspect-Ratio Preserving Resize)
Mengubah ukuran sebuah `Bitmap` berdasarkan lebar target yang ditentukan, secara otomatis menghitung tinggi untuk menjaga rasio aspek asli gambar.

```kotlin
fun resizeByWidth(src: Bitmap, targetWidth: Int, alg: Algorithm = Algorithm.LANCZOS3): Bitmap?
```
* **Parameters:**
  * `src`: Objek `Bitmap` asal.
  * `targetWidth`: Lebar target gambar hasil resize.
  * `alg`: Algoritma interpolasi (Gunakan `Algorithm.LANCZOS3` untuk manga/manhwa).
* **Returns:** Objek `Bitmap` hasil resize (dengan rasio aspek tetap terjaga), atau `null` jika proses gagal.

---

#### 3. `compress` (Direct Image Compression to Bytes)
Mengompresi objek `Bitmap` Android ke format byte array WebP atau JPEG secara native.

```kotlin
fun compress(src: Bitmap, format: CompressFormat, quality: Int): ByteArray?
```
* **Parameters:**
  * `src`: Objek `Bitmap` asal yang ingin dikompresi.
  * `format`: Format kompresi target (`CompressFormat.WEBP` atau `CompressFormat.JPEG`).
  * `quality`: Kualitas gambar hasil kompresi (nilai integer dari `1` sampai `100`).
* **Returns:** `ByteArray` berisi biner gambar terkompresi, atau `null` jika kompresi gagal.

---

## 3. Contoh Penggunaan (Kotlin)

### Contoh A: Memotong Strip Manhwa Vertikal (`splitByHeight`)
```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun sliceManhwaStrip(manhwaStrip: Bitmap): Array<Bitmap>? {
    // Wrapper Kotlin secara otomatis menangani format bitmap & membersihkan memori sementara
    return FastImageResizer.splitByHeight(manhwaStrip, numParts = 4)
}
```

### Contoh B: Mengubah Ukuran Halaman Manga Berdasarkan Lebar (`resizeByWidth`)
```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun fitMangaPageToWidth(source: Bitmap, targetWidth: Int): Bitmap? {
    // Mengubah ukuran gambar dengan algoritma LANCZOS3 secara default
    return FastImageResizer.resizeByWidth(source, targetWidth)
}
```

### Contoh C: Mengompresi Halaman Manga ke WebP / JPEG (`compress`)
```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun exportToWebp(source: Bitmap, quality: Int): ByteArray? {
    // Kompresi kualitas gambar (misalnya: 80) langsung ke WebP
    return FastImageResizer.compress(source, FastImageResizer.CompressFormat.WEBP, quality)
}

fun exportToJpeg(source: Bitmap, quality: Int): ByteArray? {
    // Kompresi langsung ke JPEG
    return FastImageResizer.compress(source, FastImageResizer.CompressFormat.JPEG, quality)
}
```

---

## 4. Langkah Integrasi ke Android Studio

### Langkah 1: Salin File `.so`
Letakkan file pustaka `.so` yang telah dikompilasi ke direktori jniLibs proyek Anda:
```text
[YourAndroidProject]/app/src/main/jniLibs/
  └── arm64-v8a/
        └── libimagefast.so
```

### Langkah 2: Salin File Kotlin
Buat berkas dengan nama `FastImageResizer.kt` di dalam proyek Android Anda dengan struktur path package berikut:
`app/src/main/java/io/github/fastimage/FastImageResizer.kt`

### Langkah 3: Konfigurasi ProGuard / R8 (Sangat Penting)
Aturan berikut wajib ditambahkan pada berkas `proguard-rules.pro` proyek Android Anda untuk memastikan kompiler R8 tidak menghapus method atau field refleksi yang dipanggil secara dinamis dari kode native C++ Rust:

```proguard
# 1. Menjaga kelas FastImageResizer beserta seluruh metode native-nya
-keep class io.github.fastimage.FastImageResizer {
    native <methods>;
    *;
}

# 2. Menjaga kelas dan field Bitmap.Config (ARGB_8888) yang dipanggil oleh Rust JNI
-keep class android.graphics.Bitmap$Config {
    public static final android.graphics.Bitmap$Config ARGB_8888;
}

# 3. Menjaga tanda tangan metode statis Bitmap.createBitmap yang dipanggil oleh Rust JNI
-keep class android.graphics.Bitmap {
    public static android.graphics.Bitmap createBitmap(int, int, android.graphics.Bitmap$Config);
}
```
