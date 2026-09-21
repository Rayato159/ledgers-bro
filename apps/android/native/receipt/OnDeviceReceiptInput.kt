package dev.dioxus.main

import android.app.Activity
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ColorSpace
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Build
import com.googlecode.tesseract.android.TessBaseAPI
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.ByteBuffer
import java.security.MessageDigest
import java.util.Timer
import java.util.TimerTask
import java.util.concurrent.atomic.AtomicBoolean

/** Platform IO only. Outputs an oriented preview and untrusted OCR text, never a transaction. */
class OnDeviceReceiptInput(private val activity: Activity) {
    @Volatile private var id = 0L
    @Volatile private var status = 4
    @Volatile private var failure = 0
    @Volatile private var outputText = ""
    @Volatile private var outputPreview = byteArrayOf()
    private val busy = AtomicBoolean(false)
    private val cancelled = AtomicBoolean(false)
    private val engineLock = Any()
    private var engine: TessBaseAPI? = null
    private var pendingPicker: Long? = null

    fun state(request: Long) = if (request == id) status else 4
    fun text(request: Long) = if (request == id && status == 3) outputText else ""
    fun preview(request: Long) = if (request == id && status == 3) outputPreview else byteArrayOf()
    fun error(request: Long) = if (request == id) failure else 0
    fun clear(request: Long) { if (request == id) { outputText = ""; outputPreview = byteArrayOf() } }

