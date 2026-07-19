package com.nsfy.app.data.db

import androidx.room.*
import com.nsfy.app.data.model.TopicEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface TopicDao {
    @Query("SELECT * FROM topics ORDER BY lastMessageTime DESC")
    fun getAllTopics(): Flow<List<TopicEntity>>

    @Query("SELECT * FROM topics ORDER BY lastMessageTime DESC")
    suspend fun getAllTopicsOnce(): List<TopicEntity>

    @Query("SELECT COUNT(*) FROM topics")
    suspend fun getTopicCount(): Int

    @Query("SELECT * FROM topics WHERE id = :topicId")
    suspend fun getTopic(topicId: String): TopicEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertTopic(topic: TopicEntity)

    @Query("UPDATE topics SET lastMessageTime = :time, lastMessagePreview = :preview WHERE id = :topicId")
    suspend fun updateLastMessage(topicId: String, time: Long, preview: String)

    @Query("UPDATE topics SET isConnected = :connected WHERE id = :topicId")
    suspend fun setConnected(topicId: String, connected: Boolean)

    @Query("DELETE FROM topics WHERE id = :topicId")
    suspend fun deleteTopic(topicId: String)

    @Query("DELETE FROM messages WHERE topicId = :topicId")
    suspend fun deleteMessages(topicId: String)
}
