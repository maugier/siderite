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
#[serde(rename_all = "lowercase")]
pub enum Message {
    Ping,
    Pong,
    Connect { version: String, support: Vec<String> },
    Method { id: NumericID, method: String, params: Vec<Value> },
    Updated { methods: Vec<NumericID> },
    Result(MethodResponse),
    Sub { id: String, name: String, params: Vec<Value> },
    Unsub { id: String },
    Nosub { id: String },
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

    fn check_message(msg: &Message) {
        let s = serde_json::to_string(msg).unwrap();
        let msg2: Message = serde_json::from_str(&s).unwrap();
        assert_eq!(msg, &msg2);
    }

    check_message(&Message::Result( 
        MethodResponse::Result {
            id: NumericID(123),
            result: Value::String("burp".to_string()),
        }
    ));

    check_message(&Message::Result(
        MethodResponse::Error {
            id: NumericID(456),
            error: Value::Bool(true),
        }
    ));

}