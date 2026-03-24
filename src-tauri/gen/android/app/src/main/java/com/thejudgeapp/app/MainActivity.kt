package com.thejudgeapp.app

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  private var bottomInsetPx: Int = 0

  inner class SafeAreaBridge {
    @JavascriptInterface
    fun getBottomInset(): Int = bottomInsetPx
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    requestAppPermissions()

    ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { view, insets ->
      val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
      bottomInsetPx = systemBars.bottom
      ViewCompat.onApplyWindowInsets(view, insets)
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(SafeAreaBridge(), "__SafeArea__")
  }

  private fun requestAppPermissions() {
    val needed = mutableListOf<String>()
    if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA)
        != PackageManager.PERMISSION_GRANTED) {
      needed.add(Manifest.permission.CAMERA)
    }
    if (Build.VERSION.SDK_INT <= Build.VERSION_CODES.P) {
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
