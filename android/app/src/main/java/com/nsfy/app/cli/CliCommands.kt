package com.nsfy.app.cli

import android.content.Context
import android.content.Intent
import android.os.Build
import com.nsfy.app.data.db.AppDatabase
import com.nsfy.app.data.model.authenticated
import com.nsfy.app.data.model.normalizeServerUrl
import com.nsfy.app.data.model.nsfyHttpClient
import com.nsfy.app.data.repository.NsfyRepository
import com.nsfy.app.service.WebSocketService
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject

internal class CliCommands(private val context: Context) {
    private val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    private val repository = NsfyRepository(AppDatabase.getInstance(context))
    private val topicDao = AppDatabase.getInstance(context).topicDao()
    private val client = nsfyHttpClient(prefs)

    suspend fun execute(intent: Intent): String = when (required(intent, "command")) {
        "server-list" -> serverList()
        "server-add" -> serverAdd(intent)
        "server-remove" -> serverRemove(intent)
        "topic-list" -> topicList()
        "topic-add" -> topicAdd(intent)
        "topic-remove" -> topicRemove(intent)
        "publish" -> publish(intent)
        "poll" -> poll(intent)
        "status" -> status(intent)
        "service-start" -> serviceStart()
        "service-stop" -> serviceStop()
        else -> throw IllegalArgumentException("unknown command")
    }

    private fun serverList(): String {
        val items = JSONArray()
        servers().forEach { server ->
            items.put(
                JSONObject()
                    .put("name", server.name)
                    .put("url", server.url)
                    .put("tokenConfigured", !prefs.getString("server_token_${server.url}", null).isNullOrBlank()),
            )
        }
        return JSONObject().put("servers", items).toString()
    }

    private fun serverAdd(intent: Intent): String {
        val url = normalizeServerUrl(required(intent, "url"))
        val name = required(intent, "name").trim()
        val token = intent.getStringExtra("token")?.trim()
        val urls = (prefs.getStringSet("servers", emptySet()) ?: emptySet()).toMutableSet()
        urls.add(url)
        prefs.edit().apply {
            putStringSet("servers", urls)
            putString("server_name_$url", name)
            if (token.isNullOrEmpty()) remove("server_token_$url") else putString("server_token_$url", token)
            apply()
        }
        return ok("server", JSONObject().put("name", name).put("url", url))
    }

    private suspend fun serverRemove(intent: Intent): String {
        val server = resolveServer(required(intent, "server"))
        val urls = (prefs.getStringSet("servers", emptySet()) ?: emptySet()).toMutableSet()
        urls.remove(server.url)
        prefs.edit()
            .putStringSet("servers", urls)
            .remove("server_name_${server.url}")
            .remove("server_token_${server.url}")
            .apply()
        topicDao.getAllTopicsOnce()
            .filter { it.serverUrl == server.url }
            .forEach { repository.deleteTopic(it.serverUrl, it.name) }
        return ok("removed", server.url)
    }

    private suspend fun topicList(): String {
        val items = JSONArray()
        topicDao.getAllTopicsOnce().forEach { topic ->
            items.put(JSONObject().put("server", topic.serverUrl).put("topic", topic.name))
        }
        return JSONObject().put("topics", items).toString()
    }

    private suspend fun topicAdd(intent: Intent): String {
        val server = resolveServer(required(intent, "server"))
        val topic = validateTopic(required(intent, "topic"))
        repository.addTopic(server.url, topic)
        startService()
        return ok("subscribed", JSONObject().put("server", server.url).put("topic", topic))
    }

    private suspend fun topicRemove(intent: Intent): String {
        val server = resolveServer(required(intent, "server"))
        val topic = validateTopic(required(intent, "topic"))
        repository.deleteTopic(server.url, topic)
        return ok("unsubscribed", JSONObject().put("server", server.url).put("topic", topic))
    }

