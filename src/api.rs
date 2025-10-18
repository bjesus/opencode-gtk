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
    #[serde(rename = "sessionID", default)]
    pub session_id: Option<String>,
    pub role: String,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub tokens: Option<MessageTokens>,
    #[serde(rename = "modelID", default)]
    pub model_id: Option<String>,
    #[serde(rename = "providerID", default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub path: Option<MessagePath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePath {
    pub cwd: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTokens {
    pub input: u32,
    pub output: u32,
    #[serde(default)]
    pub reasoning: u32,
    pub cache: TokenCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCache {
    pub read: u32,
    pub write: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePart {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(rename = "type")]
    pub part_type: String,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub state: Option<ToolState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub mode: String,
    #[serde(rename = "builtIn")]
    pub built_in: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    pub parts: Vec<MessagePart>,
    pub model: ModelInfo,
    pub agent: String,
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
        let json_body = serde_json::to_string_pretty(&request).unwrap_or_else(|_| "Failed to serialize".to_string());
        eprintln!("Sending POST to {}/session/{}/message with body:\n{}", self.base_url, session_id, json_body);
        
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

    pub async fn rename_session(&self, session_id: &str, title: &str) -> Result<Session, reqwest::Error> {
        self.client
            .patch(format!("{}/session/{}", self.base_url, session_id))
            .json(&serde_json::json!({"title": title}))
            .send()
            .await?
            .json()
            .await
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

    pub async fn fork_session(&self, session_id: &str, message_id: Option<&str>) -> Result<Session, reqwest::Error> {
        let url = if let Some(msg_id) = message_id {
            format!("{}/session/{}/fork?messageID={}", self.base_url, session_id, msg_id)
        } else {
            format!("{}/session/{}/fork", self.base_url, session_id)
        };
        
        self.client
            .post(url)
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_config(&self) -> Result<Config, reqwest::Error> {
        self.client
            .get(format!("{}/config/providers", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_agents(&self) -> Result<Vec<Agent>, reqwest::Error> {
        self.client
            .get(format!("{}/agent", self.base_url))
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
