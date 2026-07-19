package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import com.nsfy.app.data.model.normalizeServerUrl

private val PRIORITY_OPTIONS = listOf(
    5 to "紧急",
    4 to "高",
    3 to "普通",
    1 to "低",
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PublishScreen() {
    val context = LocalContext.current
    var serverUrl by remember { mutableStateOf("https://") }
    var topicName by remember { mutableStateOf("") }
    var title by remember { mutableStateOf("") }
    var message by remember { mutableStateOf("") }
    var categoryPath by remember { mutableStateOf("") }
    var priority by remember { mutableIntStateOf(3) }
    var status by remember { mutableStateOf<String?>(null) }
    val client = remember { OkHttpClient() }
    val jsonMediaType = "application/json; charset=utf-8".toMediaType()

    fun publish() {
        if (message.isBlank() || serverUrl.isBlank()) return
        val normalized = try {
            normalizeServerUrl(serverUrl)
        } catch (error: IllegalArgumentException) {
            status = error.message ?: "服务器地址无效"
            return
        }
        val t = topicName.ifBlank { "default" }
        val body = JSONObject().apply {
            put("title", title)
            put("message", message)
            put("priority", priority)
            put("tags", org.json.JSONArray())
            put(
                "category",
                org.json.JSONArray(categoryPath.split('/').map { it.trim() }.filter { it.isNotEmpty() }),
            )
        }
        val prefs = context.getSharedPreferences(PREFS_NAME, android.content.Context.MODE_PRIVATE)
        val request = com.nsfy.app.data.model.authenticated(
            Request.Builder().url("$normalized/$t"), normalized, prefs,
        ).post(body.toString().toRequestBody(jsonMediaType)).build()
        status = "发布中…"
        client.newCall(request).enqueue(object : Callback {
            override fun onFailure(call: Call, e: java.io.IOException) {
                status = "发布失败:${e.message}"
            }
            override fun onResponse(call: Call, response: Response) {
                status = if (response.isSuccessful) "已发布" else "发布失败:${response.code}"
                if (response.isSuccessful) {
                    message = ""
                    title = ""
                    categoryPath = ""
                }
            }
        })
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("发布消息", fontWeight = FontWeight.Bold) },
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
                label = { Text("服务器地址") },
                placeholder = { Text("https://your-server.example") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = topicName,
                onValueChange = { topicName = it },
                label = { Text("主题") },
                placeholder = { Text("主题名") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = title,
                onValueChange = { title = it },
                label = { Text("标题") },
                placeholder = { Text("一句话说明发生了什么") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = message,
                onValueChange = { message = it },
                label = { Text("内容") },
                placeholder = { Text("磁盘清理脚本已执行，/var 回落至 71%…") },
                modifier = Modifier.fillMaxWidth(),
                minLines = 3,
            )
            OutlinedTextField(
                value = categoryPath,
                onValueChange = { categoryPath = it },
                label = { Text("多级分类") },
                placeholder = { Text("工作/Agent/Codex") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Text("优先级", style = MaterialTheme.typography.labelMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PRIORITY_OPTIONS.forEach { (value, label) ->
                    FilterChip(
                        selected = priority == value,
                        onClick = { priority = value },
                        label = { Text(label) },
                    )
                }
            }
            Button(
                onClick = { publish() },
                modifier = Modifier.fillMaxWidth(),
                enabled = message.isNotBlank() && serverUrl.isNotBlank(),
            ) {
                Text("发布")
            }
            status?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodyMedium,
                    color = when {
                        it == "已发布" -> MaterialTheme.colorScheme.primary
                        it.startsWith("发布失败") -> MaterialTheme.colorScheme.error
                        else -> MaterialTheme.colorScheme.onSurfaceVariant
                    },
                )
            }
        }
    }
}