    private fun publish(intent: Intent): String {
        val server = resolveServer(intent.getStringExtra("server"))
        val topic = validateTopic(required(intent, "topic"))
        val message = required(intent, "message")
        val priority = intent.getStringExtra("priority")?.toIntOrNull() ?: 3
        require(priority in 1..5) { "priority must be between 1 and 5" }
        val tags = split(intent.getStringExtra("tags"), ',')
        val category = split(intent.getStringExtra("category"), '/')
        require(category.size <= 8) { "category is too deep" }
        val body = JSONObject()
            .put("title", intent.getStringExtra("title") ?: "")
            .put("message", message)
            .put("priority", priority)
            .put("popup", intent.getBooleanExtra("popup", false))
            .put("bypassDnd", intent.getBooleanExtra("bypassDnd", false))
            .put("tags", JSONArray(tags))
            .put("category", JSONArray(category))
        val request = authRequest("${server.url}/$topic", server, intent)
            .post(body.toString().toRequestBody(JSON)).build()
        return response(request)
    }

    private fun poll(intent: Intent): String {
        val server = resolveServer(intent.getStringExtra("server"))
        val topic = validateTopic(required(intent, "topic"))
        val since = intent.getStringExtra("since")
        val suffix = if (since.isNullOrBlank()) "" else "?since=$since"
        return response(authRequest("${server.url}/$topic/json$suffix", server, intent).get().build())
    }

    private fun status(intent: Intent): String {
        val server = resolveServer(intent.getStringExtra("server"))
        return response(authRequest("${server.url}/", server, intent).get().build())
    }

    private fun serviceStart(): String {
        startService()
        return ok("service", "started")
    }

    private fun serviceStop(): String {
        context.stopService(Intent(context, WebSocketService::class.java))
        return ok("service", "stopped")
    }

    private fun startService() {
        val service = Intent(context, WebSocketService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) context.startForegroundService(service)
        else context.startService(service)
    }

    private fun response(request: Request): String = client.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) throw IllegalStateException("server returned ${response.code}: $body")
        body
    }

    private fun authRequest(url: String, server: Server, intent: Intent): Request.Builder =
        authenticated(
            Request.Builder().url(url),
            server.url,
            prefs,
            intent.getStringExtra("token"),
        )

    private fun resolveServer(selector: String?): Server {
        val all = servers()
        val server = if (selector.isNullOrBlank()) all.first()
        else all.firstOrNull { it.url == selector.trimEnd('/') || it.name == selector }
            ?: if (selector.startsWith("http://") || selector.startsWith("https://")) {
                Server(normalizeServerUrl(selector), selector)
            } else {
                throw IllegalArgumentException("server is not configured: $selector")
            }
        return server.copy(url = normalizeServerUrl(server.url))
    }

    private fun servers(): List<Server> {
        val urls = prefs.getStringSet("servers", emptySet()) ?: emptySet()
        if (urls.isEmpty()) return listOf(Server("http://localhost:8419", "Local"))
        return urls.sorted().map { url -> Server(url, prefs.getString("server_name_$url", url) ?: url) }
    }

    private fun required(intent: Intent, name: String): String =
        intent.getStringExtra(name)?.takeIf { it.isNotBlank() }
            ?: throw IllegalArgumentException("missing --es $name")

    private fun validateTopic(value: String): String {
        val topic = value.trim()
        require(topic.length in 1..128 && topic.all {
            it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it in "._-"
        }) {
            "invalid topic"
        }
        return topic
    }

    private fun split(value: String?, separator: Char): List<String> = value
        ?.split(separator)
        ?.map { it.trim() }
        ?.filter { it.isNotEmpty() }
        ?: emptyList()

    private fun ok(key: String, value: Any): String = JSONObject().put("ok", true).put(key, value).toString()

    private data class Server(val url: String, val name: String)

    companion object {
        private const val PREFS_NAME = "nsfy_prefs"
        private val JSON = "application/json; charset=utf-8".toMediaType()
    }
}