    fun begin(request: Long): Boolean {
        if (busy.get() || pendingPicker != null) return false
        id = request; status = 1; failure = 0; cancelled.set(false); clear(request)
        if (Build.VERSION.SDK_INT < 28) { failure = 5; status = 5; return true }
        return try {
            // SAF grants access only to the chosen document; no broad storage permission.
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
                // Some providers label HEIC as application/octet-stream. Validate bytes below.
                putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("image/jpeg", "image/png", "image/heic", "image/heif", "application/octet-stream"))
                putExtra(Intent.EXTRA_LOCAL_ONLY, true)
            }
            pendingPicker = request
            @Suppress("DEPRECATION")
            activity.startActivityForResult(intent, REQUEST)
            true
        } catch (_: Exception) { pendingPicker = null; failure = 7; status = 5; false }
    }

    fun result(resultCode: Int, uri: Uri?) {
        val request = pendingPicker ?: return
        pendingPicker = null
        if (request != id || cancelled.get()) return
        if (resultCode != Activity.RESULT_OK || uri == null) { status = 4; return }
        if (!busy.compareAndSet(false, true)) { failure = 7; status = 5; return }
        status = 2
        Thread({ process(request, uri) }, "ledger-receipt").start()
    }

    fun cancel(request: Long) {
        if (request != id) return
        cancelled.set(true); status = 4; clear(request)
        synchronized(engineLock) { engine?.stop() }
    }
    fun backgrounded() { if (status == 2) cancel(id) }
    fun destroy() { cancel(id); pendingPicker = null }
    private fun checkActive() { if (cancelled.get()) throw InterruptedException() }

    private fun process(request: Long, uri: Uri) {
        var bitmap: Bitmap? = null
        val timer = Timer("receipt-deadline", true)
        val timedOut = AtomicBoolean(false)
        timer.schedule(object : TimerTask() {
            override fun run() {
                timedOut.set(true); cancelled.set(true)
                synchronized(engineLock) { engine?.stop() }
            }
        }, 45_000)
        try {
            val bytes = activity.contentResolver.openInputStream(uri)?.use { input ->
                val output = ByteArrayOutputStream()
                val buffer = ByteArray(64 * 1024)
                while (true) {
                    checkActive()
                    val read = input.read(buffer)
                    if (read < 0) break
                    if (output.size() + read > MAX_BYTES) throw ReceiptFailure(1)
                    output.write(buffer, 0, read)
                }
                output.toByteArray()
            } ?: throw ReceiptFailure(1)
            if (!supported(bytes)) throw ReceiptFailure(1)
            checkActive()
            if (Build.VERSION.SDK_INT < 28) throw ReceiptFailure(5)
            bitmap = ImageDecoder.decodeBitmap(ImageDecoder.createSource(ByteBuffer.wrap(bytes))) { decoder, info, _ ->
                val w = info.size.width; val h = info.size.height
                if (w <= 0 || h <= 0 || w > 10000 || h > 10000 || w.toLong() * h > 50_000_000) throw ReceiptFailure(2)
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
                decoder.setTargetColorSpace(ColorSpace.get(ColorSpace.Named.SRGB))
                val ratio = minOf(1.0, 3200.0 / maxOf(w, h))
                decoder.setTargetSize(maxOf(1, (w * ratio).toInt()), maxOf(1, (h * ratio).toInt()))
                decoder.setOnPartialImageListener { false }
            }
            checkActive()
            val preview = ByteArrayOutputStream().use {
                if (!bitmap.compress(Bitmap.CompressFormat.JPEG, 95, it)) throw ReceiptFailure(2)
                it.toByteArray()
            }
            if (preview.size > MAX_BYTES) throw ReceiptFailure(2)
            bitmap.recycle()
            // OCR uses the exact pixels in the preview, without EXIF/GPS metadata.
            bitmap = BitmapFactory.decodeByteArray(preview, 0, preview.size) ?: throw ReceiptFailure(2)
            val root = prepareModels()
            checkActive()
            val tess = TessBaseAPI()
            synchronized(engineLock) { engine = tess }
            if (!tess.init(root.absolutePath, "tha+eng", TessBaseAPI.OEM_LSTM_ONLY)) throw ReceiptFailure(3)
            tess.setVariable("debug_file", "/dev/null")
            tess.pageSegMode = TessBaseAPI.PageSegMode.PSM_SINGLE_BLOCK
            tess.setImage(bitmap)
            checkActive()
            tess.getHOCRText(0) // This recognition entry point supports stop().
            checkActive()
            val text = tess.getUTF8Text() ?: ""
            if (text.isBlank()) throw ReceiptFailure(4)
            if (text.toByteArray(Charsets.UTF_8).size > 65536) throw ReceiptFailure(2)
            checkActive()
            if (request == id) { outputText = text; outputPreview = preview; status = 3 }
        } catch (error: ReceiptFailure) {
            if (request == id && !cancelled.get()) { failure = error.code; status = 5 }
        } catch (_: InterruptedException) {
            if (request == id) status = 4
        } catch (_: ImageDecoder.DecodeException) {
            if (request == id && !cancelled.get()) { failure = 8; status = 5 }
        } catch (_: Exception) {
            if (request == id && !cancelled.get()) { failure = 2; status = 5 }
        } catch (_: OutOfMemoryError) {
            if (request == id) { failure = 2; status = 5 }
        } finally {
            timer.cancel()
            synchronized(engineLock) { engine?.recycle(); engine = null }
            bitmap?.recycle()
            if (timedOut.get() && request == id) { failure = 6; status = 5 }
            busy.set(false)
        }
    }

    private fun prepareModels(): File {
        val root = File(activity.noBackupFilesDir, "ocr")
        val models = File(root, "tessdata")
        if (!models.isDirectory && !models.mkdirs()) throw ReceiptFailure(3)
        val hashes = mapOf(
            "tha" to "294227cc2d1292b0acb28d61d4115c88252b96d466ca90b417cf4cf0c67bf07c",
            "eng" to "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
        )
        for ((language, hash) in hashes) {
            checkActive()
            val file = File(models, "$language.traineddata")
            if (!file.isFile || digest(file) != hash) {
                val temp = File(models, "$language.tmp")
                try {
                    activity.assets.open("tessdata/$language.traineddata").use { source -> temp.outputStream().use { source.copyTo(it) } }
                    if (digest(temp) != hash || !temp.renameTo(file)) throw ReceiptFailure(3)
                } finally { temp.delete() }
            }
        }
        return root
    }
    private fun digest(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(65536)
            while (true) { val count = input.read(buffer); if (count < 0) break; digest.update(buffer, 0, count) }
        }
        return digest.digest().joinToString("") { "%02x".format(it.toInt() and 255) }
    }
    private fun supported(bytes: ByteArray): Boolean {
        if (bytes.size >= 8 && bytes.copyOfRange(0, 8).contentEquals(byteArrayOf(0x89.toByte(), 80, 78, 71, 13, 10, 26, 10))) return true
        if (bytes.size >= 3 && bytes[0] == 0xff.toByte() && bytes[1] == 0xd8.toByte() && bytes[2] == 0xff.toByte()) return true
        if (bytes.size < 16 || String(bytes, 4, 4, Charsets.US_ASCII) != "ftyp") return false
        val size = ByteBuffer.wrap(bytes, 0, 4).int
        if (size !in 16..256 || size > bytes.size || size % 4 != 0) return false
        var heif = false
        for (offset in 8 until size step 4) {
            if (offset == 12) continue
            val brand = String(bytes, offset, 4, Charsets.US_ASCII)
            if (brand in listOf("avif", "avis", "hevc", "hevx", "msf1")) return false
            if (brand in listOf("heic", "heix", "mif1")) heif = true
        }
        return heif
    }
    private class ReceiptFailure(val code: Int) : Exception()
    companion object { const val REQUEST = 4202; private const val MAX_BYTES = 32 * 1024 * 1024 }
}
