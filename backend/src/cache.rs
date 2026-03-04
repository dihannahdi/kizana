use crate::models::*;
use redis::AsyncCommands;

pub struct CacheManager {
    client: redis::Client,
}

impl CacheManager {
    pub fn new(redis_url: &str) -> Result<Self, String> {
        let client = redis::Client::open(redis_url).map_err(|e| e.to_string())?;
        Ok(CacheManager { client })
    }

    async fn get_conn(&self) -> Result<redis::aio::MultiplexedConnection, String> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))
    }

    // ─── Query cache ───
    pub async fn get_cached_search(&self, query: &str) -> Option<Vec<SearchResult>> {
        let key = format!("search:{}", query);
        match self.get_conn().await {
            Ok(mut conn) => {
                let cached: Option<String> = conn.get(&key).await.ok()?;
                cached.and_then(|s| serde_json::from_str(&s).ok())
            }
            Err(_) => None,
        }
    }

    pub async fn cache_search(&self, query: &str, results: &[SearchResult]) {
        let key = format!("search:{}", query);
        if let Ok(json) = serde_json::to_string(results) {
            if let Ok(mut conn) = self.get_conn().await {
                let _: Result<(), _> = conn.set_ex(&key, &json, 3600).await; // 1h TTL
            }
        }
    }

    // ─── Session cache ───
    pub async fn cache_session(&self, session: &ChatSession) {
        let key = format!("session:{}", session.id);
        if let Ok(json) = serde_json::to_string(session) {
            if let Ok(mut conn) = self.get_conn().await {
                let _: Result<(), _> = conn.set_ex(&key, &json, 86400).await; // 24h TTL
            }
        }
    }

    pub async fn get_cached_session(&self, session_id: &str) -> Option<ChatSession> {
        let key = format!("session:{}", session_id);
        match self.get_conn().await {
            Ok(mut conn) => {
                let cached: Option<String> = conn.get(&key).await.ok()?;
                cached.and_then(|s| serde_json::from_str(&s).ok())
            }
            Err(_) => None,
        }
    }

    pub async fn delete_session(&self, session_id: &str) {
        let key = format!("session:{}", session_id);
        if let Ok(mut conn) = self.get_conn().await {
            let _: Result<(), _> = conn.del(&key).await;
        }
    }

    pub async fn delete_sessions(&self, session_ids: &[String]) {
        if let Ok(mut conn) = self.get_conn().await {
            for id in session_ids {
                let key = format!("session:{}", id);
                let _: Result<(), _> = conn.del(&key).await;
            }
        }
    }

    // ─── User session list cache ───
    pub async fn invalidate_user_sessions(&self, user_id: i64) {
        let key = format!("user_sessions:{}", user_id);
        if let Ok(mut conn) = self.get_conn().await {
            let _: Result<(), _> = conn.del(&key).await;
        }
    }
}

