package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.nsfy.app.NsfyApp
import com.nsfy.app.data.db.MessageWithTopic
import com.nsfy.app.data.model.dateGroup
import com.nsfy.app.data.model.fmtTime
import com.nsfy.app.data.model.priorityLabel
import com.nsfy.app.data.repository.NsfyRepository
import com.nsfy.app.ui.theme.priorityColor
import com.nsfy.app.ui.theme.topicColor

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TimelineScreen() {
    val db = NsfyApp.instance.database
    val repo = remember { NsfyRepository(db) }
    val rows by repo.getAllMessagesWithTopic().collectAsState(initial = emptyList())

    var filter by remember { mutableStateOf("all") }

    // Messages from the last hour count as fresh for the 未读 chip; per-message
    // read state is not tracked in the schema.
    val fresh = rows.count { System.currentTimeMillis() / 1000 - it.msg.time < 3600 }
    val visible = if (filter == "all") rows
        else rows.filter { System.currentTimeMillis() / 1000 - it.msg.time < 3600 }

    // Group into 今天 / 昨天 / 更早 preserving order.
    val grouped = mutableListOf<Pair<String, MutableList<MessageWithTopic>>>()
    for (row in visible) {
        val label = dateGroup(row.msg.time)
        val last = grouped.lastOrNull()
        if (last != null && last.first == label) last.second.add(row)
        else grouped.add(label to mutableListOf(row))
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("信鸽", fontWeight = FontWeight.Bold) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                FilterChip(
                    selected = filter == "all",
                    onClick = { filter = "all" },
                    label = { Text("全部") },
                )
                FilterChip(
                    selected = filter == "unread",
                    onClick = { filter = "unread" },
                    label = { Text(if (fresh > 0) "未读 $fresh" else "未读") },
                )
            }

            if (visible.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text(
                            if (filter == "unread") "没有未读消息" else "暂无消息",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        if (filter == "all") {
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                "订阅主题后，推送会按时间线汇总在这里",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                            )
                        }
                    }
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    grouped.forEach { (label, items) ->
                        item(key = "header-$label") {
                            Text(
                                label,
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(top = 10.dp, bottom = 2.dp),
                            )
                        }
                        items(items, key = { it.msg.id }) { row ->
                            TimelineCard(row)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun TimelineCard(row: MessageWithTopic) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Surface(
                    modifier = Modifier.size(7.dp),
                    shape = MaterialTheme.shapes.extraLarge,
                    color = topicColor(row.topicName),
                ) {}
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    row.topicName,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.SemiBold,
                    color = topicColor(row.topicName),
                )
                if (row.msg.priority >= 4) {
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        priorityLabel(row.msg.priority),
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.SemiBold,
                        color = priorityColor(row.msg.priority),
                    )
                }
                Spacer(modifier = Modifier.weight(1f))
                Text(
                    fmtTime(row.msg.time),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                )
            }
            if (row.msg.title.isNotEmpty()) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    row.msg.title,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                row.msg.message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}
