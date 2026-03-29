package com.thejudgeapp.app

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.media.Ringtone
import android.media.RingtoneManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONArray
import org.json.JSONObject

class MainActivity : TauriActivity() {
  private var bottomInsetPx: Int = 0

  inner class SafeAreaBridge {
    @JavascriptInterface
    fun getBottomInset(): Int = bottomInsetPx
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    // Some OEM launchers deliver a duplicate launcher intent even with singleTask,
    // creating a second instance that immediately finishes — making the app appear
    // to close. Detect this and finish early so the existing task comes to front.
    if (!isTaskRoot
        && savedInstanceState == null
        && intent?.action == Intent.ACTION_MAIN
        && intent?.hasCategory(Intent.CATEGORY_LAUNCHER) == true
        && !intent.hasExtra("NotificationId")) {
      finish()
      return
    }
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    requestAppPermissions()

    ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { view, insets ->
      val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
      bottomInsetPx = systemBars.bottom
      ViewCompat.onApplyWindowInsets(view, insets)
    }
  }

  inner class AlarmBridge {
    @Volatile private var currentRingtone: Ringtone? = null

    @JavascriptInterface
    fun listAlarmSounds(): String {
      if (isFinishing || isDestroyed) return "[]"
      return try {
        val mgr = RingtoneManager(this@MainActivity)
        mgr.setType(RingtoneManager.TYPE_ALARM)
        val cursor = mgr.cursor
        val arr = JSONArray()
        while (cursor.moveToNext()) {
          val obj = JSONObject()
          obj.put("title", cursor.getString(RingtoneManager.TITLE_COLUMN_INDEX))
          obj.put("uri", mgr.getRingtoneUri(cursor.position).toString())
          arr.put(obj)
        }
        cursor.close()
        arr.toString()
      } catch (_: Exception) { "[]" }
    }

    @JavascriptInterface
    fun playAlarmSound(uriString: String) {
      if (isFinishing || isDestroyed) return
      runOnUiThread {
        if (isFinishing || isDestroyed) return@runOnUiThread
        currentRingtone?.stop()
        try {
          val ringtone = RingtoneManager.getRingtone(applicationContext, Uri.parse(uriString))
          ringtone?.play()
          currentRingtone = ringtone
        } catch (_: Exception) { }
      }
    }

    @JavascriptInterface
    fun stopAlarmSound() {
      if (isFinishing || isDestroyed) return
      runOnUiThread {
        currentRingtone?.stop()
        currentRingtone = null
      }
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(SafeAreaBridge(), "__SafeArea__")
    webView.addJavascriptInterface(AlarmBridge(), "__AlarmSounds__")
  }

  private fun requestAppPermissions() {
    val needed = mutableListOf<String>()
    if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA)
        != PackageManager.PERMISSION_GRANTED) {
      needed.add(Manifest.permission.CAMERA)
    }
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.READ_MEDIA_IMAGES)
          != PackageManager.PERMISSION_GRANTED) {
        needed.add(Manifest.permission.READ_MEDIA_IMAGES)
      }
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
          != PackageManager.PERMISSION_GRANTED) {
        needed.add(Manifest.permission.POST_NOTIFICATIONS)
      }
    } else if (Build.VERSION.SDK_INT <= Build.VERSION_CODES.P) {
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.WRITE_EXTERNAL_STORAGE)
          != PackageManager.PERMISSION_GRANTED) {
        needed.add(Manifest.permission.WRITE_EXTERNAL_STORAGE)
      }
    }
    if (needed.isNotEmpty()) {
      ActivityCompat.requestPermissions(this, needed.toTypedArray(), 0)
    }
  }
}
