package run.diceroll

import android.annotation.SuppressLint
import android.content.Intent
import android.content.Context
import android.os.Bundle
import android.view.View
import android.view.Window
import android.net.Uri
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.OnBackPressedCallback
import androidx.activity.ComponentActivity
import androidx.webkit.WebViewAssetLoader
import androidx.core.net.toUri

class MainActivity : ComponentActivity() {

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        super.onCreate(savedInstanceState)

        val webView = WebView(this).apply {
            settings.javaScriptEnabled = true
            settings.domStorageEnabled = true
            settings.setSupportZoom(false)
            settings.builtInZoomControls = false
            settings.displayZoomControls = false
            settings.allowFileAccess = false
            settings.allowContentAccess = false
            isVerticalScrollBarEnabled = false
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
        }

        val assetLoader = WebViewAssetLoader.Builder()
            .addPathHandler("/", WasmAwareAssetsPathHandler(this))
            .build()

        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(
                view: WebView,
                request: WebResourceRequest,
            ): Boolean = handleUrl(request.url)

            override fun shouldOverrideUrlLoading(
                view: WebView,
                url: String,
            ): Boolean = handleUrl(url.toUri())

            override fun shouldInterceptRequest(
                webView: WebView,
                request: WebResourceRequest,
            ): WebResourceResponse? = assetLoader.shouldInterceptRequest(request.url)

            private fun handleUrl(uri: Uri): Boolean {
                if (uri.host == "appassets.androidplatform.net") {
                    return false
                }

                this@MainActivity.startActivity(Intent(Intent.ACTION_VIEW, uri))
                return true
            }
        }

        setContentView(webView)

        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (webView.canGoBack()) {
                    webView.goBack()
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                }
            }
        })

        webView.loadUrl("https://appassets.androidplatform.net/index.html")
    }
}

private class WasmAwareAssetsPathHandler(
    context: Context,
) : WebViewAssetLoader.PathHandler {

    private val assets = context.assets
    private val fallback = WebViewAssetLoader.AssetsPathHandler(context)

    override fun handle(path: String): WebResourceResponse? {
        val assetPath = path.removePrefix("/")
        if (assetPath.endsWith(".wasm")) {
            return try {
                WebResourceResponse("application/wasm", null, assets.open(assetPath))
            } catch (_: java.io.IOException) {
                null
            }
        }
        return fallback.handle(path)
    }
}
