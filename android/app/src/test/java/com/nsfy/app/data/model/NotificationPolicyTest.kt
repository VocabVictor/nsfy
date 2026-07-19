package com.nsfy.app.data.model

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationPolicyTest {
    private val message = NsfyMessage(
        id = "id", time = 1, priority = 4, popup = true,
    )

    @Test
    fun popupFlagAndDndPriorityAreRespected() {
        assertFalse(shouldPresentNotification(message.copy(popup = false), false, emptySet()))
        assertFalse(shouldPresentNotification(message, true, setOf(5)))
        assertTrue(shouldPresentNotification(message, true, setOf(4, 5)))
    }

    @Test
    fun senderCanBypassApplicationDnd() {
        assertTrue(shouldPresentNotification(message.copy(bypassDnd = true), true, emptySet()))
    }

    @Test
    fun notificationModesRoundTripTheirStoredValues() {
        NotificationMode.entries.forEach { mode ->
            assertTrue(NotificationMode.from(mode.value) == mode)
        }
    }
}
