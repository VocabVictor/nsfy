package com.nsfy.app.ui.screens

import android.content.Context
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.sp

enum class Screen(val label: String, val icon: String) {
    Inbox("收件", "▤"),
    Topics("主题", "▦"),
    Publish("发送", "↑"),
    Settings("设置", "⚙"),
}

enum class TimelineTab(val label: String, val icon: String) {
    Timeline("时间线", "▤"),
    Subs("订阅", "▦"),
    Settings("设置", "⚙"),
}

const val PREFS_NAME = "nsfy_prefs"
const val KEY_LAYOUT_MODE = "layout_mode"

@Composable
fun NsfyMainScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE) }
    var layoutMode by remember {
        mutableStateOf(prefs.getString(KEY_LAYOUT_MODE, "split") ?: "split")
    }
    val setLayoutMode: (String) -> Unit = {
        layoutMode = it
        prefs.edit().putString(KEY_LAYOUT_MODE, it).apply()
    }

    if (layoutMode == "timeline") {
        TimelineShell(modifier, onLayoutChange = setLayoutMode)
    } else {
        SplitShell(modifier, onLayoutChange = setLayoutMode)
    }
}

@Composable
private fun SplitShell(modifier: Modifier = Modifier, onLayoutChange: (String) -> Unit) {
    var current by remember { mutableStateOf(Screen.Inbox) }

    Scaffold(
        modifier = modifier,
        bottomBar = {
            NavigationBar(
                containerColor = MaterialTheme.colorScheme.surface,
                contentColor = MaterialTheme.colorScheme.onSurface,
            ) {
                Screen.entries.forEach { screen ->
                    NavigationBarItem(
                        selected = current == screen,
                        onClick = { current = screen },
                        icon = { Text(screen.icon, fontSize = 20.sp) },
                        label = { Text(screen.label) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = MaterialTheme.colorScheme.primary,
                            selectedTextColor = MaterialTheme.colorScheme.primary,
                            indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                        ),
                    )
                }
            }
        },
    ) { padding ->
        Box(modifier = Modifier.padding(padding)) {
            when (current) {
                Screen.Inbox -> TimelineScreen()
                Screen.Topics -> TopicListScreen()
                Screen.Publish -> PublishScreen()
                Screen.Settings -> SettingsScreen(
                    onLayoutChange = onLayoutChange,
                    onSaved = { current = Screen.Inbox },
                )
            }
        }
    }
}

@Composable
private fun TimelineShell(modifier: Modifier = Modifier, onLayoutChange: (String) -> Unit) {
    var current by remember { mutableStateOf(TimelineTab.Timeline) }

    Scaffold(
        modifier = modifier,
        bottomBar = {
            NavigationBar(
                containerColor = MaterialTheme.colorScheme.surface,
                contentColor = MaterialTheme.colorScheme.onSurface,
            ) {
                TimelineTab.entries.forEach { tab ->
                    NavigationBarItem(
                        selected = current == tab,
                        onClick = { current = tab },
                        icon = { Text(tab.icon, fontSize = 20.sp) },
                        label = { Text(tab.label) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = MaterialTheme.colorScheme.primary,
                            selectedTextColor = MaterialTheme.colorScheme.primary,
                            indicatorColor = MaterialTheme.colorScheme.primaryContainer,
                        ),
                    )
                }
            }
        },
    ) { padding ->
        Box(modifier = Modifier.padding(padding)) {
            when (current) {
                TimelineTab.Timeline -> TimelineScreen()
                TimelineTab.Subs -> TopicListScreen()
                TimelineTab.Settings -> SettingsScreen(
                    onLayoutChange = onLayoutChange,
                    onSaved = { current = TimelineTab.Timeline },
                )
            }
        }
    }
}
