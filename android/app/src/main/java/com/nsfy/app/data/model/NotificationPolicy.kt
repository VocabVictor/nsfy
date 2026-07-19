package com.nsfy.app.data.model

const val KEY_DO_NOT_DISTURB = "do_not_disturb"
const val KEY_DND_PRIORITIES = "dnd_allowed_priorities"
const val KEY_NOTIFICATION_MODE = "notification_mode"

enum class NotificationMode(val value: String) {
    Silent("silent"),
    System("system"),
    Temporary("temporary"),
    Persistent("persistent");

    companion object {
        fun from(value: String?): NotificationMode = entries.firstOrNull {
            it.value == value
        } ?: System
    }
}

fun shouldPresentNotification(
    message: NsfyMessage,
    doNotDisturb: Boolean,
    allowedPriorities: Set<Int>,
): Boolean = message.popup && (
    !doNotDisturb || message.bypassDnd || message.priority in allowedPriorities
)
