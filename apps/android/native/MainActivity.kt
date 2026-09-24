package dev.dioxus.main

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.provider.OpenableColumns
import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.speech.RecognitionListener
import android.speech.RecognitionSupport
import android.speech.RecognitionSupportCallback
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import androidx.annotation.Keep
import com.dancingwithmycode.ledgersbro.test.BuildConfig
import java.util.concurrent.atomic.AtomicInteger

typealias BuildConfig = BuildConfig

// Platform adapter only. Accounting and CSV serialization remain in Rust.
class MainActivity : WryActivity() {
    private val voice by lazy { OnDeviceVoiceInput(this) }
    private val receipts by lazy { OnDeviceReceiptInput(this) }
    @Keep fun beginReceipt(id: Long): Boolean = receipts.begin(id)
    @Keep fun receiptState(id: Long): Int = receipts.state(id)
    @Keep fun receiptCount(id: Long): Int = receipts.count(id)
    @Keep fun receiptName(id: Long, index: Int): String = receipts.name(id, index)
    @Keep fun receiptItemError(id: Long, index: Int): Int = receipts.itemError(id, index)
    @Keep fun receiptText(id: Long, index: Int): String = receipts.text(id, index)
    @Keep fun receiptPreview(id: Long, index: Int): ByteArray = receipts.preview(id, index)
    @Keep fun receiptError(id: Long): Int = receipts.error(id)
    @Keep fun cancelReceipt(id: Long) = receipts.cancel(id)
    @Keep fun clearReceipt(id: Long) = receipts.clear(id)

    @Keep fun beginVoice(id: Long) = voice.begin(id)
    @Keep fun beginVoiceModelDownload(id: Long) = voice.begin(id, downloadModel = true)
    @Keep fun voiceState(id: Long): Int = voice.state(id)
    @Keep fun voiceText(id: Long): String = voice.text(id)
    @Keep fun voiceError(id: Long): Int = voice.error(id)
    @Keep fun stopVoice(id: Long) = voice.stop(id)
    @Keep fun cancelVoice(id: Long) = voice.cancel(id)

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == OnDeviceVoiceInput.PERMISSION_REQUEST) voice.permissionResult(grantResults)
    }

    override fun onStop() {
        voice.cancelActive()
        receipts.backgrounded()
        super.onStop()
    }

    override fun onDestroy() {
        voice.cancelActive()
        receipts.destroy()
        super.onDestroy()
    }

    @Suppress("DEPRECATION")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = Color.rgb(247, 241, 252)
        window.navigationBarColor = Color.rgb(255, 253, 253)
        // Dark system icons on the pastel, light app surface.
        window.decorView.systemUiVisibility = window.decorView.systemUiVisibility or
            android.view.View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR
        if (android.os.Build.VERSION.SDK_INT >= 26) {
            window.decorView.systemUiVisibility = window.decorView.systemUiVisibility or
                android.view.View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
        }
    }

    private val exportState = AtomicInteger(0)
    private var exportBytes: ByteArray? = null
    @Volatile private var exportName: String = ""

    @Keep
    fun beginCsvExport(filename: String, bytes: ByteArray): Boolean {
        if (exportState.get() == 1) return false
        exportState.set(1)
        exportBytes = bytes
        exportName = filename
        return try {
            val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "text/csv"
                putExtra(Intent.EXTRA_TITLE, filename)
            }
            @Suppress("DEPRECATION")
            startActivityForResult(intent, CSV_REQUEST)
            true
        } catch (_: Exception) {
            exportBytes = null
            exportState.set(4)
            false
        }
    }

    @Keep
    fun csvExportState(): Int = exportState.get()

    @Keep
    fun csvExportName(): String = exportName

    @Deprecated("Activity result bridge for the Dioxus WryActivity host")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == OnDeviceReceiptInput.REQUEST) {
            receipts.result(resultCode, data)
            return
        }
        if (requestCode != CSV_REQUEST) return
        val bytes = exportBytes
        exportBytes = null
        if (resultCode != Activity.RESULT_OK) {
            exportState.set(3)
            return
        }
        val uri = data?.data
        if (uri == null || bytes == null) {
            exportState.set(4)
            return
        }
        val resolver = applicationContext.contentResolver
        Thread({
            try {
                val output = resolver.openOutputStream(uri, "wt")
                    ?: throw java.io.IOException("No document output stream")
                output.use { it.write(bytes) }
                // The user/provider may rename the document in the system picker.
                exportName = runCatching {
                    resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                        if (it.moveToFirst()) it.getString(0) else null
                    }
                }.getOrNull() ?: exportName
                exportState.set(2)
            } catch (_: Exception) {
                exportState.set(4)
            }
        }, "ledger-csv-export").start()
    }

    companion object {
        private const val CSV_REQUEST = 4201
    }
}

