package com.nsfy.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.nsfy.app.data.model.TopicEntity
import com.nsfy.app.data.model.fmtTime
import com.nsfy.app.data.repository.NsfyRepository
import com.nsfy.app.NsfyApp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopicListScreen() {
    val db = NsfyApp.instance.database
    val repo = remember { NsfyRepository(db) }
    val topics by repo.getAllTopics().collectAsState(initial = emptyList())

    var showAddDialog by remember { mutableStateOf(false) }
    var selectedTopic by remember { mutableStateOf<TopicEntity?>(null) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("nsfy", fontWeight = FontWeight.Bold) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
                actions = {
                    TextButton(onClick = { showAddDialog = true }) {
                        Text("＋", fontSize = MaterialTheme.typography.titleLarge.fontSize)
                    }
                }
            )
        },
    ) { padding ->
        if (selectedTopic != null) {
            TopicDetailScreen(
                topic = selectedTopic!!,
                onBack = { selectedTopic = null },
                modifier = Modifier.padding(padding),
            )
        } else if (topics.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("🔔", style = MaterialTheme.typography.displayMedium)
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        "No topics yet",
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        "Tap ＋ to subscribe",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                    )
                }
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                items(topics) { topic ->
                    TopicCard(topic = topic, onClick = { selectedTopic = topic })
                }
            }
        }
    }

    if (showAddDialog) {
        AddTopicDialog(
            onDismiss = { showAddDialog = false },
            onAdd = { server, name ->
                GlobalScope.launch(Dispatchers.Main) {
                    repo.addTopic(server, name)
                    val intent = android.content.Intent(
                        NsfyApp.instance,
                        com.nsfy.app.service.WebSocketService::class.java
                    )
                    NsfyApp.instance.startService(intent)
                }
                showAddDialog = false
            }
        )
    }
}

@Composable
fun TopicCard(topic: TopicEntity, onClick: () -> Unit) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                modifier = Modifier.size(8.dp),
                shape = MaterialTheme.shapes.extraLarge,
                color = if (topic.isConnected)
                    MaterialTheme.colorScheme.primary
                else
                    MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.3f),
            ) {}
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    topic.name,
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
                if (topic.lastMessagePreview.isNotEmpty()) {
                    Text(
                        topic.lastMessagePreview,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
            if (topic.lastMessageTime > 0) {
                Text(
                    fmtTime(topic.lastMessageTime),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                )
            }
        }
    }
}

@Composable
fun AddTopicDialog(onDismiss: () -> Unit, onAdd: (String, String) -> Unit) {
    var server by remember { mutableStateOf("http://") }
    var name by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Subscribe to Topic") },
        text = {
            Column {
                OutlinedTextField(
                    value = server,
                    onValueChange = { server = it },
                    label = { Text("Server URL") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(8.dp))
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Topic name") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onAdd(server, name) },
                enabled = server.isNotBlank() && name.isNotBlank(),
            ) {
                Text("Subscribe")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}
