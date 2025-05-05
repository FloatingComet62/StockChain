use oqs::{sig, kem};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use crate::gossip::{MessageData, Room};

#[derive(Serialize, Deserialize, Debug)]
pub enum InteractionMessage {
    Ping,
    RequestPublicKey,
    ReplyPublicKey(sig::PublicKey),
    SharedSecretExchange(SharedSecretExchange),
    SharedSecretExchangeResponse(SharedSecretExchangeResponse),
    SharedSecretCommunication(String),
    Other(String),
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
        serde_json::from_str(&message_data.message)?
    ) {
        (_, InteractionMessage::Ping) => Ok(InteractionMessage::Ping),
        (Room::PublicRoom(_), e) => Ok(InteractionMessage::Other(format!("Public room: {:?}", e))),
        // we can't have request public key in public room, because the group gets flooded with everyone saying their public keys
        (_, InteractionMessage::RequestPublicKey) => Ok(InteractionMessage::RequestPublicKey),
        (_, InteractionMessage::ReplyPublicKey(e)) => Ok(InteractionMessage::ReplyPublicKey(e)),
        (_, InteractionMessage::SharedSecretExchange(e)) => Ok(InteractionMessage::SharedSecretExchange(e)),
        (_, InteractionMessage::SharedSecretExchangeResponse(e)) => Ok(InteractionMessage::SharedSecretExchangeResponse(e)),
        (_, InteractionMessage::SharedSecretCommunication(e)) => Ok(InteractionMessage::SharedSecretCommunication(e)),
        (_, InteractionMessage::Other(e)) => Ok(InteractionMessage::Other(e)),
    }
}