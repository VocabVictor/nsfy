package com.nsfy.app.data.repository

import com.nsfy.app.data.db.AppDatabase
import com.nsfy.app.data.db.MessageWithTopic
import com.nsfy.app.data.model.*
import kotlinx.coroutines.flow.Flow

class NsfyRepository(private val db: AppDatabase) {

    private val topicDao = db.topicDao()
    private val messageDao = db.messageDao()

    // Topics
    fun getAllTopics(): Flow<List<TopicEntity>> = topicDao.getAllTopics()

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
        topicDao.setConnected(topicId(serverUrl, name), connected)
    }

    // Messages
    fun getMessages(topicId: String): Flow<List<MessageEntity>> =
        messageDao.getMessagesForTopic(topicId)

    fun getAllMessagesWithTopic(): Flow<List<MessageWithTopic>> =
        messageDao.getAllMessagesWithTopic()

    suspend fun saveMessage(serverUrl: String, topicName: String, msg: NsfyMessage) {
        val tid = topicId(serverUrl, topicName)
        val entity = MessageEntity(
            id = msg.id,
            topicId = tid,
            time = msg.time,
            title = msg.title,
            message = msg.message,
            priority = msg.priority,
            tags = msg.tags.joinToString(","),
            category = org.json.JSONArray(msg.category).toString(),
        )
        messageDao.insertMessage(entity)
        topicDao.updateLastMessage(tid, msg.time, msg.title.ifEmpty { msg.message })
        // Clean old messages
        messageDao.deleteOldMessages(tid, msg.time - 86400 * 7) // keep 7 days
    }

    // Server configs (stored in DataStore, handled elsewhere)
    companion object {
        fun topicId(server: String, name: String) = "$server|$name"
    }
}
