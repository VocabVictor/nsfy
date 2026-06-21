package com.nsfy.app.data.db

import androidx.room.*
import com.nsfy.app.data.model.MessageEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface MessageDao {
    @Query("SELECT * FROM messages WHERE topicId = :topicId ORDER BY time DESC LIMIT 200")
    fun getMessagesForTopic(topicId: String): Flow<List<MessageEntity>>

    @Insert(onConflict = OnConflictStrategy.IGNORE)
    suspend fun insertMessage(msg: MessageEntity)

    @Query("DELETE FROM messages WHERE topicId = :topicId AND time < :before")
    suspend fun deleteOldMessages(topicId: String, before: Long)

    @Query("DELETE FROM messages WHERE topicId = :topicId")
    suspend fun deleteAllForTopic(topicId: String)
}
