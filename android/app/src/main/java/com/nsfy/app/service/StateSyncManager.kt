package com.nsfy.app.service

import android.content.Context
import android.util.Log
import com.nsfy.app.data.model.MessageStateEntity
import com.nsfy.app.data.model.authenticated
import com.nsfy.app.data.model.normalizeServerUrl
import com.nsfy.app.data.model.parseStateUpdates
import com.nsfy.app.data.model.stateUpdateBody
import com.nsfy.app.data.repository.NsfyRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.io.IOException

class StateSyncManager(
    context: Context,
    private val client: OkHttpClient,
    private val repository: NsfyRepository,
    private val scope: CoroutineScope,
) {
    private val prefs = context.getSharedPreferences("nsfy_prefs", Context.MODE_PRIVATE)
    private val sockets = mutableMapOf<String, WebSocket>()
    private val flushMutex = Mutex()

    fun retain(topics: Set<String>) {
        sockets.keys.filterNot(topics::contains).forEach { key ->
            sockets.remove(key)?.close(1000, "topic unsubscribed")
        }
    }

    fun connect(serverUrl: String, topicName: String) {
        val key = "$serverUrl/$topicName"
        if (sockets.containsKey(key)) return
        val base = try { normalizeServerUrl(serverUrl) } catch (_: IllegalArgumentException) { return }
        val url = base.replace("http://", "ws://").replace("https://", "wss://") +
            "/$topicName/state/ws"
        val request = authenticated(Request.Builder().url(url), base, prefs).build()
        sockets[key] = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                scope.launch { flush() }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val updates = parseStateUpdates(text)
                    scope.launch {
                        repository.applyRemote(NsfyRepository.topicId(serverUrl, topicName), updates)
                    }
                } catch (error: Exception) {
                    Log.i("nsfy", "Ignored invalid state frame: ${error.message}")
                }
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) = reconnect(key, serverUrl, topicName)
            override fun onFailure(webSocket: WebSocket, error: Throwable, response: Response?) =
                reconnect(key, serverUrl, topicName)
        })
    }

    fun flushSoon() {
        scope.launch { flush() }
    }

    suspend fun flush() = flushMutex.withLock {
        val pending = repository.getPendingStatesOnce()
        for (states in pending.groupBy(MessageStateEntity::topicId).values) send(states)
    }

    fun close() {
        val open = sockets.values.toList()
        sockets.clear()
        open.forEach { it.close(1000, "service stopped") }
    }

    private fun reconnect(key: String, serverUrl: String, topicName: String) {
        scope.launch {
            delay(5000)
            if (sockets.remove(key) != null) connect(serverUrl, topicName)
        }
    }

    private suspend fun send(states: List<MessageStateEntity>) {
        val topic = repository.getTopic(states.first().topicId) ?: return
        val body = stateUpdateBody(states)
            .toRequestBody("application/json; charset=utf-8".toMediaType())
        val request = authenticated(
            Request.Builder().url("${topic.serverUrl}/${topic.name}/state"), topic.serverUrl, prefs,
        ).post(body).build()
        val success = kotlinx.coroutines.suspendCancellableCoroutine { continuation ->
            val call = client.newCall(request)
            continuation.invokeOnCancellation { call.cancel() }
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, error: IOException) = continuation.resume(false) {}
                override fun onResponse(call: Call, response: Response) {
                    response.use { continuation.resume(it.isSuccessful) {} }
                }
            })
        }
        if (success) repository.markStatesSent(states.map(MessageStateEntity::key))
    }
}
