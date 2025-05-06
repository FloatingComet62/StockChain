use super::{GossipEvent, MyBehaviourEvent};
use libp2p::{Multiaddr, PeerId, gossipsub::Message, swarm::SwarmEvent};

pub trait EventHandler {
    fn new_connections(&mut self, list: Vec<(PeerId, Multiaddr)>) -> Option<GossipEvent>;
    fn new_disconnections(&mut self, list: Vec<(PeerId, Multiaddr)>) -> Option<GossipEvent>;
    fn message(&mut self, peer_id: PeerId, message: Message) -> Option<GossipEvent>;
    fn handle(&mut self, event: SwarmEvent<MyBehaviourEvent>) -> Option<GossipEvent>;
}
