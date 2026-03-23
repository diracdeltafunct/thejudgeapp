package com.thejudgeapp.app

import android.app.Activity
import android.content.ContentValues
import android.os.Build
import android.provider.MediaStore
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

data class SaveImageArgs(
    val album: String = "TheJudgeApp",
    val filename: String = "photo.jpg",
    val data: String = ""
)

data class SaveTextArgs(
    val filename: String = "notes.txt",
    val content: String = ""
)

@TauriPlugin
class SaveToGalleryPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun saveImage(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(SaveImageArgs::class.java)
            val bytes = Base64.decode(args.data, Base64.DEFAULT)

            val values = ContentValues().apply {
                put(MediaStore.Images.Media.DISPLAY_NAME, args.filename)
                put(MediaStore.Images.Media.MIME_TYPE, "image/jpeg")
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    put(MediaStore.Images.Media.RELATIVE_PATH, "Pictures/${args.album}")
                }
            }

            val uri = activity.contentResolver.insert(
                MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values
            ) ?: throw Exception("Failed to create MediaStore entry")

            activity.contentResolver.openOutputStream(uri)?.use { out ->
                out.write(bytes)
            } ?: throw Exception("Failed to open output stream")

            invoke.resolve()
        } catch (ex: Exception) {
            invoke.reject(ex.message)
        }
    }

    @Command
    fun saveTextFile(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(SaveTextArgs::class.java)
            val bytes = args.content.toByteArray(Charsets.UTF_8)

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val values = ContentValues().apply {
                    put(MediaStore.Downloads.DISPLAY_NAME, args.filename)
                    put(MediaStore.Downloads.MIME_TYPE, "text/plain")
                    put(MediaStore.Downloads.RELATIVE_PATH, "Download")
                }
                val uri = activity.contentResolver.insert(
                    MediaStore.Downloads.EXTERNAL_CONTENT_URI, values
                ) ?: throw Exception("Failed to create MediaStore entry")
                activity.contentResolver.openOutputStream(uri)?.use { out ->
                    out.write(bytes)
                } ?: throw Exception("Failed to open output stream")
            } else {
                val dir = android.os.Environment.getExternalStoragePublicDirectory(
                    android.os.Environment.DIRECTORY_DOWNLOADS
                )
                dir.mkdirs()
                java.io.File(dir, args.filename).writeBytes(bytes)
            }

            invoke.resolve()
        } catch (ex: Exception) {
            invoke.reject(ex.message)
        }
    }
}
