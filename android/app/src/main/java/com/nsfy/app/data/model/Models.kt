package com.nsfy.app.data.model

import androidx.room.Entity
import androidx.room.PrimaryKey
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// --- API models ---
data class NsfyMessage(
    val id: String,
    val time: Long,
    val title: String = "",
    val message: String = "",
    val priority: Int = 3,
    val tags: List<String> = emptyList(),
)

data class PublishRequest(
    val title: String = "",
    val message: String,
    val priority: Int = 3,
    val tags: List<String> = emptyList(),
)

data class ServerStats(
    val topics: Int,
    val total_subscribers: Long,
    val topic_names: List<String>,
)

// --- Room entities ---
@Entity(tableName = "topics")
data class TopicEntity(
    @PrimaryKey val id: String,  // "server_url|topic_name"
    val serverUrl: String,
    val name: String,
    val lastMessageTime: Long = 0,
    val lastMessagePreview: String = "",
    val isConnected: Boolean = false,
)

@Entity(tableName = "messages")
data class MessageEntity(
    @PrimaryKey val id: String,
    val topicId: String,
    val time: Long,
    val title: String,
    val message: String,
    val priority: Int,
    val tags: String,  // comma-separated
)

// --- Server config ---
data class ServerConfig(
    val url: String,
    val name: String,
)

// --- Auth ---
// Append ?auth=<token> when a token is stored for the server (prefs key
// server_token_<url>). Works for both http and ws URLs.
fun withAuth(url: String, serverUrl: String, prefs: android.content.SharedPreferences): String {
    val token = prefs.getString("server_token_$serverUrl", null)?.takeIf { it.isNotBlank() }
        ?: return url
    val sep = if (url.contains('?')) '&' else '?'
    return "$url${sep}auth=${java.net.URLEncoder.encode(token, "UTF-8")}"
}

// --- Formatting helpers ---
fun fmtTime(ts: Long): String {
    val now = System.currentTimeMillis()
    val msgTime = ts * 1000
    val diff = now - msgTime
    return when {
        diff < 60_000 -> "刚刚"
        diff < 3600_000 -> "${diff / 60_000} 分钟前"
        diff < 86400_000 -> "${diff / 3600_000} 小时前"
        else -> {
            val fmt = SimpleDateFormat("M月d日 HH:mm", Locale.CHINA)
            fmt.format(Date(msgTime))
        }
    }
}

// Which calendar bucket a timestamp falls into, for the timeline grouping.
fun dateGroup(ts: Long): String {
    val cal = java.util.Calendar.getInstance()
    val today = cal.get(java.util.Calendar.YEAR) * 1000 + cal.get(java.util.Calendar.DAY_OF_YEAR)
    cal.timeInMillis = ts * 1000
    val day = cal.get(java.util.Calendar.YEAR) * 1000 + cal.get(java.util.Calendar.DAY_OF_YEAR)
    return when (today - day) {
        0 -> "今天"
        1 -> "昨天"
        else -> "更早"
    }
}

fun priorityLabel(p: Int): String = when {
    p >= 5 -> "紧急"
    p >= 4 -> "高"
    p >= 3 -> "普通"
    else -> "低"
}
