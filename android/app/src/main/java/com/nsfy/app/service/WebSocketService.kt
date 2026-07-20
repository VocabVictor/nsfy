package com.nsfy.app.service

import android.app.*
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import com.nsfy.app.data.model.*
import com.nsfy.app.data.repository.NsfyRepository
import kotlinx.coroutines.*
import com.nsfy.app.data.db.AppDatabase
import okhttp3.*
import org.json.JSONObject

class WebSocketService : Service() {

    private lateinit var wakeLock: PowerManager.WakeLock
    private val connections = mutableMapOf<String, WebSocket>()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private lateinit var okHttp: OkHttpClient
    private lateinit var repository: NsfyRepository
    private var servers: List<Pair<String, String>> = emptyList() // url -> name
    private var subscriptionJob: Job? = null
    private var stateSyncJob: Job? = null
    private lateinit var stateSync: StateSyncManager

    override fun onCreate() {
        super.onCreate()
        val db = com.nsfy.app.data.db.AppDatabase.getInstance(this)
        repository = NsfyRepository(db)
        val prefs = getSharedPreferences("nsfy_prefs", MODE_PRIVATE)
        okHttp = nsfyHttpClient(prefs, websocket = true)
        stateSync = StateSyncManager(this, okHttp, repository, scope)
        acquireWakeLock()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        android.util.Log.i("nsfy", "WebSocketService.onStartCommand called")
        val notification = buildNotification("nsfy running", "Waiting for connections...")
        startForeground(NOTIFICATION_ID, notification)

        subscriptionJob?.cancel()
        stateSyncJob?.cancel()
        stateSyncJob = scope.launch {
            repository.getPendingStates().collect { if (it.isNotEmpty()) stateSync.flushSoon() }
        }
        scope.launch {
            val settings = loadMobilePreferences(getSharedPreferences("nsfy_prefs", MODE_PRIVATE))
            repository.prune(settings.retentionDays, settings.trashRetentionDays)
        }
        subscriptionJob = scope.launch {
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
                val wanted = topicList.mapTo(mutableSetOf()) { "${it.serverUrl}/${it.name}" }
                connections.keys.filterNot { it in wanted }.forEach { key ->
                    connections.remove(key)?.close(1000, "topic unsubscribed")
                }
                stateSync.retain(wanted)
                for (topic in topicList) {
                    val key = "${topic.serverUrl}/${topic.name}"
                    android.util.Log.i("nsfy", "WebSocketService: topic ${topic.name} @ ${topic.serverUrl}, connected=${connections.containsKey(key)}")
                    if (!connections.containsKey(key)) {
                        android.util.Log.i("nsfy", "WebSocketService: connecting to ${topic.name}")
                        connectWs(topic.serverUrl, topic.name)
                    }
                    stateSync.connect(topic.serverUrl, topic.name)
                }
            }
        }

        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun connectWs(serverUrl: String, topicName: String) {
        val prefs = getSharedPreferences("nsfy_prefs", MODE_PRIVATE)
        val base = try {
            normalizeServerUrl(serverUrl)
        } catch (error: IllegalArgumentException) {
            android.util.Log.e("nsfy", "Blocked insecure server URL: ${error.message}")
            return
        }
        val wsUrl = base
            .replace("http://", "ws://")
            .replace("https://", "wss://") + "/$topicName/ws"
        val key = "$serverUrl/$topicName"
        android.util.Log.i("nsfy", "WebSocketService: connectWs to $topicName on $serverUrl")

        val request = com.nsfy.app.data.model.authenticated(
            Request.Builder().url(wsUrl), base, prefs,
        ).build()
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
                        category = json.optJSONArray("category")?.let { arr ->
                            (0 until arr.length()).map { arr.getString(it) }
                        } ?: emptyList(),
                        popup = if (json.has("popup")) json.optBoolean("popup")
                            else json.optInt("priority", 3) >= 4,
                        bypassDnd = json.optBoolean("bypassDnd", false),
                    )
                    scope.launch {
                        repository.saveMessage(serverUrl, topicName, msg)
                    }
                    val prefs = getSharedPreferences("nsfy_prefs", MODE_PRIVATE)
                    val settings = loadMobilePreferences(prefs)
                    val rule = settings.topicRules[topicRuleKey(serverUrl, topicName)] ?: TopicRule()
                    val allowed = prefs.getStringSet(KEY_DND_PRIORITIES, emptySet())
                        .orEmpty().mapNotNull(String::toIntOrNull).toSet()
                    val policy = msg.copy(bypassDnd = msg.bypassDnd || rule.bypassDnd)
                    val permitted = rule.mode != "mute" && (rule.mode != "high" || msg.priority >= 4)
                    if (permitted && shouldPresentNotification(
                            policy, prefs.getBoolean(KEY_DO_NOT_DISTURB, false) || scheduledDnd(settings), allowed,
                        )) {
                        showNotification(
                            topicName,
                            msg.title.ifEmpty { msg.message },
                            if (settings.showPreview) msg.message else "有一条新消息",
                            msg.priority,
                            NotificationMode.from(prefs.getString(KEY_NOTIFICATION_MODE, null)),
                            settings,
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

    private fun showNotification(
        topic: String, title: String, body: String, priority: Int, mode: NotificationMode,
        settings: MobilePreferences,
    ) {
        if (mode == NotificationMode.Silent) return
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val headsUp = mode == NotificationMode.Temporary || mode == NotificationMode.Persistent
        val channelId = when {
            !settings.soundEnabled && (headsUp || priority >= 5) -> CHANNEL_SILENT_URGENT
            !settings.soundEnabled -> CHANNEL_SILENT
            priority >= 5 && settings.urgentSoundEnabled -> CHANNEL_URGENT
            else -> CHANNEL_DEFAULT
        }
        val builder = NotificationCompat.Builder(this, channelId)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(title.ifEmpty { topic })
            .setContentText(body)
            .setPriority(if (headsUp || priority >= 5) NotificationCompat.PRIORITY_HIGH else NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
        if (mode == NotificationMode.Temporary) builder.setTimeoutAfter(5000)
        nm.notify(System.currentTimeMillis().toInt(), builder.build())
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
            NotificationChannel(CHANNEL_SILENT, "Silent", NotificationManager.IMPORTANCE_LOW).apply {
                setSound(null, null)
            },
            NotificationChannel(CHANNEL_SILENT_URGENT, "Silent alerts", NotificationManager.IMPORTANCE_HIGH).apply {
                setSound(null, null)
            },
        ).forEach { nm.createNotificationChannel(it) }
    }

    private fun acquireWakeLock() {
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "nsfy:wakelock")
        wakeLock.acquire(10 * 60 * 1000L)
    }

    override fun onDestroy() {
        subscriptionJob?.cancel()
        stateSyncJob?.cancel()
        connections.values.forEach { it.close(1000, "service stopped") }
        connections.clear()
        stateSync.close()
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
        private const val CHANNEL_SILENT = "nsfy_silent"
        private const val CHANNEL_SILENT_URGENT = "nsfy_silent_urgent"
    }
}
