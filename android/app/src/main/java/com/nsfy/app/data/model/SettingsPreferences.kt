package com.nsfy.app.data.model

import android.content.SharedPreferences
import org.json.JSONObject
import java.time.LocalDateTime

data class TopicRule(val mode: String = "normal", val bypassDnd: Boolean = false)

data class MobilePreferences(
    val autoStart: Boolean = false,
    val dndScheduleEnabled: Boolean = false,
    val dndStart: String = "22:00",
    val dndEnd: String = "08:00",
    val dndDays: Set<Int> = (1..7).toSet(),
    val showPreview: Boolean = true,
    val soundEnabled: Boolean = true,
    val urgentSoundEnabled: Boolean = true,
    val retentionDays: Int = 30,
    val trashRetentionDays: Int = 30,
    val proxyMode: String = "system",
    val topicRules: Map<String, TopicRule> = emptyMap(),
)

fun loadMobilePreferences(prefs: SharedPreferences): MobilePreferences = MobilePreferences(
    autoStart = prefs.getBoolean("auto_start", false),
    dndScheduleEnabled = prefs.getBoolean("dnd_schedule_enabled", false),
    dndStart = prefs.getString("dnd_start", "22:00") ?: "22:00",
    dndEnd = prefs.getString("dnd_end", "08:00") ?: "08:00",
    dndDays = prefs.getStringSet("dnd_days", (1..7).map(Int::toString).toSet()).orEmpty()
        .mapNotNull(String::toIntOrNull).toSet(),
    showPreview = prefs.getBoolean("show_preview", true),
    soundEnabled = prefs.getBoolean("sound_enabled", true),
    urgentSoundEnabled = prefs.getBoolean("urgent_sound_enabled", true),
    retentionDays = prefs.getInt("retention_days", 30),
    trashRetentionDays = prefs.getInt("trash_retention_days", 30),
    proxyMode = prefs.getString("proxy_mode", "system") ?: "system",
    topicRules = parseTopicRules(prefs.getString("topic_rules", null)),
)

fun saveMobilePreferences(prefs: SharedPreferences, value: MobilePreferences) {
    val rules = JSONObject()
    value.topicRules.forEach { (key, rule) ->
        rules.put(key, JSONObject().put("mode", rule.mode).put("bypassDnd", rule.bypassDnd))
    }
    prefs.edit()
        .putBoolean("auto_start", value.autoStart)
        .putBoolean("dnd_schedule_enabled", value.dndScheduleEnabled)
        .putString("dnd_start", value.dndStart)
        .putString("dnd_end", value.dndEnd)
        .putStringSet("dnd_days", value.dndDays.map(Int::toString).toSet())
        .putBoolean("show_preview", value.showPreview)
        .putBoolean("sound_enabled", value.soundEnabled)
        .putBoolean("urgent_sound_enabled", value.urgentSoundEnabled)
        .putInt("retention_days", value.retentionDays.coerceIn(1, 3650))
        .putInt("trash_retention_days", value.trashRetentionDays.coerceIn(1, 3650))
        .putString("proxy_mode", value.proxyMode)
        .putString("topic_rules", rules.toString())
        .apply()
}

fun topicRuleKey(server: String, topic: String) = "$server\n$topic"

fun scheduledDnd(value: MobilePreferences, now: LocalDateTime = LocalDateTime.now()): Boolean {
    if (!value.dndScheduleEnabled) return false
    val minutes = now.hour * 60 + now.minute
    fun parse(text: String): Int {
        val parts = text.split(':').mapNotNull(String::toIntOrNull)
        return if (parts.size == 2) parts[0] * 60 + parts[1] else 0
    }
    val start = parse(value.dndStart)
    val end = parse(value.dndEnd)
    val scheduleDay = if (start > end && minutes < end) {
        if (now.dayOfWeek.value == 1) 7 else now.dayOfWeek.value - 1
    } else now.dayOfWeek.value
    if (scheduleDay !in value.dndDays) return false
    return if (start <= end) minutes in start until end else minutes >= start || minutes < end
}

private fun parseTopicRules(text: String?): Map<String, TopicRule> = try {
    val json = JSONObject(text ?: "{}")
    json.keys().asSequence().associateWith { key ->
        val rule = json.getJSONObject(key)
        TopicRule(rule.optString("mode", "normal"), rule.optBoolean("bypassDnd", false))
    }
} catch (_: Exception) {
    emptyMap()
}
