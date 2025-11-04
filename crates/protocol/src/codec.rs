use crate::Message;
use common::Result;

/// Message codec for serialization/deserialization
pub trait MessageCodec {
    fn encode(&self, message: &Message) -> Result<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> Result<Message>;
}

/// JSON codec (human-readable, debugging)
pub struct JsonCodec;

impl MessageCodec for JsonCodec {
    fn encode(&self, message: &Message) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(message)?)
    }

    fn decode(&self, data: &[u8]) -> Result<Message> {
        Ok(serde_json::from_slice(data)?)
    }
}

/// Bincode codec (compact, fast)
pub struct BincodeCodec;

impl MessageCodec for BincodeCodec {
    fn encode(&self, message: &Message) -> Result<Vec<u8>> {
        bincode::serialize(message)
            .map_err(|e| common::Error::SerializationError(
                serde_json::Error::custom(e.to_string())
            ))
    }

    fn decode(&self, data: &[u8]) -> Result<Message> {
        bincode::deserialize(data)
            .map_err(|e| common::Error::SerializationError(
                serde_json::Error::custom(e.to_string())
            ))
    }
}

/// Codec factory
pub enum CodecType {
    Json,
    Bincode,
}

impl CodecType {
    pub fn create(&self) -> Box<dyn MessageCodec> {
        match self {
            CodecType::Json => Box::new(JsonCodec),
            CodecType::Bincode => Box::new(BincodeCodec),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MessagePayload, ConnectRequest};
    use common::BranchId;
    use std::collections::HashMap;

    #[test]
    fn test_json_codec() {
        let message = Message::new(
            BranchId::new("test"),
            None,
            MessagePayload::Connect(ConnectRequest {
                branch_id: BranchId::new("test"),
                api_key: "key".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![],
                metadata: HashMap::new(),
            }),
        );

        let codec = JsonCodec;
        let encoded = codec.encode(&message).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(message.id, decoded.id);
    }

    #[test]
    fn test_bincode_codec() {
        let message = Message::new(
            BranchId::new("test"),
            None,
            MessagePayload::Heartbeat,
        );

        let codec = BincodeCodec;
        let encoded = codec.encode(&message).unwrap();
        let decoded = codec.decode(&encoded).unwrap();

        assert_eq!(message.id, decoded.id);
    }
}
