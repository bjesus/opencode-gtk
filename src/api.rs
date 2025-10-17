use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub info: MessageInfo,
    pub parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePart {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub part_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub models: std::collections::HashMap<String, Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub providers: Vec<Provider>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    pub parts: Vec<MessagePart>,
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn get_sessions(&self) -> Result<Vec<Session>, reqwest::Error> {
        self.client
            .get(format!("{}/session", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn create_session(&self) -> Result<Session, reqwest::Error> {
        self.client
            .post(format!("{}/session", self.base_url))
            .json(&serde_json::json!({}))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .delete(format!("{}/session/{}", self.base_url, session_id))
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<Message>, reqwest::Error> {
        self.client
            .get(format!("{}/session/{}/message", self.base_url, session_id))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn send_message(
        &self,
        session_id: &str,
        request: SendMessageRequest,
    ) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/session/{}/message", self.base_url, session_id))
            .json(&request)
            .send()
            .await?;
        Ok(())
    }

    pub async fn abort_generation(&self, session_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/session/{}/abort", self.base_url, session_id))
            .send()
            .await?;
        Ok(())
    }

    pub async fn share_session(&self, session_id: &str) -> Result<Session, reqwest::Error> {
        self.client
            .post(format!("{}/session/{}/share", self.base_url, session_id))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn unshare_session(&self, session_id: &str) -> Result<Session, reqwest::Error> {
        self.client
            .delete(format!("{}/session/{}/share", self.base_url, session_id))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn summarize_session(&self, session_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/session/{}/summarize", self.base_url, session_id))
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_config(&self) -> Result<Config, reqwest::Error> {
        self.client
            .get(format!("{}/config/providers", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn revert_message(&self, session_id: &str, message_id: &str) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/session/{}/revert", self.base_url, session_id))
            .json(&serde_json::json!({"messageID": message_id}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_event_stream(&self) -> Result<impl futures::Stream<Item = Result<eventsource_stream::Event, eventsource_stream::EventStreamError<reqwest::Error>>>, reqwest::Error> {
        use eventsource_stream::Eventsource;
        
        let response = self.client
            .get(format!("{}/event", self.base_url))
            .send()
            .await?;
        
        Ok(response.bytes_stream().eventsource())
    }
}

pub type SharedApiClient = Arc<Mutex<ApiClient>>;

pub fn create_shared_client(base_url: String) -> SharedApiClient {
    Arc::new(Mutex::new(ApiClient::new(base_url)))
}
