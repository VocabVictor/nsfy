package com.nsfy.app.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import com.nsfy.app.data.model.MessageStateEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface MessageStateDao {
    @Query("SELECT * FROM message_states WHERE topicId = :topicId AND messageId = :messageId")
    suspend fun get(topicId: String, messageId: String): MessageStateEntity?

    @Query("SELECT * FROM message_states WHERE pending = 1 ORDER BY updatedAt LIMIT 500")
    fun observePending(): Flow<List<MessageStateEntity>>

    @Query("SELECT * FROM message_states WHERE pending = 1 ORDER BY updatedAt LIMIT 500")
    suspend fun getPending(): List<MessageStateEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(state: MessageStateEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertAll(states: List<MessageStateEntity>)

    @Query("UPDATE message_states SET pending = 0 WHERE key IN (:keys)")
    suspend fun markSent(keys: List<String>)

    @Query("DELETE FROM message_states WHERE updatedAt < :cutoff AND pending = 0")
    suspend fun prune(cutoff: Long)
}
