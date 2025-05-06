use super::{
    GenerateRoomName, Gossip,
    room::{GossipRooms, Room},
};
use crate::communication::InteractionMessage;

#[derive(Debug)]
pub struct MessageData {
    pub peer: libp2p::PeerId,
    pub message: String,
    pub room: Room,
}

impl MessageData {
    pub fn reply_to_peer(&self, gossip: &mut Gossip, message: &InteractionMessage) {
        let room_name = self.peer.generate_room_name();
        if let Err(e) = gossip.join_room(&room_name) {
            println!("Error joining room: {e:?}");
            return;
        }
        let room_name = gossip.get_topic_from_name(&room_name);
        let Some(room_name) = room_name else {
            println!("Error getting room name");
            return;
        };
        if let Err(e) = gossip.gossip(message, room_name) {
            println!("Error sending shared secret exchange response: {e:?}");
        }
    }
    pub fn reply_to_room(&self, gossip: &mut Gossip, message: &InteractionMessage) {
        if let Err(e) = gossip.gossip(
            message,
            gossip.get_topic_from_name(&self.room.name()).unwrap(),
        ) {
            println!("Error sending public key: {e:?}");
        }
    }
}
