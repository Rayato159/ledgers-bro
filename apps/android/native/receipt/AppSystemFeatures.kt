package dev.dioxus.main

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import java.io.File
import java.security.MessageDigest

/** The system owns installation consent. Only this app's newer, same-signer APK is accepted. */
class AppSystemFeatures(private val activity: MainActivity) {
    @Suppress("DEPRECATION")
    fun install(filename: String): Int = try {
        val file = File(filename).canonicalFile
        val root = File(activity.cacheDir, "updates").canonicalFile
        require(file.parentFile == root && file.isFile && file.extension == "apk")
        val pm = activity.packageManager
        val flags = if (Build.VERSION.SDK_INT >= 28) PackageManager.GET_SIGNING_CERTIFICATES else PackageManager.GET_SIGNATURES
        val installed = pm.getPackageInfo(activity.packageName, flags)
        val candidate = pm.getPackageArchiveInfo(file.path, flags) ?: error("Invalid APK")
        require(candidate.packageName == activity.packageName)
        val newer = if (Build.VERSION.SDK_INT >= 28) candidate.longVersionCode > installed.longVersionCode else candidate.versionCode > installed.versionCode
        require(newer && signers(installed) == signers(candidate))
        if (Build.VERSION.SDK_INT >= 26 && !pm.canRequestPackageInstalls()) {
            activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${activity.packageName}")))
            2
        } else {
            val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.updates", file)
            activity.startActivity(Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            })
            1
        }
    } catch (_: Exception) { 0 }

    @Suppress("DEPRECATION")
    private fun signers(info: PackageInfo): Set<String> {
        val signatures = if (Build.VERSION.SDK_INT >= 28) info.signingInfo?.apkContentsSigners else info.signatures
        require(!signatures.isNullOrEmpty())
        return signatures.map { signature -> MessageDigest.getInstance("SHA-256").digest(signature.toByteArray()).joinToString("") { "%02x".format(it) } }.toSet()
    }

    fun enableNotifications(): Boolean {
        if (Build.VERSION.SDK_INT >= 33 && activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 7105)
            return false
        }
        val manager = activity.getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26) manager.createNotificationChannel(NotificationChannel("ledger-reminders", "Bills and app updates", NotificationManager.IMPORTANCE_DEFAULT))
        return manager.areNotificationsEnabled()
    }

    @Suppress("DEPRECATION")
    fun notify(title: String, body: String): Boolean = try {
        val manager = activity.getSystemService(NotificationManager::class.java)
        if (!manager.areNotificationsEnabled()) false else {
            val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(activity, "ledger-reminders") else Notification.Builder(activity)
            val intent = PendingIntent.getActivity(activity, 7105, Intent(activity, MainActivity::class.java), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            manager.notify(7105, builder.setSmallIcon(activity.applicationInfo.icon).setContentTitle(title).setContentText(body).setContentIntent(intent).setAutoCancel(true).setVisibility(Notification.VISIBILITY_PRIVATE).build())
            true
        }
    } catch (_: Exception) { false }
}
