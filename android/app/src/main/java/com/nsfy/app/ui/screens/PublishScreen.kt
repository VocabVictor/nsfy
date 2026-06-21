package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import org.json.JSONObject

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PublishScreen() {
    var serverUrl by remember { mutableStateOf("http://") }
    var topicName by remember { mutableStateOf("") }
    var title by remember { mutableStateOf("") }
    var message by remember { mutableStateOf("") }
    var priority by remember { mutableIntStateOf(3) }
    var tags by remember { mutableStateOf("") }
    var status by remember { mutableStateOf<String?>(null) }
    val client = remember { OkHttpClient() }
    val jsonMediaType = "application/json; charset=utf-8".toMediaType()

    fun publish() {
        if (message.isBlank() || serverUrl.isBlank()) return
        val t = topicName.ifBlank { "default" }
        val body = JSONObject().apply {
            put("title", title)
            put("message", message)
            put("priority", priority)
            put("tags", org.json.JSONArray(
                tags.split(",").map { it.trim() }.filter { it.isNotEmpty() }
            ))
        }
        val request = Request.Builder()
            .url("$serverUrl/$t")
            .post(RequestBody.create(jsonMediaType, body.toString()))
            .build()
        status = "Sending..."
        client.newCall(request).enqueue(object : Callback {
            override fun onFailure(call: Call, e: java.io.IOException) {
                status = "Error: ${e.message}"
            }
            override fun onResponse(call: Call, response: Response) {
                status = if (response.isSuccessful) "✓ Sent!" else "Failed: ${response.code}"
                if (response.isSuccessful) {
                    message = ""
                    title = ""
                    tags = ""
                }
            }
        })
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Publish", fontWeight = FontWeight.Bold) },
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
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            OutlinedTextField(
                value = serverUrl,
                onValueChange = { serverUrl = it },
                label = { Text("Server URL") },
                placeholder = { Text("http://your-server:8080") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = topicName,
                onValueChange = { topicName = it },
                label = { Text("Topic") },
                placeholder = { Text("topic name") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = title,
                onValueChange = { title = it },
                label = { Text("Title (optional)") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = message,
                onValueChange = { message = it },
                label = { Text("Message") },
                modifier = Modifier.fillMaxWidth(),
                minLines = 3,
            )
            Text("Priority", style = MaterialTheme.typography.labelMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                (1..5).forEach { p ->
                    FilterChip(
                        selected = priority == p,
                        onClick = { priority = p },
                        label = { Text("$p") },
                    )
                }
            }
            OutlinedTextField(
                value = tags,
                onValueChange = { tags = it },
                label = { Text("Tags (comma-separated)") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Button(
                onClick = { publish() },
                modifier = Modifier.fillMaxWidth(),
                enabled = message.isNotBlank() && serverUrl.isNotBlank(),
            ) {
                Text("Send Notification")
            }
            status?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodyMedium,
                    color = when {
                        it.startsWith("✓") -> MaterialTheme.colorScheme.primary
                        it.startsWith("Error") -> MaterialTheme.colorScheme.error
                        else -> MaterialTheme.colorScheme.onSurfaceVariant
                    },
                )
            }
        }
    }
}
