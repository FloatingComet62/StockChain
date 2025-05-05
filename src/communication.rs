use oqs::{sig, kem};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use crate::gossip::{MessageData, Room};

#[derive(Serialize, Deserialize)]
pub enum InteractionMessage {
    Ping,
    RequestPublicKey,
    SharedSecretExchange(SharedSecretExchange),
    SharedSecretExchangeResponse(SharedSecretExchangeResponse),
    SharedSecretCommunication(String),
    Other
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedSecretExchange {
    pub kem_pk: kem::PublicKey,
    pub signature: sig::Signature,
    pub pk: sig::PublicKey,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SharedSecretExchangeResponse {
    pub kem_ct: kem::Ciphertext,
    pub signature: sig::Signature,
    pub pk: sig::PublicKey,
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
        (_, InteractionMessage::SharedSecretExchange(e)) => Ok(InteractionMessage::SharedSecretExchange(e)),
        (_, InteractionMessage::SharedSecretExchangeResponse(e)) => Ok(InteractionMessage::SharedSecretExchangeResponse(e)),
        (_, InteractionMessage::SharedSecretCommunication(e)) => Ok(InteractionMessage::SharedSecretCommunication(e)),
        (_, InteractionMessage::Other) => Ok(InteractionMessage::Other),
    }
}