use serde::{Serialize, Deserialize};
use serde_json::{self, Value};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Timestamp {
    #[serde(rename="$date")]
    millis: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "msg")]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    Connect {
        version: String,
        support: Vec<String>,
        #[serde(skip_serializing_if="Option::is_none")]
        session: Option<String>,
    },

    Ping { 
        #[serde(skip_serializing_if="Option::is_none")]
        id: Option<String>
    },
    Pong { 
        #[serde(skip_serializing_if="Option::is_none")]
        id: Option<String>
    },

    Method { 
        id: String, 
        method: String, 
        params: Vec<Value>
    },


    Sub { 
        id: String, name: String, params: Vec<Value> 
    },
    Unsub { 
        id: String 
    },

}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "msg")]
#[serde(rename_all = "camelCase")]
pub enum ServerMessage {
    Connected {
        session: String,
    },
    Failed {
        version: String,
    },
    Ping { 
        #[serde(default, skip_serializing_if="Option::is_none")]
        id: Option<String>
    },
    Pong { 
        #[serde(default, skip_serializing_if="Option::is_none")]
        id: Option<String>
    },
    Result(MethodResponse),
    Nosub { 
        id: String, 
        #[serde(default, skip_serializing_if="Option::is_none")]
        error: Option<Value>
    },
    Updated { 
        methods: Vec<String> 
    },
    Added {
        collection: String,
        id: String,
        fields: Option<Value>,
    },
    Changed {
        collection: String,
        id: String,
        #[serde(default, skip_serializing_if="Option::is_none")]
        fields: Option<Value>,
        #[serde(default, skip_serializing_if="Option::is_none")]
        cleared: Option<Vec<String>>,
    },
    Removed {
        collection: String,
        id: String,
    },
    Ready {
        subs: Vec<String>,
    },
    AddedBefore {
        collection: String,
        id: String,
        #[serde(default, skip_serializing_if="Option::is_none")]
        fields: Option<Value>,
        before: Option<String>,
    },
    MovedBefore {
        before: Option<String>,
    }

}

impl ServerMessage {

    pub fn pretty(&self) -> String {
        serde_json::to_value(&self)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or_else(|_| "<<serialization error>>".to_string())
    }

}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "lowercase")]
pub enum MethodResponse {
    Result { id: String, result: Value },
    Error { id: String, error: Value },
}

impl MethodResponse {
    pub fn id(&self) -> &str {
        match self {
            MethodResponse::Result { id, .. } => id,
            MethodResponse::Error { id, .. } => id,
        }
    }
}

impl Into<Result<Value,Value>> for MethodResponse {
    fn into(self) -> Result<Value,Value> {
        match self {
            MethodResponse::Result { result, .. } => Ok(result),
            MethodResponse::Error { error, .. } => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use serde::de::DeserializeOwned;

    fn check_message<M>(msg: &M)
        where M: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug
    {
        let s = serde_json::to_string(msg).unwrap();
        let msg2: M = serde_json::from_str(&s).unwrap();
        assert_eq!(msg, &msg2);
        
    }

    #[test]
    fn test_method_result() {
        check_message(&ServerMessage::Result( 
            MethodResponse::Result {
                id: "123".to_string(),
                result: Value::String("burp".to_string()),
            }
        ));
    }

    #[test]
    fn test_method_error() {
        check_message(&ServerMessage::Result(
            MethodResponse::Error {
                id: "456:kahcubwdasd".to_string(),
                error: Value::Bool(true),
            }
        ));
    }

    #[test]
    fn test_pingpong() {

        check_message(&ServerMessage::Ping { id: None });
        check_message(&ServerMessage::Ping { id: Some("pingpong".to_string()) });
    }

    #[test]
    fn test_timestamp() {
        assert_eq!(serde_json::from_str::<Timestamp>("{\"$date\": 129348109238}").unwrap(), Timestamp { millis: 129348109238 });
        check_message(&Timestamp{ millis: 129348109238 });
    }
}