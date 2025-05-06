use rand::{fill, rngs::ThreadRng};

static NONCE_LEN: usize = 16;

pub struct Nonce {
    pub nonce_thread: ThreadRng,
}
impl Nonce {
    pub fn new() -> Self {
        Nonce {
            nonce_thread: rand::rng(),
        }
    }

    // Duplicate messages are apparantly not allowed, so we need to add a nonce to the message
    pub fn add_nonce(&self, message: &[u8]) -> Vec<u8> {
        let mut nonce = [0; NONCE_LEN];
        fill(&mut nonce);
        let mut data = Vec::with_capacity(message.len() + nonce.len());
        data.extend_from_slice(&nonce);
        data.extend_from_slice(message);
        data
    }
    pub fn remove_nonce(message: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(message.len() - NONCE_LEN);
        data.extend_from_slice(&message[NONCE_LEN..]);
        data
    }

    pub fn add_nonce_wsize(&self, message: &[u8], size: usize) -> Vec<u8> {
        let mut nonce = vec![0; size];
        fill(&mut nonce[..]);
        let mut data = Vec::with_capacity(message.len() + nonce.len());
        data.extend_from_slice(&nonce);
        data.extend_from_slice(message);
        data
    }
    pub fn remove_nonce_wsize(message: &[u8], size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(message.len() - size);
        data.extend_from_slice(&message[size..]);
        data
    }
}