/** Native microphone adapter; never receives accounts or writes ledger data.
 * Kept in the activity template because Dioxus copies that template into Gradle.
 * Every method/callback runs on Android's main thread. IDs reject stale callbacks.
 */
private class OnDeviceVoiceInput(private val activity: Activity) {
    private val handler = Handler(Looper.getMainLooper())
    private var recognizer: SpeechRecognizer? = null
    private var sessionId = 0L
    private var currentState = CANCELLED
    private var transcript = ""
    private var failure = 0
    private var permissionSession: Long? = null
    private var deadline: Runnable? = null

    fun state(id: Long): Int = if (id == sessionId) currentState else CANCELLED
    fun text(id: Long): String = if (id == sessionId && currentState == FINISHED) transcript else ""
    fun error(id: Long): Int = if (id == sessionId) failure else 0
    private fun active(id: Long) = id == sessionId && currentState in PREPARING..PROCESSING

    fun begin(id: Long, downloadModel: Boolean = false) {
        cancelActive()
        sessionId = id
        currentState = PREPARING
        transcript = ""
        failure = 0
        if (permissionSession != null) {
            fail(id, PERMISSION_PENDING)
            return
        }
        if (Build.VERSION.SDK_INT < 31 || !SpeechRecognizer.isOnDeviceRecognitionAvailable(activity)) {
            fail(id, UNAVAILABLE)
            return
        }
        try {
            // Do not replace this with createSpeechRecognizer + EXTRA_PREFER_OFFLINE:
            // that flag may be ignored by a recognizer and permit network fallback.
            val engine = SpeechRecognizer.createOnDeviceSpeechRecognizer(activity)
            recognizer = engine
            engine.setRecognitionListener(listener(id))
            armDeadline(id, 45_000)
            if (Build.VERSION.SDK_INT >= 33) {
                engine.checkRecognitionSupport(intent(), activity.mainExecutor, object : RecognitionSupportCallback {
                    override fun onSupportResult(support: RecognitionSupport) {
                        if (!active(id)) return
                        val installed = support.installedOnDeviceLanguages.any {
                            java.util.Locale.forLanguageTag(it.replace('_', '-')).language == "th"
                        }
                        val downloadable = support.supportedOnDeviceLanguages.any {
                            java.util.Locale.forLanguageTag(it.replace('_', '-')).language == "th"
                        }
                        val pending = support.pendingOnDeviceLanguages.any {
                            java.util.Locale.forLanguageTag(it.replace('_', '-')).language == "th"
                        }
                        when {
                            installed -> {
                                if (downloadModel) {
                                    currentState = MODEL_READY
                                    release()
                                } else requestMicrophone(id)
                            }
                            pending -> fail(id, THAI_MODEL_PENDING)
                            downloadable -> {
                                if (downloadModel) {
                                    try {
                                        // Support callback means the service is already connected.
                                        // Android owns the download; requesting is not completion.
                                        engine.triggerModelDownload(intent())
                                        currentState = MODEL_DOWNLOAD_REQUESTED
                                        release()
                                    } catch (_: Exception) { fail(id, MODEL_DOWNLOAD_FAILED) }
                                } else fail(id, THAI_MODEL_MISSING)
                            }
                            else -> fail(id, THAI_UNSUPPORTED)
                        }
                    }
                    override fun onError(error: Int) {
                        if (active(id)) fail(id, SUPPORT_UNCONFIRMED)
                    }
                })
            } else {
                // API 31-32 has no support query. This is still an on-device-only
                // recognizer; unsupported Thai will be reported by onError.
                if (downloadModel) fail(id, MODEL_DOWNLOAD_FAILED) else requestMicrophone(id)
            }
        } catch (_: Exception) {
            fail(id, START_FAILED)
        }
    }

