package com.nsfy.app.data.model

import android.content.SharedPreferences
import okhttp3.OkHttpClient
import java.net.Proxy
import java.util.concurrent.TimeUnit

fun nsfyHttpClient(prefs: SharedPreferences, websocket: Boolean = false): OkHttpClient {
    val builder = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(if (websocket) 0 else 15, TimeUnit.SECONDS)
    if (prefs.getString("proxy_mode", "system") == "direct") builder.proxy(Proxy.NO_PROXY)
    return builder.build()
}
