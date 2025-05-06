use std::error::Error;

use libp2p::Multiaddr;
use libp2p::gossipsub::{Event, Message, TopicHash};
use libp2p::mdns::Event::{Discovered, Expired};
use libp2p::swarm::SwarmEvent;
use libp2p::{PeerId, gossipsub::IdentTopic};

use crate::communication::InteractionMessage;

use super::events::EventHandler;
use super::message::MessageData;
use super::nonce::Nonce;
use super::room::{GossipRooms, Room};
use super::{Gossip, GossipEvent, MyBehaviourEvent};

impl GossipRooms for Gossip {
    fn get_peer_from_room_name(&self, room_name: &str) -> Option<&PeerId> {
        self.peer_ids
            .iter()
            .find(|id| id.to_string().contains(&room_name))
    }
    fn get_topic_from_name(&self, topic_self: &str) -> Option<IdentTopic> {
        for (room_name, room) in self.topics.iter() {
            if room_name == topic_self {
                return Some(room.clone());
            }
        }
        None
    }
    fn join_room(&mut self, topic_str: &str) -> Result<(), Box<dyn Error>> {
        let topic = IdentTopic::new(topic_str);
        self.topics.push((topic_str.to_string(), topic.clone()));

        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        Ok(())
    }
    fn leave_room(&mut self, topic_str: &str) -> Result<(), Box<dyn Error>> {
        let topic = IdentTopic::new(topic_str);
        self.topics.retain(|(t, _)| t != topic_str);
        let _ = self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);
        Ok(())
    }
    fn get_room_from_hash(&self, topic: TopicHash) -> Room {
        for t in &self.topics {
            if t.1.hash() == topic {
                return self.get_room_from_name(t.0.clone());
            }
        }
        panic!("Topic not found");
    }
    fn get_room_from_name(&self, topic: String) -> Room {
        if topic.starts_with("public_") {
            return Room::PublicRoom(topic);
        }
        Room::DirectMessage(topic)
    }
}

impl EventHandler for Gossip {
    fn new_connections(&mut self, list: Vec<(PeerId, Multiaddr)>) -> Option<GossipEvent> {
        let mut peers = Vec::with_capacity(list.len());
        for (peer_id, _multiaddr) in list {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .add_explicit_peer(&peer_id);
            peers.push(peer_id);
        }
        for peer in peers.iter() {
            self.peer_ids.insert(peer.clone());
        }
        return Some(GossipEvent::NewConnection(peers));
    }
    fn new_disconnections(&mut self, list: Vec<(PeerId, Multiaddr)>) -> Option<GossipEvent> {
        let mut peers = Vec::with_capacity(list.len());
        for (peer_id, _multiaddr) in list {
            self.swarm
                .behaviour_mut()
                .gossipsub
                .remove_explicit_peer(&peer_id);
            peers.push(peer_id);
        }
        for peer in peers.iter() {
            self.peer_ids.remove(peer);
        }
        return Some(GossipEvent::Disconnection(peers));
    }
    fn message(&mut self, peer_id: PeerId, message: Message) -> Option<GossipEvent> {
        let is_public_room = message.topic.to_string().starts_with("public_");
        let is_message_by_the_dm_op = peer_id.to_string().contains(&message.topic.to_string());
        let is_message_in_self_dm = self
            .peer_id()
            .to_string()
            .contains(&message.topic.to_string());
        // Messages to ignore
        // Private Room: Other DM's, other's messages
        // Messages to allow
        // Public Rooms
        // Private Room: DM OP's messages
        // FTF: Valid
        // FFF: Invalid
        // T__: Valid
        if !is_public_room && !is_message_by_the_dm_op && !is_message_in_self_dm {
            // probably someone asking the OP something, we don't care
            return None;
        }
        let data = Nonce::remove_nonce(&message.data);
        let content = String::from_utf8_lossy(&data);
        let msg_data = MessageData {
            peer: peer_id,
            message: content.to_string(),
            room: self.get_room_from_hash(message.topic),
        };
        let Ok(interaction) = InteractionMessage::from_msg(self.peer_id(), &msg_data) else {
            println!("Error parsing message: {:?}", msg_data);
            return None;
        };
        return Some(GossipEvent::Message((msg_data, interaction)));
    }
    fn handle(&mut self, event: SwarmEvent<MyBehaviourEvent>) -> Option<GossipEvent> {
        match event {
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(Discovered(list))) => {
                self.new_connections(list)
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(Expired(list))) => {
                self.new_disconnections(list)
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(Event::Message {
                propagation_source: peer_id,
                message_id: _,
                message,
            })) => self.message(peer_id, message),
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Local node is listening on {address}");
                None
            }
            _ => None,
        }
    }
}
