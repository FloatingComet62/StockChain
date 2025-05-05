use oqs::{sig, kem};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeError;
use crate::gossip::{generate_room_name, Gossip, MessageData, Room};

#[derive(Serialize, Deserialize, Debug)]
pub enum InteractionMessage {
    Ping,
    RequestPublicKey,
    ReplyPublicKey(sig::PublicKey),
    SharedSecretExchange(SharedSecretExchange),
    SharedSecretExchangeResponse(SharedSecretExchangeResponse),
    SharedSecretCommunication(([u8; 12], Vec<u8>)),
    Other(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedSecretExchange {
    pub kem_pk: kem::PublicKey,
    pub signature: sig::Signature,
    pub pk: sig::PublicKey,
}

impl SharedSecretExchange {
    pub fn new(kem_pk: kem::PublicKey, signature: sig::Signature, pk: sig::PublicKey) -> Self {
        Self { kem_pk, signature, pk }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SharedSecretExchangeResponse {
    pub kem_ct: kem::Ciphertext,
    pub signature: sig::Signature,
    pub pk: sig::PublicKey,
}

impl SharedSecretExchangeResponse {
    pub fn new(kem_ct: kem::Ciphertext, signature: sig::Signature, pk: sig::PublicKey) -> Self {
        Self { kem_ct, signature, pk }
    }
}

#[derive(Debug)]
pub enum GetDataViaMessageError {
    NotOurChannel,
    Serde(SerdeError),
}
impl From<serde_json::Error> for GetDataViaMessageError {
    fn from(err: serde_json::Error) -> Self {
        GetDataViaMessageError::Serde(err)
    }
}

pub fn get_message_via_data(
    gossip: &mut Gossip,
    message_data: &MessageData
) -> Result<InteractionMessage, GetDataViaMessageError> {
    match (
        &message_data.room,
        serde_json::from_str(&message_data.message)?
    ) {
        (_, InteractionMessage::Ping) => Ok(InteractionMessage::Ping),
        (Room::PublicRoom(_), e) => Ok(InteractionMessage::Other(format!("Public room: {:?}", e))),
        // we can't have request public key in public room, because the group gets flooded with everyone saying their public keys
        (_, InteractionMessage::RequestPublicKey) => Ok(InteractionMessage::RequestPublicKey),
        (_, InteractionMessage::ReplyPublicKey(e)) => Ok(InteractionMessage::ReplyPublicKey(e)),
        (_, InteractionMessage::SharedSecretExchange(e)) => {
            if generate_room_name(gossip.peer_id()) != message_data.room.name() {
                // we don't care if it's not in our channel
                return Err(GetDataViaMessageError::NotOurChannel);
            }
            return Ok(InteractionMessage::SharedSecretExchange(e));
        },
        (_, InteractionMessage::SharedSecretExchangeResponse(e)) => Ok(InteractionMessage::SharedSecretExchangeResponse(e)),
        (_, InteractionMessage::SharedSecretCommunication(e)) => Ok(InteractionMessage::SharedSecretCommunication(e)),
        (_, InteractionMessage::Other(e)) => Ok(InteractionMessage::Other(e)),
    }
}