package com.thejudgeapp.app

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.media.Ringtone
import android.media.RingtoneManager
import android.net.Uri
import android.net.http.SslError
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.RenderProcessGoneDetail
import android.webkit.SslErrorHandler
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.enableEdgeToEdge
import androidx.annotation.RequiresApi
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONArray
import org.json.JSONObject

class MainActivity : TauriActivity() {
  private var bottomInsetPx: Int = 0
  private var alarmBridge: AlarmBridge? = null

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
      stopSound()
    }

    fun stopSound() {
      runOnUiThread {
        currentRingtone?.stop()
        currentRingtone = null
      }
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
    if (intent.hasExtra("NotificationId")) {
      alarmBridge?.stopSound()
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(SafeAreaBridge(), "__SafeArea__")
    alarmBridge = AlarmBridge()
    webView.addJavascriptInterface(alarmBridge!!, "__AlarmSounds__")

    // On API 26+ the WebView renderer runs in a separate process. If Android kills
    // that process while the app is backgrounded, onRenderProcessGone is called on
    // resume. WRY/Tauri's WebViewClient does not override it, so the default returns
    // false and Android force-closes the app. We wrap the existing client to intercept
    // that callback and recreate the Activity instead of crashing.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val tauriClient = webView.webViewClient
      webView.webViewClient = object : WebViewClient() {
        @RequiresApi(Build.VERSION_CODES.O)
        override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
          if (!isFinishing && !isDestroyed) recreate()
          return true
        }
        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest) =
          tauriClient.shouldOverrideUrlLoading(view, request)
        override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest) =
          tauriClient.shouldInterceptRequest(view, request)
        override fun onPageStarted(view: WebView, url: String?, favicon: Bitmap?) =
          tauriClient.onPageStarted(view, url, favicon)
        override fun onPageFinished(view: WebView, url: String?) =
          tauriClient.onPageFinished(view, url)
        override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) =
          tauriClient.onReceivedSslError(view, handler, error)
      }
    }
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
