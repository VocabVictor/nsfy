package com.nsfy.app.data.model

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import java.time.LocalDateTime

class StateSyncProtocolTest {
    @Test
    fun statePayloadRoundTripsAllStatuses() {
        val states = listOf("read", "trash", "purged").mapIndexed { index, status ->
            MessageStateEntity("key$index", "topic", "id$index", status, 1, true)
        }
        assertEquals(
            listOf("id0" to "read", "id1" to "trash", "id2" to "purged"),
            parseStateUpdates(stateUpdateBody(states)),
        )
    }

    @Test
    fun statePayloadRejectsUnknownStatuses() {
        assertThrows(IllegalArgumentException::class.java) {
            parseStateUpdates("""{"updates":[{"id":"one","status":"unknown"}]}""")
        }
    }

    @Test
    fun scheduledDndSupportsOvernightRangesAndSelectedDays() {
        val settings = MobilePreferences(
            dndScheduleEnabled = true, dndStart = "22:00", dndEnd = "08:00", dndDays = setOf(1),
        )
        assertEquals(true, scheduledDnd(settings, LocalDateTime.of(2026, 7, 20, 23, 0)))
        assertEquals(true, scheduledDnd(settings, LocalDateTime.of(2026, 7, 21, 1, 0)))
        assertEquals(false, scheduledDnd(settings, LocalDateTime.of(2026, 7, 21, 23, 0)))
    }
}
