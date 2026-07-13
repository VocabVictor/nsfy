package com.nsfy.app.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Light palette per the 信鸽 design mockup: white canvas, sky-blue accent.
val SkyBlue = Color(0xFF0EA5E9)
val SkyBlueDark = Color(0xFF0284C7)
val SkyBlueDim = Color(0x1F0EA5E9)
val Canvas = Color(0xFFFFFFFF)
val Shell = Color(0xFFF0F4F8)
val Card = Color(0xFFF6F7F9)
val Ink1 = Color(0xFF111827)
val Ink2 = Color(0xFF4B5563)
val Ink3 = Color(0xFF6B7280)
val Ink4 = Color(0xFF9CA3AF)
val Line = Color(0xFFE5E7EB)
val Danger = Color(0xFFEF4444)
val Warn = Color(0xFFF97316)
val Ok = Color(0xFF22C55E)

private val LightColors = lightColorScheme(
    primary = SkyBlue,
    onPrimary = Color.White,
    primaryContainer = SkyBlueDim,
    onPrimaryContainer = SkyBlueDark,
    secondary = SkyBlueDark,
    onSecondary = Color.White,
    secondaryContainer = SkyBlueDim,
    onSecondaryContainer = SkyBlueDark,
    background = Canvas,
    onBackground = Ink1,
    surface = Canvas,
    onSurface = Ink1,
    surfaceVariant = Card,
    onSurfaceVariant = Ink3,
    outline = Line,
    error = Danger,
    onError = Color.White,
)

// Deterministic color for a topic dot/tag, same palette as the desktop.
private val TopicPalette = listOf(
    Color(0xFFEF4444), Color(0xFFF97316), Color(0xFFF59E0B), Color(0xFF22C55E),
    Color(0xFF14B8A6), Color(0xFF0EA5E9), Color(0xFF3B82F6), Color(0xFF8B5CF6),
)

fun topicColor(name: String): Color {
    var h = 0
    for (c in name) h = h * 31 + c.code
    val idx = ((h % TopicPalette.size) + TopicPalette.size) % TopicPalette.size
    return TopicPalette[idx]
}

fun priorityColor(p: Int): Color = when {
    p >= 5 -> Danger
    p >= 4 -> Warn
    p >= 3 -> Ink3
    else -> Ink4
}

@Composable
fun NsfyTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = LightColors,
        content = content,
    )
}
