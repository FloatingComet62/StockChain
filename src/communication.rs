use crate::gossip::{GenerateRoomName, message::MessageData, room::Room};
use libp2p::PeerId;
use oqs::{kem, sig};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeError;

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
        Self {
            kem_pk,
            signature,
            pk,
        }
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
        Self {
            kem_ct,
            signature,
            pk,
        }
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

impl InteractionMessage {
    pub fn from_msg(
        self_peer_id: PeerId,
        message_data: &MessageData,
    ) -> Result<Self, GetDataViaMessageError> {
        match (
            &message_data.room,
            serde_json::from_str(&message_data.message)?,
        ) {
            (_, Self::Ping) => Ok(Self::Ping),
            (Room::PublicRoom(_), e) => Ok(Self::Other(format!("Public room: {:?}", e))),
            // we can't have request public key in public room, because the group gets flooded with everyone saying their public keys
            (_, Self::RequestPublicKey) => Ok(Self::RequestPublicKey),
            (_, Self::ReplyPublicKey(e)) => Ok(Self::ReplyPublicKey(e)),
            (_, Self::SharedSecretExchange(e)) => {
                if self_peer_id.generate_room_name() != message_data.room.name() {
                    // we don't care if it's not in our channel
                    return Err(GetDataViaMessageError::NotOurChannel);
                }
                return Ok(Self::SharedSecretExchange(e));
            }
            (_, Self::SharedSecretExchangeResponse(e)) => Ok(Self::SharedSecretExchangeResponse(e)),
            (_, Self::SharedSecretCommunication(e)) => Ok(Self::SharedSecretCommunication(e)),
            (_, Self::Other(e)) => Ok(Self::Other(e)),
        }
    }
}