    private fun intent() = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
        putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
        putExtra(RecognizerIntent.EXTRA_LANGUAGE, "th-TH")
        putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
        putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, false)
        putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
    }

    private fun requestMicrophone(id: Long) {
        if (!active(id)) return
        if (activity.checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) {
            listen(id)
        } else {
            currentState = PERMISSION
            permissionSession = id
            try {
                activity.requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), PERMISSION_REQUEST)
            } catch (_: Exception) {
                permissionSession = null
                fail(id, MICROPHONE_DENIED)
            }
        }
    }

    fun permissionResult(grants: IntArray) {
        val id = permissionSession
        permissionSession = null
        if (id == null || !active(id) || currentState != PERMISSION) return
        if (grants.firstOrNull() == PackageManager.PERMISSION_GRANTED) listen(id)
        else fail(id, MICROPHONE_DENIED)
    }

    private fun listen(id: Long) {
        if (!active(id)) return
        try {
            currentState = PREPARING
            recognizer?.startListening(intent()) ?: fail(id, START_FAILED)
        } catch (_: SecurityException) {
            fail(id, MICROPHONE_DENIED)
        } catch (_: Exception) {
            fail(id, START_FAILED)
        }
    }

    fun stop(id: Long) {
        if (!active(id) || currentState != LISTENING) return
        currentState = PROCESSING
        armDeadline(id, 10_000)
        try { recognizer?.stopListening() } catch (_: Exception) { fail(id, START_FAILED) }
    }

    fun cancel(id: Long) {
        if (id != sessionId) return
        currentState = CANCELLED
        transcript = ""
        release()
    }

    fun cancelActive() = cancel(sessionId)

    private fun fail(id: Long, code: Int) {
        if (!active(id)) return
        failure = code
        currentState = FAILED
        transcript = ""
        release()
    }

    private fun armDeadline(id: Long, milliseconds: Long) {
        deadline?.let { handler.removeCallbacks(it) }
        val timeout = Runnable { fail(id, TIMED_OUT) }
        deadline = timeout
        handler.postDelayed(timeout, milliseconds)
    }

    private fun release() {
        deadline?.let { handler.removeCallbacks(it) }
        deadline = null
        val old = recognizer
        recognizer = null
        // Terminal state is set first, so callbacks from cancel/destroy are ignored.
        try { old?.cancel() } catch (_: Exception) { /* Best effort; still destroy. */ }
        try { old?.destroy() } catch (_: Exception) { /* References are already released. */ }
    }

    private fun listener(id: Long) = object : RecognitionListener {
        override fun onReadyForSpeech(params: Bundle?) {
            if (active(id) && currentState == PREPARING) {
                currentState = LISTENING
                armDeadline(id, 30_000)
            }
        }
        override fun onEndOfSpeech() {
            if (active(id)) {
                currentState = PROCESSING
                armDeadline(id, 10_000)
            }
        }
        override fun onError(error: Int) = fail(id, error)
        override fun onResults(results: Bundle?) {
            if (!active(id)) return
            val value = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull()?.trim()
            if (value.isNullOrEmpty()) {
                fail(id, SpeechRecognizer.ERROR_NO_MATCH)
            } else {
                transcript = value
                currentState = FINISHED
                release()
            }
        }
        override fun onBeginningOfSpeech() {}
        override fun onRmsChanged(rmsdB: Float) {}
        override fun onBufferReceived(buffer: ByteArray?) {} // Never retain or log audio.
        override fun onPartialResults(partialResults: Bundle?) {}
        override fun onEvent(eventType: Int, params: Bundle?) {}
    }

    companion object {
        const val PERMISSION_REQUEST = 4202
        // JNI wire values shared with src/voice.rs. No transcript in error messages.
        private const val PREPARING = 1
        private const val PERMISSION = 2
        private const val LISTENING = 3
        private const val PROCESSING = 4
        private const val FINISHED = 5
        private const val CANCELLED = 6
        private const val FAILED = 7
        private const val MODEL_DOWNLOAD_REQUESTED = 8
        private const val MODEL_READY = 9
        private const val UNAVAILABLE = 1001
        private const val THAI_MODEL_MISSING = 1002
        private const val SUPPORT_UNCONFIRMED = 1003
        private const val MICROPHONE_DENIED = 1004
        private const val TIMED_OUT = 1005
        private const val PERMISSION_PENDING = 1006
        private const val START_FAILED = 1007
        private const val THAI_UNSUPPORTED = 1008
        private const val THAI_MODEL_PENDING = 1009
        private const val MODEL_DOWNLOAD_FAILED = 1010
    }
}
