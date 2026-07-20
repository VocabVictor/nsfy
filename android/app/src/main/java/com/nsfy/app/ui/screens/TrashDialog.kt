package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nsfy.app.data.repository.NsfyRepository
import kotlinx.coroutines.launch

@Composable
fun TrashDialog(repository: NsfyRepository, onDismiss: () -> Unit) {
    val messages by repository.getTrashWithTopic().collectAsState(initial = emptyList())
    val scope = rememberCoroutineScope()
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("回收站") },
        text = {
            LazyColumn(
                modifier = Modifier.fillMaxWidth().heightIn(max = 460.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                if (messages.isEmpty()) item { Text("回收站为空") }
                items(messages, key = { "trash-${it.msg.id}" }) { row ->
                    Column(modifier = Modifier.fillMaxWidth()) {
                        Text(row.msg.title.ifEmpty { row.msg.message })
                        Text(row.topicName, style = MaterialTheme.typography.labelSmall)
                        Row(horizontalArrangement = Arrangement.End, modifier = Modifier.fillMaxWidth()) {
                            TextButton(onClick = { scope.launch { repository.restore(row.msg) } }) {
                                Text("恢复")
                            }
                            TextButton(onClick = { scope.launch { repository.purge(row.msg) } }) {
                                Text("永久删除", color = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("关闭") } },
    )
}
