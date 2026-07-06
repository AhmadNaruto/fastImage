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

#### 1. `splitByHeight` (Direct Zero-Copy Height Splitting)
Membagi sebuah `Bitmap` vertikal yang panjang (seperti strip manhwa/webtoon) secara horizontal (berdasarkan tinggi) menjadi beberapa objek `Bitmap` yang lebih kecil secara rata.

```kotlin
fun splitByHeight(src: Bitmap, numParts: Int): Array<Bitmap>?
```
* **Parameters:**
  * `src`: Objek `Bitmap` asal yang ingin dipotong (harus menggunakan `Bitmap.Config.ARGB_8888`).
  * `numParts`: Jumlah potongan gambar yang diinginkan (harus > 0).
* **Returns:** `Array<Bitmap>` berisi objek-objek Bitmap hasil potongan, atau `null` jika proses gagal.

---

#### 2. `resizeByWidth` (Direct Aspect-Ratio Preserving Resize)
Mengubah ukuran sebuah `Bitmap` berdasarkan lebar target yang ditentukan, secara otomatis menghitung tinggi untuk menjaga rasio aspek asli gambar.

```kotlin
fun resizeByWidth(src: Bitmap, targetWidth: Int, alg: Algorithm): Bitmap?
```
* **Parameters:**
  * `src`: Objek `Bitmap` asal (harus memiliki konfigurasi `Bitmap.Config.ARGB_8888`).
  * `targetWidth`: Lebar target gambar hasil resize.
  * `alg`: Algoritma interpolasi (Gunakan `Algorithm.LANCZOS3` untuk manga/manhwa).
* **Returns:** Objek `Bitmap` hasil resize (dengan rasio aspek tetap terjaga), atau `null` jika proses gagal.

---

## 3. Contoh Penggunaan (Kotlin)

### Contoh A: Memotong Strip Manhwa Vertikal (`splitByHeight`)
Membagi strip manhwa panjang menjadi 4 bagian secara vertikal secara native & cepat tanpa overhead memori JVM:

```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun sliceManhwaStrip(manhwaStrip: Bitmap): Array<Bitmap>? {
    val srcBitmap = if (manhwaStrip.config != Bitmap.Config.ARGB_8888) {
        manhwaStrip.copy(Bitmap.Config.ARGB_8888, false)
    } else {
        manhwaStrip
    }
    return FastImageResizer.splitByHeight(srcBitmap, numParts = 4)
}
```

---

### Contoh B: Mengubah Ukuran Halaman Manga Berdasarkan Lebar (`resizeByWidth`)
Mengubah lebar halaman manga (misalnya menjadi lebar layar ponsel targetWidth = 1080px) secara otomatis menyesuaikan tinggi agar gambar tidak gepeng:

```kotlin
import android.graphics.Bitmap
import io.github.fastimage.FastImageResizer

fun fitMangaPageToWidth(source: Bitmap, targetWidth: Int): Bitmap? {
    val srcBitmap = if (source.config != Bitmap.Config.ARGB_8888) {
        source.copy(Bitmap.Config.ARGB_8888, false)
    } else {
        source
    }

    // Resize menggunakan Lanczos3 agar teks komik tetap tajam & rasio aspek terjaga
    return FastImageResizer.resizeByWidth(
        src = srcBitmap,
        targetWidth = targetWidth,
        alg = FastImageResizer.Algorithm.LANCZOS3
    )
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

### Langkah 3: Konfigurasi ProGuard / R8 (Sangat Penting)
Saat Anda membuild aplikasi Android dalam mode **Release**, R8/ProGuard akan melakukan optimalisasi, penciutan kode (*shrinking*), dan pengaburan nama (*obfuscation*). 

Karena kode native Rust JNI memanggil method Java/Android (`Bitmap.createBitmap`) dan mereferensikan kelas secara dinamis dari memori C++, Anda **wajib** mendaftarkan aturan ProGuard berikut agar aplikasi tidak mengalami crash (`NoSuchMethodError` atau `ClassNotFoundException`) di versi rilis:

Tambahkan baris berikut di berkas `proguard-rules.pro` proyek Android Anda:

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

#### Penjelasan Aturan:
1. **Rule 1**: Menjamin kelas Kotlin/Java `FastImageResizer` tidak diganti namanya (obfuscated) dan fungsi `external` (native) tetap dapat diakses oleh linker JNI.
2. **Rule 2 & 3**: Rust JNI menggunakan fungsi refleksi internal Android untuk membuat Bitmap baru secara native saat proses `splitByHeight`. Jika nama parameter/metode `createBitmap` di-obfuscate oleh ProGuard, Rust tidak akan bisa membuat Bitmap potongan baru di memori C++, yang akan menyebabkan aplikasi crash.
```
