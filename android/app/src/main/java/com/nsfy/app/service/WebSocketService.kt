package com.nsfy.app.service

import android.app.*
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import com.nsfy.app.data.model.NsfyMessage
import com.nsfy.app.data.model.PublishRequest
import com.nsfy.app.data.repository.NsfyRepository
import kotlinx.coroutines.*
import com.nsfy.app.data.db.AppDatabase
import okhttp3.*
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class WebSocketService : Service() {

    private lateinit var wakeLock: PowerManager.WakeLock
    private val connections = mutableMapOf<String, WebSocket>()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private lateinit var okHttp: OkHttpClient
    private lateinit var repository: NsfyRepository
    private var servers: List<Pair<String, String>> = emptyList() // url -> name

    override fun onCreate() {
        super.onCreate()
        val db = com.nsfy.app.data.db.AppDatabase.getInstance(this)
        repository = NsfyRepository(db)
        okHttp = OkHttpClient.Builder()
            .connectTimeout(10, TimeUnit.SECONDS)
            .readTimeout(0, TimeUnit.MILLISECONDS) // no read timeout for WS
            .build()
        acquireWakeLock()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        android.util.Log.i("nsfy", "WebSocketService.onStartCommand called")
        val notification = buildNotification("nsfy running", "Waiting for connections...")
        startForeground(NOTIFICATION_ID, notification)

        scope.launch {
            // Ensure at least one default topic exists
            try {
                val db = AppDatabase.getInstance(this@WebSocketService)
                val count = db.topicDao().getTopicCount()
                android.util.Log.i("nsfy", "WebSocketService: topic count = $count")
                if (count == 0) {
                    android.util.Log.i("nsfy", "WebSocketService: auto-adding test topic")
                    repository.addTopic("http://localhost:8419", "test")
                }
            } catch (e: Exception) {
                android.util.Log.e("nsfy", "auto-add topic failed: ${e.message}", e)
            }

            // Load servers and connect
            android.util.Log.i("nsfy", "WebSocketService: loading servers from prefs")
            val prefs = getSharedPreferences("nsfy_prefs", MODE_PRIVATE)
            val serverUrls = prefs.getStringSet("servers", emptySet()) ?: emptySet()
            servers = if (serverUrls.isEmpty()) {
                listOf("http://localhost:8419" to "Local")
            } else {
                serverUrls.map { url ->
                    url to (prefs.getString("server_name_$url", url) ?: url)
                }
            }

            // Connect to all subscribed topics
            android.util.Log.i("nsfy", "WebSocketService: starting topic collection")
            repository.getAllTopics().collect { topicList ->
                android.util.Log.i("nsfy", "WebSocketService: got ${topicList.size} topics")
                for (topic in topicList) {
                    val key = "${topic.serverUrl}/${topic.name}"
                    android.util.Log.i("nsfy", "WebSocketService: topic ${topic.name} @ ${topic.serverUrl}, connected=${connections.containsKey(key)}")
                    if (!connections.containsKey(key)) {
                        android.util.Log.i("nsfy", "WebSocketService: connecting to ${topic.name}")
                        connectWs(topic.serverUrl, topic.name)
                    }
                }
            }
        }

        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun connectWs(serverUrl: String, topicName: String) {
        val wsUrl = serverUrl
            .replace("http://", "ws://")
            .replace("https://", "wss://") + "/$topicName/ws"
        val key = "$serverUrl/$topicName"
        android.util.Log.i("nsfy", "WebSocketService: connectWs to $wsUrl")

        val request = Request.Builder().url(wsUrl).build()
        val ws = okHttp.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                android.util.Log.i("nsfy", "WS OPEN: $topicName on $serverUrl")
                scope.launch {
                    repository.setTopicConnected(serverUrl, topicName, true)
                    updateNotification()
                }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val json = JSONObject(text)
                    val msg = NsfyMessage(
                        id = json.getString("id"),
                        time = json.getLong("time"),
                        title = json.optString("title", ""),
                        message = json.optString("message", ""),
                        priority = json.optInt("priority", 3),
                        tags = json.optJSONArray("tags")?.let { arr ->
                            (0 until arr.length()).map { arr.getString(it) }
                        } ?: emptyList(),
                    )
                    scope.launch {
                        repository.saveMessage(serverUrl, topicName, msg)
                    }
                    if (msg.priority >= 4) {
                        showNotification(
                            topicName,
                            msg.title.ifEmpty { msg.message },
                            msg.message,
                            msg.priority,
                        )
                    }
                } catch (e: Exception) {
                    // silently skip bad messages
                }
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                webSocket.close(1000, null)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                // Reconnect on the coroutine scope, not this thread — this
                // callback runs on OkHttp's own dispatcher pool, and a
                // blocking Thread.sleep here ties up one of its worker
                // threads for the whole delay on every single reconnect.
                scope.launch {
                    repository.setTopicConnected(serverUrl, topicName, false)
                    updateNotification()
                    delay(5000)
                    if (connections.containsKey(key)) {
                        connectWs(serverUrl, topicName)
                    }
                }
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                android.util.Log.e("nsfy", "WS FAILURE: $topicName on $serverUrl: ${t.message}", t)
                scope.launch {
                    repository.setTopicConnected(serverUrl, topicName, false)
                    updateNotification()
                    delay(10000)
                    if (connections.containsKey(key)) {
                        connectWs(serverUrl, topicName)
                    }
                }
            }
        })

        connections[key] = ws
    }

    fun disconnectTopic(serverUrl: String, topicName: String) {
        val key = "$serverUrl/$topicName"
        connections.remove(key)?.close(1000, "user disconnected")
    }

    private suspend fun updateNotification() {
        val connected = connections.count { (_, ws) -> true } // rough count
        val notification = buildNotification(
            "nsfy",
            "$connected topic(s) connected"
        )
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
    }

    private fun showNotification(topic: String, title: String, body: String, priority: Int) {
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val channelId = if (priority >= 5) CHANNEL_URGENT else CHANNEL_DEFAULT
        val n = NotificationCompat.Builder(this, channelId)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(title.ifEmpty { topic })
            .setContentText(body)
            .setPriority(if (priority >= 5) NotificationCompat.PRIORITY_HIGH else NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
            .build()
        nm.notify(System.currentTimeMillis().toInt(), n)
    }

    private fun buildNotification(title: String, content: String): Notification {
        return NotificationCompat.Builder(this, CHANNEL_SERVICE)
            .setContentTitle(title)
            .setContentText(content)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun createNotificationChannel() {
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        listOf(
            NotificationChannel(CHANNEL_SERVICE, "Service", NotificationManager.IMPORTANCE_LOW),
            NotificationChannel(CHANNEL_DEFAULT, "Notifications", NotificationManager.IMPORTANCE_DEFAULT),
            NotificationChannel(CHANNEL_URGENT, "Urgent", NotificationManager.IMPORTANCE_HIGH),
        ).forEach { nm.createNotificationChannel(it) }
    }

    private fun acquireWakeLock() {
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "nsfy:wakelock")
        wakeLock.acquire(10 * 60 * 1000L)
    }

    override fun onDestroy() {
        connections.values.forEach { it.close(1000, "service stopped") }
        connections.clear()
        scope.cancel()
        if (::wakeLock.isInitialized && wakeLock.isHeld) {
            wakeLock.release()
        }
        super.onDestroy()
    }

    companion object {
        private const val NOTIFICATION_ID = 1001
        private const val CHANNEL_SERVICE = "nsfy_service"
        private const val CHANNEL_DEFAULT = "nsfy_notifications"
        private const val CHANNEL_URGENT = "nsfy_urgent"
    }
}
