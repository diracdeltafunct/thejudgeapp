package com.thejudgeapp.app

import android.app.Activity
import android.net.Uri
import androidx.browser.customtabs.CustomTabsIntent
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@TauriPlugin
class CustomTabsPlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun openUrl(invoke: Invoke) {
        try {
            val url = invoke.parseArgs(String::class.java)
            val intent = CustomTabsIntent.Builder().build()
            activity.runOnUiThread {
                intent.launchUrl(activity, Uri.parse(url))
            }
            invoke.resolve()
        } catch (ex: Exception) {
            invoke.reject(ex.message)
        }
    }
}
