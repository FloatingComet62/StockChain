use futures::stream::StreamExt;
use std::error::Error;
use tokio::{io, io::AsyncBufReadExt, select};

use stockchain::{
    communication::{get_message_via_data, InteractionMessage},
    gossip::{Gossip, GossipEvent}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut gossip = Gossip::new()?;
    gossip.join_room("test")?;
    gossip.open_ears()?;

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Kick it off
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                if let Err(e) = gossip.gossip(&parse_command(line.as_str()), gossip.topics[0].1.clone()) {
                    println!("Publish error: {e:?}");
                }
            }
            event = gossip.swarm.select_next_some() => {
                let Some(action) = gossip.handle_event(event) else {
                    continue;
                };
                let GossipEvent::Message(data) = action else {
                    println!("Event: {action:?}");
                    continue;
                };
                let message = match get_message_via_data(&data) {
                    Ok(message) => message,
                    Err(e) => {
                        println!("Error parsing message: {e:?}");
                        continue;
                    }
                };
                match message {
                    InteractionMessage::Ping => {
                        println!("Ping received");
                    }
                    InteractionMessage::SharedSecretExchange(shared_secret_exchange) => {
                        println!("Shared secret exchange: {:?}", shared_secret_exchange);
                    }
                    InteractionMessage::SharedSecretExchangeResponse(response) => {
                        println!("Shared secret exchange response: {:?}", response);
                    }
                    InteractionMessage::SharedSecretCommunication(communication) => {
                        println!("Shared secret communication: {:?}", communication);
                    }
                    InteractionMessage::RequestPublicKey => {
                        println!("Request public key received");
                    }
                    InteractionMessage::Other => {
                        println!("Other message received");
                    }
                }
            }
        }
    }
}

fn parse_command(command: &str) -> InteractionMessage {
    match command {
        "ping" => InteractionMessage::Ping,
        "request_public_key" => InteractionMessage::RequestPublicKey,
        "shared_secret_exchange" => todo!(),
        "shared_secret_exchange_response" => InteractionMessage::SharedSecretExchangeResponse("response".to_string()),
        "shared_secret_communication" => InteractionMessage::SharedSecretCommunication("communication".to_string()),
        _ => InteractionMessage::Other,
    }
}