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

// --- Formatting helpers ---
fun fmtTime(ts: Long): String {
    val now = System.currentTimeMillis()
    val msgTime = ts * 1000
    val diff = now - msgTime
    return when {
        diff < 60_000 -> "just now"
        diff < 3600_000 -> "${diff / 60_000}m ago"
        diff < 86400_000 -> "${diff / 3600_000}h ago"
        else -> {
            val fmt = SimpleDateFormat("MMM d HH:mm", Locale.getDefault())
            fmt.format(Date(msgTime))
        }
    }
}

fun priorityEmoji(p: Int): String = when {
    p >= 5 -> "⚡⚡"
    p >= 4 -> "⚡"
    else -> ""
}
