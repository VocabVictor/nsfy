package com.nsfy.app.data.model

import androidx.room.Entity
import androidx.room.PrimaryKey
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.net.URI
import okhttp3.Request

// --- API models ---
data class NsfyMessage(
    val id: String,
    val time: Long,
    val title: String = "",
    val message: String = "",
    val priority: Int = 3,
    val tags: List<String> = emptyList(),
    val category: List<String> = emptyList(),
    val popup: Boolean = false,
    val bypassDnd: Boolean = false,
)

data class PublishRequest(
    val title: String = "",
    val message: String,
    val priority: Int = 3,
    val tags: List<String> = emptyList(),
    val category: List<String> = emptyList(),
    val popup: Boolean = false,
    val bypassDnd: Boolean = false,
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
    val lastConnectedAt: Long = 0,
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
    val category: String = "[]",
    val popup: Boolean = false,
    val bypassDnd: Boolean = false,
    val read: Boolean = false,
    val deletedAt: Long? = null,
)

@Entity(tableName = "message_states")
data class MessageStateEntity(
    @PrimaryKey val key: String,
    val topicId: String,
    val messageId: String,
    val status: String,
    val updatedAt: Long,
    val pending: Boolean = false,
)

// --- Server config ---
data class ServerConfig(
    val url: String,
    val name: String,
)

// --- Auth ---
fun authenticated(
    builder: Request.Builder,
    serverUrl: String,
    prefs: android.content.SharedPreferences,
    overrideToken: String? = null,
): Request.Builder {
    normalizeServerUrl(serverUrl)
    val token = overrideToken?.takeIf { it.isNotBlank() }
        ?: prefs.getString("server_token_$serverUrl", null)?.takeIf { it.isNotBlank() }
    return if (token == null) builder else builder.header("Authorization", "Bearer $token")
}

fun normalizeServerUrl(value: String): String {
    val text = value.trim().trimEnd('/')
    val uri = try {
        URI(text)
    } catch (_: Exception) {
        throw IllegalArgumentException("服务器地址无效")
    }
    require(uri.userInfo == null && uri.query == null && uri.fragment == null) {
        "服务器地址不能包含凭据、查询参数或片段"
    }
    val host = uri.host?.lowercase()?.trim('[', ']')
        ?: throw IllegalArgumentException("服务器地址缺少主机")
    val loopback = host == "localhost" || host == "::1" || isIpv4Loopback(host)
    require(uri.scheme == "https" || uri.scheme == "http" && loopback) {
        "远程服务器必须使用 HTTPS"
    }
    return text
}

private fun isIpv4Loopback(host: String): Boolean {
    val parts = host.split('.')
    return parts.size == 4 && parts[0] == "127" && parts.all {
        it.toIntOrNull()?.let { value -> value in 0..255 } == true
    }
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

fun categorySegments(value: String): List<String> = try {
    val array = org.json.JSONArray(value)
    (0 until array.length()).map { array.getString(it) }
} catch (_: Exception) {
    emptyList()
}

fun categoryPath(value: String): String = categorySegments(value).joinToString(" / ")
