package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.nsfy.app.NsfyApp
import com.nsfy.app.data.model.normalizeServerUrl

data class ServerItem(val url: String, val name: String)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(onLayoutChange: (String) -> Unit = {}) {
    val prefs = NsfyApp.instance.getSharedPreferences(
        PREFS_NAME, android.content.Context.MODE_PRIVATE
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
    var layoutMode by remember {
        mutableStateOf(prefs.getString(KEY_LAYOUT_MODE, "split") ?: "split")
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("设置", fontWeight = FontWeight.Bold) },
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
                "布局",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.primary,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(
                    selected = layoutMode == "split",
                    onClick = {
                        layoutMode = "split"
                        onLayoutChange("split")
                    },
                    label = { Text("分栏排版") },
                )
                FilterChip(
                    selected = layoutMode == "timeline",
                    onClick = {
                        layoutMode = "timeline"
                        onLayoutChange("timeline")
                    },
                    label = { Text("统一时间线") },
                )
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))

            Text(
                "服务器",
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
                            if (!prefs.getString("server_token_${server.url}", null).isNullOrBlank()) {
                                Text(
                                    "已配置令牌",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
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
                Text("＋ 添加服务器")
            }

            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

            Text(
                "关于信鸽",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.primary,
            )
            Text(
                "信鸽 Android v0.1.0",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "订阅主题，接收服务器推送。",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
            )
        }
    }

    if (showAddDialog) {
        var newUrl by remember { mutableStateOf("https://") }
        var newName by remember { mutableStateOf("") }
        var newToken by remember { mutableStateOf("") }
        var addError by remember { mutableStateOf("") }

        AlertDialog(
            onDismissRequest = { showAddDialog = false },
            title = { Text("添加服务器") },
            text = {
                Column {
                    OutlinedTextField(
                        value = newUrl,
                        onValueChange = { newUrl = it },
                        label = { Text("服务器地址") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    if (addError.isNotEmpty()) {
                        Text(addError, color = MaterialTheme.colorScheme.error)
                    }
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = newName,
                        onValueChange = { newName = it },
                        label = { Text("显示名称") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = newToken,
                        onValueChange = { newToken = it },
                        label = { Text("访问令牌（可选）") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        val normalized = try {
                            normalizeServerUrl(newUrl)
                        } catch (error: IllegalArgumentException) {
                            addError = error.message ?: "服务器地址无效"
                            return@TextButton
                        }
                        servers = servers + ServerItem(normalized, newName)
                        prefs.edit().apply {
                            val urls = servers.map { it.url }.toSet()
                            putStringSet("servers", urls)
                            for (s in servers) {
                                putString("server_name_${s.url}", s.name)
                            }
                            if (newToken.isNotBlank()) {
                                putString("server_token_$normalized", newToken.trim())
                            }
                            apply()
                        }
                        showAddDialog = false
                    },
                    enabled = newUrl.isNotBlank() && newName.isNotBlank(),
                ) {
                    Text("添加")
                }
            },
            dismissButton = {
                TextButton(onClick = { showAddDialog = false }) { Text("取消") }
            },
        )
    }
}
