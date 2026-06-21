package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.nsfy.app.NsfyApp

data class ServerItem(val url: String, val name: String)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen() {
    val prefs = NsfyApp.instance.getSharedPreferences(
        "nsfy_prefs", android.content.Context.MODE_PRIVATE
    )
    val initialServers = remember {
        val urls = prefs.getStringSet("servers", emptySet()) ?: emptySet()
        urls.map { url ->
            val name = prefs.getString("server_name_$url", url) ?: url
            ServerItem(url, name)
        }
    }
    var servers by remember { mutableStateOf(initialServers) }
    var showAddDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings", fontWeight = FontWeight.Bold) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                "Servers",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.primary,
            )

            for (server in servers) {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
                    ),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(12.dp),
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(server.name, style = MaterialTheme.typography.titleSmall)
                            Text(
                                server.url,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        TextButton(onClick = {
                            servers = servers.filter { it.url != server.url }
                            prefs.edit().apply {
                                val urls = servers.map { it.url }.toSet()
                                putStringSet("servers", urls)
                                for (s in servers) {
                                    putString("server_name_${s.url}", s.name)
                                }
                                apply()
                            }
                        }) {
                            Text(
                                "✕",
                                color = MaterialTheme.colorScheme.error,
                            )
                        }
                    }
                }
            }

            OutlinedButton(
                onClick = { showAddDialog = true },
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("＋ Add Server")
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            Text(
                "About",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.primary,
            )
            Text(
                "nsfy Android v0.1.0",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "A minimal, high-performance pub-sub notification system.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
            )
        }
    }

    if (showAddDialog) {
        var newUrl by remember { mutableStateOf("http://") }
        var newName by remember { mutableStateOf("") }

        AlertDialog(
            onDismissRequest = { showAddDialog = false },
            title = { Text("Add Server") },
            text = {
                Column {
                    OutlinedTextField(
                        value = newUrl,
                        onValueChange = { newUrl = it },
                        label = { Text("Server URL") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = newName,
                        onValueChange = { newName = it },
                        label = { Text("Display name") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        servers = servers + ServerItem(newUrl, newName)
                        prefs.edit().apply {
                            val urls = servers.map { it.url }.toSet()
                            putStringSet("servers", urls)
                            for (s in servers) {
                                putString("server_name_${s.url}", s.name)
                            }
                            apply()
                        }
                        showAddDialog = false
                    },
                    enabled = newUrl.isNotBlank() && newName.isNotBlank(),
                ) {
                    Text("Add")
                }
            },
            dismissButton = {
                TextButton(onClick = { showAddDialog = false }) { Text("Cancel") }
            },
        )
    }
}
