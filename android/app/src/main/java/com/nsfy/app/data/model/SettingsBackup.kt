package com.nsfy.app.data.model

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

fun exportPreferences(prefs: SharedPreferences, includeTokens: Boolean): String {
    val output = JSONObject()
    prefs.all.toSortedMap().forEach { (key, value) ->
        if (!includeTokens && key.startsWith("server_token_")) return@forEach
        when (value) {
            is Boolean, is Int, is Long, is Float, is String -> output.put(key, value)
            is Set<*> -> output.put(key, JSONArray(value.filterIsInstance<String>().sorted()))
        }
    }
    return output.toString(2)
}

fun importPreferences(prefs: SharedPreferences, content: String) {
    val input = JSONObject(content)
    input.optJSONArray("servers")?.let { servers ->
        for (index in 0 until servers.length()) normalizeServerUrl(servers.getString(index))
    }
    val editor = prefs.edit()
    input.keys().forEach { key ->
        when (val value = input.get(key)) {
            is Boolean -> editor.putBoolean(key, value)
            is Int -> editor.putInt(key, value)
            is Long -> editor.putLong(key, value)
            is Double -> editor.putFloat(key, value.toFloat())
            is String -> editor.putString(key, value)
            is JSONArray -> editor.putStringSet(key, (0 until value.length()).map {
                value.getString(it)
            }.toSet())
        }
    }
    editor.apply()
}
