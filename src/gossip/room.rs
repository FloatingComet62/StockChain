use libp2p::{
    PeerId,
    gossipsub::{self, IdentTopic},
};
use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum Room {
    PublicRoom(String),
    DirectMessage(String),
}
impl Display for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Room::PublicRoom(name) => write!(f, "PublicRoom({})", name),
            Room::DirectMessage(name) => write!(f, "DirectMessage({})", name),
        }
    }
}
impl Room {
    pub fn name(&self) -> String {
        match self {
            Room::PublicRoom(name) => name.clone(),
            Room::DirectMessage(name) => name.clone(),
        }
    }
    pub fn is_public(&self) -> bool {
        match self {
            Room::PublicRoom(_) => true,
            Room::DirectMessage(_) => false,
        }
    }
    pub fn is_direct_message(&self) -> bool {
        match self {
            Room::PublicRoom(_) => false,
            Room::DirectMessage(_) => true,
        }
    }
}

pub trait GossipRooms {
    fn get_peer_from_room_name(&self, room_name: &str) -> Option<&PeerId>;
    fn get_topic_from_name(&self, topic_self: &str) -> Option<IdentTopic>;
    fn join_room(&mut self, topic_str: &str) -> Result<(), Box<dyn Error>>;
    fn leave_room(&mut self, topic_str: &str) -> Result<(), Box<dyn Error>>;
    fn get_room_from_hash(&self, topic: gossipsub::TopicHash) -> Room;
    fn get_room_from_name(&self, topic: String) -> Room;
}
