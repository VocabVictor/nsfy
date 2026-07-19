package com.nsfy.app.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.nsfy.app.data.model.*
import com.nsfy.app.data.repository.NsfyRepository
import com.nsfy.app.NsfyApp
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.IOException

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopicDetailScreen(
    topic: TopicEntity,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val db = NsfyApp.instance.database
    val repo = remember { NsfyRepository(db) }
    val topicId = NsfyRepository.topicId(topic.serverUrl, topic.name)
    val messages by repo.getMessages(topicId).collectAsState(initial = emptyList())

    var replyText by remember { mutableStateOf("") }
    var replyTitle by remember { mutableStateOf("") }
    var replyCategory by remember { mutableStateOf("") }
    val listState = rememberLazyListState()
    val client = remember { OkHttpClient() }
    val jsonMediaType = "application/json; charset=utf-8".toMediaType()

    LaunchedEffect(messages.size) {
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(0)
        }
    }

    fun publish() {
        if (replyText.isBlank()) return
        val body = JSONObject().apply {
            put("title", replyTitle)
            put("message", replyText)
            put("priority", 3)
            put(
                "category",
                org.json.JSONArray(replyCategory.split('/').map { it.trim() }.filter { it.isNotEmpty() }),
            )
        }
        val prefs = NsfyApp.instance.getSharedPreferences(
            PREFS_NAME, android.content.Context.MODE_PRIVATE
        )
        val request = com.nsfy.app.data.model.authenticated(
            Request.Builder().url("${topic.serverUrl}/${topic.name}"), topic.serverUrl, prefs,
        ).post(body.toString().toRequestBody(jsonMediaType)).build()
        client.newCall(request).enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) {}
            override fun onResponse(call: Call, response: Response) {
                replyText = ""
                replyTitle = ""
            }
        })
    }

    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(topic.name, fontWeight = FontWeight.Bold)
                        Text(
                            topic.serverUrl,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                },
                navigationIcon = {
                    TextButton(onClick = onBack) {
                        Text("←", fontSize = MaterialTheme.typography.titleLarge.fontSize)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
        bottomBar = {
            Surface(tonalElevation = 1.dp, shadowElevation = 4.dp) {
                Column(modifier = Modifier.padding(12.dp)) {
                    OutlinedTextField(
                        value = replyTitle,
                        onValueChange = { replyTitle = it },
                        placeholder = { Text("标题（可选）") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    OutlinedTextField(
                        value = replyCategory,
                        onValueChange = { replyCategory = it },
                        placeholder = { Text("分类，如 工作/Agent") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        OutlinedTextField(
                            value = replyText,
                            onValueChange = { replyText = it },
                            placeholder = { Text("发送通知…") },
                            modifier = Modifier.weight(1f),
                            singleLine = true,
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        TextButton(
                            onClick = { publish() },
                            enabled = replyText.isNotBlank(),
                        ) {
                            Text(
                                "↑",
                                fontSize = MaterialTheme.typography.titleLarge.fontSize,
                                color = if (replyText.isNotBlank())
                                    MaterialTheme.colorScheme.primary
                                else
                                    MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.3f),
                            )
                        }
                    }
                }
            }
        },
    ) { padding ->
        if (messages.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("暂无消息", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        "在下方发布，或等待服务器推送",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                    )
                }
            }
        } else {
            LazyColumn(
                state = listState,
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
                reverseLayout = true,
            ) {
                items(messages.reversed()) { msg ->
                    MessageCard(msg)
                }
            }
        }
    }
}

@Composable
fun MessageCard(msg: MessageEntity) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f),
        ),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
            if (msg.title.isNotEmpty()) {
                    Text(
                        msg.title,
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
                Row {
                    if (msg.priority >= 4) {
                        Text(
                            priorityLabel(msg.priority),
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.SemiBold,
                            color = com.nsfy.app.ui.theme.priorityColor(msg.priority),
                        )
                    }
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        fmtTime(msg.time),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
                    )
                }
            }
            val category = categoryPath(msg.category)
            if (category.isNotEmpty()) {
                Spacer(modifier = Modifier.height(3.dp))
                Text(
                    category,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                msg.message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface,
            )
            if (msg.tags.isNotEmpty()) {
                Spacer(modifier = Modifier.height(6.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    msg.tags.split(",").filter { it.isNotBlank() }.forEach { tag ->
                        SuggestionChip(
                            onClick = {},
                            label = { Text(tag, style = MaterialTheme.typography.labelSmall) },
                            modifier = Modifier.height(24.dp),
                        )
                    }
                }
            }
        }
    }
}
