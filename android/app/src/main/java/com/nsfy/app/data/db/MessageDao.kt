package com.nsfy.app.data.db

import androidx.room.*
import com.nsfy.app.data.model.MessageEntity
import kotlinx.coroutines.flow.Flow

// One timeline row: a message plus the display name of its topic,
// for the 1b unified-timeline layout.
data class MessageWithTopic(
    @Embedded val msg: MessageEntity,
    val topicName: String,
    val serverUrl: String,
)

@Dao
interface MessageDao {
    @Query("SELECT * FROM messages WHERE topicId = :topicId AND deletedAt IS NULL ORDER BY time DESC LIMIT 200")
    fun getMessagesForTopic(topicId: String): Flow<List<MessageEntity>>

    @Query(
        "SELECT messages.*, topics.name AS topicName, topics.serverUrl AS serverUrl " +
        "FROM messages JOIN topics ON messages.topicId = topics.id WHERE messages.deletedAt IS NULL " +
        "ORDER BY messages.time DESC LIMIT 500"
    )
    fun getAllMessagesWithTopic(): Flow<List<MessageWithTopic>>

    @Insert(onConflict = OnConflictStrategy.IGNORE)
    suspend fun insertMessage(msg: MessageEntity)

    @Query("SELECT * FROM messages WHERE deletedAt IS NULL ORDER BY time DESC LIMIT 500")
    suspend fun getVisibleOnce(): List<MessageEntity>

    @Query("SELECT * FROM messages WHERE topicId = :topicId AND deletedAt IS NULL")
    suspend fun getVisibleForTopicOnce(topicId: String): List<MessageEntity>

    @Query("SELECT * FROM messages WHERE deletedAt IS NOT NULL ORDER BY deletedAt DESC LIMIT 500")
    fun getTrash(): Flow<List<MessageEntity>>

    @Query(
        "SELECT messages.*, topics.name AS topicName, topics.serverUrl AS serverUrl " +
            "FROM messages JOIN topics ON messages.topicId = topics.id " +
            "WHERE messages.deletedAt IS NOT NULL ORDER BY messages.deletedAt DESC LIMIT 500"
    )
    fun getTrashWithTopic(): Flow<List<MessageWithTopic>>

    @Query("UPDATE messages SET read = 1 WHERE id IN (:ids)")
    suspend fun markRead(ids: List<String>)

    @Query("UPDATE messages SET read = 1, deletedAt = NULL WHERE id = :id")
    suspend fun restore(id: String)

    @Query("UPDATE messages SET read = 1, deletedAt = :deletedAt WHERE id IN (:ids)")
    suspend fun moveToTrash(ids: List<String>, deletedAt: Long)

    @Query("DELETE FROM messages WHERE id IN (:ids)")
    suspend fun deleteByIds(ids: List<String>)

    @Query("DELETE FROM messages WHERE deletedAt IS NOT NULL AND deletedAt < :cutoff")
    suspend fun pruneTrash(cutoff: Long)

    @Query("DELETE FROM messages WHERE topicId = :topicId AND time < :before")
    suspend fun deleteOldMessages(topicId: String, before: Long)

    @Query("DELETE FROM messages WHERE topicId = :topicId")
    suspend fun deleteAllForTopic(topicId: String)
}
