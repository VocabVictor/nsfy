package com.nsfy.app.data.repository

import com.nsfy.app.data.db.AppDatabase
import com.nsfy.app.data.db.MessageWithTopic
import com.nsfy.app.data.model.*
import kotlinx.coroutines.flow.Flow
import androidx.room.withTransaction

class NsfyRepository(private val db: AppDatabase) {

    private val topicDao = db.topicDao()
    private val messageDao = db.messageDao()
    private val stateDao = db.messageStateDao()

    // Topics
    fun getAllTopics(): Flow<List<TopicEntity>> = topicDao.getAllTopics()
    suspend fun getTopic(id: String): TopicEntity? = topicDao.getTopic(id)

    suspend fun addTopic(serverUrl: String, name: String) {
        val id = topicId(serverUrl, name)
        topicDao.insertTopic(TopicEntity(id = id, serverUrl = serverUrl, name = name))
    }

    suspend fun deleteTopic(serverUrl: String, name: String) {
        val id = topicId(serverUrl, name)
        topicDao.deleteMessages(id)
        topicDao.deleteTopic(id)
    }

    suspend fun setTopicConnected(serverUrl: String, name: String, connected: Boolean) {
        topicDao.setConnected(topicId(serverUrl, name), connected, System.currentTimeMillis())
    }

    // Messages
    fun getMessages(topicId: String): Flow<List<MessageEntity>> =
        messageDao.getMessagesForTopic(topicId)

    fun getAllMessagesWithTopic(): Flow<List<MessageWithTopic>> =
        messageDao.getAllMessagesWithTopic()

    fun getTrash(): Flow<List<MessageEntity>> = messageDao.getTrash()
    fun getTrashWithTopic(): Flow<List<MessageWithTopic>> = messageDao.getTrashWithTopic()
    fun getPendingStates(): Flow<List<MessageStateEntity>> = stateDao.observePending()
    suspend fun getPendingStatesOnce(): List<MessageStateEntity> = stateDao.getPending()

    suspend fun saveMessage(serverUrl: String, topicName: String, msg: NsfyMessage) {
        val tid = topicId(serverUrl, topicName)
        val savedState = stateDao.get(tid, msg.id)
        if (savedState?.status == STATUS_PURGED) return
        val entity = MessageEntity(
            id = msg.id,
            topicId = tid,
            time = msg.time,
            title = msg.title,
            message = msg.message,
            priority = msg.priority,
            tags = msg.tags.joinToString(","),
            category = org.json.JSONArray(msg.category).toString(),
            popup = msg.popup,
            bypassDnd = msg.bypassDnd,
            read = savedState != null,
            deletedAt = if (savedState?.status == STATUS_TRASH) System.currentTimeMillis() else null,
        )
        messageDao.insertMessage(entity)
        topicDao.updateLastMessage(tid, msg.time, msg.title.ifEmpty { msg.message })
        // Clean old messages
    }

    suspend fun markAllRead() = applyLocal(messageDao.getVisibleOnce().filterNot { it.read }, STATUS_READ)
    suspend fun trashAll() = applyLocal(messageDao.getVisibleOnce(), STATUS_TRASH)
    suspend fun markTopicRead(topicId: String) =
        applyLocal(messageDao.getVisibleForTopicOnce(topicId).filterNot { it.read }, STATUS_READ)
    suspend fun trashTopic(topicId: String) =
        applyLocal(messageDao.getVisibleForTopicOnce(topicId), STATUS_TRASH)
    suspend fun markRead(message: MessageEntity) = applyLocal(listOf(message), STATUS_READ)
    suspend fun trash(message: MessageEntity) = applyLocal(listOf(message), STATUS_TRASH)
    suspend fun restore(message: MessageEntity) = applyLocal(listOf(message), STATUS_READ)
    suspend fun purge(message: MessageEntity) = applyLocal(listOf(message), STATUS_PURGED)

    suspend fun applyRemote(topicId: String, updates: List<Pair<String, String>>) {
        val now = System.currentTimeMillis()
        db.withTransaction {
            for ((id, status) in updates) {
                stateDao.upsert(MessageStateEntity(
                    key = stateKey(topicId, id), topicId = topicId, messageId = id,
                    status = status, updatedAt = now, pending = false,
                ))
                applyMessageStatus(id, status, now)
            }
        }
    }

    suspend fun markStatesSent(keys: List<String>) = stateDao.markSent(keys)

    suspend fun prune(retentionDays: Int, trashDays: Int) {
        val now = System.currentTimeMillis()
        val messageCutoff = now / 1000 - retentionDays.coerceAtLeast(1) * 86_400L
        val trashCutoff = now - trashDays.coerceAtLeast(1) * 86_400_000L
        for (topic in topicDao.getAllTopicsOnce()) messageDao.deleteOldMessages(topic.id, messageCutoff)
        messageDao.pruneTrash(trashCutoff)
        stateDao.prune(now - 365L * 86_400_000L)
    }

    private suspend fun applyLocal(messages: List<MessageEntity>, status: String) {
        if (messages.isEmpty()) return
        val now = System.currentTimeMillis()
        db.withTransaction {
            stateDao.upsertAll(messages.map { message ->
                MessageStateEntity(
                    key = stateKey(message.topicId, message.id), topicId = message.topicId,
                    messageId = message.id, status = status, updatedAt = now, pending = true,
                )
            })
            val ids = messages.map { it.id }
            when (status) {
                STATUS_READ -> ids.forEach { messageDao.restore(it) }
                STATUS_TRASH -> messageDao.moveToTrash(ids, now)
                STATUS_PURGED -> messageDao.deleteByIds(ids)
            }
        }
    }

    private suspend fun applyMessageStatus(id: String, status: String, now: Long) {
        when (status) {
            STATUS_READ -> messageDao.restore(id)
            STATUS_TRASH -> messageDao.moveToTrash(listOf(id), now)
            STATUS_PURGED -> messageDao.deleteByIds(listOf(id))
        }
    }

    // Server configs (stored in DataStore, handled elsewhere)
    companion object {
        const val STATUS_READ = "read"
        const val STATUS_TRASH = "trash"
        const val STATUS_PURGED = "purged"
        fun topicId(server: String, name: String) = "$server|$name"
        fun stateKey(topicId: String, messageId: String) = "$topicId|$messageId"
    }
}
