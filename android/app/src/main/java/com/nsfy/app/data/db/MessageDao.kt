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
    @Query("SELECT * FROM messages WHERE topicId = :topicId ORDER BY time DESC LIMIT 200")
    fun getMessagesForTopic(topicId: String): Flow<List<MessageEntity>>

    @Query(
        "SELECT messages.*, topics.name AS topicName, topics.serverUrl AS serverUrl " +
        "FROM messages JOIN topics ON messages.topicId = topics.id " +
        "ORDER BY messages.time DESC LIMIT 500"
    )
    fun getAllMessagesWithTopic(): Flow<List<MessageWithTopic>>

    @Insert(onConflict = OnConflictStrategy.IGNORE)
    suspend fun insertMessage(msg: MessageEntity)

    @Query("DELETE FROM messages WHERE topicId = :topicId AND time < :before")
    suspend fun deleteOldMessages(topicId: String, before: Long)

    @Query("DELETE FROM messages WHERE topicId = :topicId")
    suspend fun deleteAllForTopic(topicId: String)
}
