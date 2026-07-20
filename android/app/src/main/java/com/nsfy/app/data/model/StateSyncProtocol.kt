package com.nsfy.app.data.model

import org.json.JSONArray
import org.json.JSONObject

fun parseStateUpdates(text: String): List<Pair<String, String>> {
    val array = JSONObject(text).getJSONArray("updates")
    return (0 until array.length()).map { index ->
        val item = array.getJSONObject(index)
        val id = item.getString("id")
        val status = item.getString("status")
        require(id.isNotBlank() && status in setOf("read", "trash", "purged"))
        id to status
    }
}

fun stateUpdateBody(states: List<MessageStateEntity>): String {
    val updates = JSONArray()
    states.forEach { state ->
        updates.put(JSONObject().put("id", state.messageId).put("status", state.status))
    }
    return JSONObject().put("updates", updates).toString()
}
