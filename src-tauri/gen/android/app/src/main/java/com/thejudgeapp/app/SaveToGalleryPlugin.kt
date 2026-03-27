package com.thejudgeapp.app

import android.app.Activity
import android.content.ContentValues
import android.media.MediaScannerConnection
import android.os.Build
import android.provider.MediaStore
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@TauriPlugin
class SaveToGalleryPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun saveImage(invoke: Invoke) {
        try {
            val args = invoke.getArgs()
            val album = args.getString("album", "TheJudgeApp")!!
            val filename = args.getString("filename", "photo.jpg")!!
            val imageData = args.getString("data", null) ?: throw Exception("No image data")
            val bytes: ByteArray = Base64.decode(imageData, Base64.DEFAULT)

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val values = ContentValues().apply {
                    put(MediaStore.Images.Media.DISPLAY_NAME, filename)
                    put(MediaStore.Images.Media.MIME_TYPE, "image/jpeg")
                    put(MediaStore.Images.Media.RELATIVE_PATH, "DCIM/$album")
                }
                val uri = activity.contentResolver.insert(
                    MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values
                ) ?: throw Exception("Failed to create MediaStore entry")
                activity.contentResolver.openOutputStream(uri)?.use { out ->
                    out.write(bytes)
                } ?: throw Exception("Failed to open output stream")
            } else {
                val dir = android.os.Environment.getExternalStoragePublicDirectory(
                    android.os.Environment.DIRECTORY_DCIM
                )
                val albumDir = java.io.File(dir, album)
                albumDir.mkdirs()
                val file = java.io.File(albumDir, filename)
                file.writeBytes(bytes)
                MediaScannerConnection.scanFile(
                    activity, arrayOf(file.absolutePath), arrayOf("image/jpeg"), null
                )
            }

            invoke.resolve()
        } catch (ex: Exception) {
            invoke.reject(ex.message ?: ex.toString())
        }
    }

}
