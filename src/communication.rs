use oqs::sig::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use crate::gossip::{MessageData, Room};

#[derive(Serialize, Deserialize)]
pub enum InteractionMessage {
    Ping,
    RequestPublicKey,
    SharedSecretExchange(SharedSecretExchange),
    SharedSecretExchangeResponse(String),
    SharedSecretCommunication(String),
    Other
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedSecretExchange {
    // rethink this
    pub pk: PublicKey,
    pub sig: Signature,
}

pub fn get_message_via_data(
    message_data: &MessageData
) -> Result<InteractionMessage, Error> {
    match (
        &message_data.room,
        serde_json::from_str(&message_data.message).unwrap_or(InteractionMessage::Other)
    ) {
        (_, InteractionMessage::Ping) => Ok(InteractionMessage::Ping),
        (_, InteractionMessage::RequestPublicKey) => Ok(InteractionMessage::RequestPublicKey),
        (Room::PublicRoom(_), _) => Ok(InteractionMessage::Other),
        (Room::DirectMessage(_), InteractionMessage::SharedSecretExchange(shared_secret_exchange)) => Ok(InteractionMessage::SharedSecretExchange(shared_secret_exchange)),
        (Room::DirectMessage(_), InteractionMessage::SharedSecretExchangeResponse(_)) => todo!(),
        (Room::DirectMessage(_), InteractionMessage::SharedSecretCommunication(_)) => todo!(),
        (Room::DirectMessage(_), InteractionMessage::Other) => Ok(InteractionMessage::Other),
    }
}