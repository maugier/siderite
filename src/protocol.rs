use serde::{Serialize, Deserialize};
use serde_json::{self, Value};


#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Timestamp {
    #[serde(rename="$date")]
    millis: Option<u64>,
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
pub struct MethodResponse {
    pub id: String,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if="Option::is_none")]
    pub error: Option<Value>,
}



#[cfg(test)]
mod tests {

    use super::*;

    use serde::de::DeserializeOwned;

    fn check_message<M>(msg: &M, string: &str)
        where M: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug
    {
        let serialized = serde_json::to_string(msg).unwrap();
        assert_eq!(serialized, string);
        let deserialized: M = serde_json::from_str(&string).unwrap();
        assert_eq!(msg, &deserialized);
        
    }

    #[test]
    fn test_method_result() {
        check_message(&ServerMessage::Result( 
            MethodResponse {
                id: "123".to_string(),
                result: Some(Value::String("burp".to_string())),
                error: None
            }
        ), r#"{"msg":"result","id":"123","result":"burp"}"#);
    }

    #[test]
    fn test_method_error() {
        check_message(&ServerMessage::Result(
            MethodResponse {
                id: "456:kahcubwdasd".to_string(),
                error: Some(Value::Bool(true)),
                result: None,
            }

        ), r#"{"msg":"result","id":"456:kahcubwdasd","error":true}"#);
    }

    #[test]
    fn test_pingpong() {

        check_message(&ServerMessage::Ping { id: None }, r#"{"msg":"ping"}"#);
        check_message(&ServerMessage::Ping { id: Some("pingpong".to_string()) }, r#"{"msg":"ping","id":"pingpong"}"#);
    }

    #[test]
    fn test_timestamp() {
        check_message(&Timestamp{ millis: Some(129348109238) }, r#"{"$date":129348109238}"#);
    }
}