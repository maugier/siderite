use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug,PartialEq,Eq)]
pub struct NumericID(pub usize);

impl Serialize for NumericID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for NumericID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let s = <&str>::deserialize(deserializer)?;
        Ok(NumericID(usize::from_str_radix(s, 10).map_err(serde::de::Error::custom)?))
    }
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
        id: NumericID, 
        method: String, 
        params: Vec<Value>
    },


    Sub { 
        id: NumericID, name: String, params: Vec<Value> 
    },
    Unsub { 
        id: NumericID 
    },
    Nosub { 
        id: NumericID, 
        #[serde(skip_serializing_if="Option::is_none")]
        error: Option<Value>
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
        #[serde(skip_serializing_if="Option::is_none")]
        id: Option<String>
    },
    Pong { 
        #[serde(skip_serializing_if="Option::is_none")]
        id: Option<String>
    },
    Result(MethodResponse),
    Updated { 
        methods: Vec<NumericID> 
    },
    Added {
        collection: String,
        id: String,
        fields: Option<Value>,
    },
    Changed {
        collection: String,
        id: String,
        #[serde(skip_serializing_if="Option::is_none")]
        fields: Option<Value>,
        #[serde(skip_serializing_if="Option::is_none")]
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
        #[serde(skip_serializing_if="Option::is_none")]
        fields: Option<Value>,
        before: Option<String>,
    },
    MovedBefore {
        before: Option<String>,
    }

}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "lowercase")]
pub enum MethodResponse {
    Result { id: NumericID, result: Value },
    Error { id: NumericID, error: Value },
}

impl MethodResponse {
    pub fn id(&self) -> usize {
        match self {
            MethodResponse::Result { id, .. } => id.0,
            MethodResponse::Error { id, .. } => id.0,
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

#[test]
fn test_method_format() {

    use serde::de::DeserializeOwned;

    fn check_message<M>(msg: &M)
        where M: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug
    {
        let s = serde_json::to_string(msg).unwrap();
        let msg2: M = serde_json::from_str(&s).unwrap();
        assert_eq!(msg, &msg2);
        
    }

    check_message(&ServerMessage::Result( 
        MethodResponse::Result {
            id: NumericID(123),
            result: Value::String("burp".to_string()),
        }
    ));

    check_message(&ServerMessage::Result(
        MethodResponse::Error {
            id: NumericID(456),
            error: Value::Bool(true),
        }
    ));

    check_message(&ServerMessage::Ping { id: None });
    check_message(&ServerMessage::Ping { id: Some("pingpong".to_string()) });

}