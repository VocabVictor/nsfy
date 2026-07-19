package com.nsfy.app.ui.screens

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nsfy.app.data.model.NotificationMode

private val modes = listOf(
    NotificationMode.Silent to ("静默" to "只进入收件箱"),
    NotificationMode.System to ("系统通知" to "进入通知中心"),
    NotificationMode.Temporary to ("临时横幅" to "五秒后消失"),
    NotificationMode.Persistent to ("持续提醒" to "手动关闭前保留"),
)

@Composable
fun NotificationSettingsSection(
    doNotDisturb: Boolean,
    onDoNotDisturbChange: (Boolean) -> Unit,
    allowedPriorities: Set<Int>,
    onAllowedPrioritiesChange: (Set<Int>) -> Unit,
    mode: NotificationMode,
    onModeChange: (NotificationMode) -> Unit,
) {
    Text("窗口与通知", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Column(modifier = Modifier.weight(1f)) {
            Text("勿扰模式", style = MaterialTheme.typography.bodyMedium)
            Text("消息仍会进入收件箱", style = MaterialTheme.typography.bodySmall)
        }
        Switch(checked = doNotDisturb, onCheckedChange = onDoNotDisturbChange)
    }
    Text("勿扰时仍允许弹窗", style = MaterialTheme.typography.labelMedium)
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(5.dp),
    ) {
        listOf(5 to "紧急", 4 to "高", 3 to "普通", 2 to "低", 1 to "最低").forEach { item ->
            FilterChip(
                selected = item.first in allowedPriorities,
                onClick = {
                    onAllowedPrioritiesChange(if (item.first in allowedPriorities) {
                        allowedPriorities - item.first
                    } else {
                        allowedPriorities + item.first
                    })
                },
                label = { Text(item.second) },
            )
        }
    }
    Text("通知样式", style = MaterialTheme.typography.labelMedium)
    modes.chunked(2).forEach { row ->
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            row.forEach { (value, text) ->
                FilterChip(
                    modifier = Modifier.weight(1f),
                    selected = mode == value,
                    onClick = { onModeChange(value) },
                    label = {
                        Column { Text(text.first); Text(text.second, style = MaterialTheme.typography.labelSmall) }
                    },
                )
            }
        }
    }
}
