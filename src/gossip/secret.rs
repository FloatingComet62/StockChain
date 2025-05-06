use libp2p::PeerId;
use oqs::{
    kem::{self, Kem, SharedSecret},
    sig::{self, Sig},
};
use std::collections::HashMap;
use std::error::Error;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::rand_core::RngCore}; // AES-GCM cipher // Traits and random number generator

pub struct Secret {
    sig: Sig,
    kem: Kem,
    pub private_key: oqs::sig::SecretKey,
    pub public_key: oqs::sig::PublicKey,
    pub shared_secret: HashMap<PeerId, SharedSecret>,
    pub shared_secret_unresponded_requests: HashMap<PeerId, kem::SecretKey>,
}
impl Secret {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let sig = Sig::new(sig::Algorithm::MlDsa87)?;
        let (public_key, private_key) = sig.keypair()?;
        Ok(Self {
            sig,
            private_key,
            public_key,
            kem: Kem::new(kem::Algorithm::MlKem1024)?,
            shared_secret: HashMap::new(),
            shared_secret_unresponded_requests: HashMap::new(),
        })
    }
    pub fn keys(&self) -> (oqs::sig::PublicKey, oqs::sig::SecretKey) {
        (self.public_key.clone(), self.private_key.clone())
    }
    pub fn send_shared_secret(
        &mut self,
        peer_id: PeerId,
    ) -> Result<(kem::PublicKey, sig::Signature, sig::PublicKey), oqs::Error> {
        let (kem_pk, kem_sk) = self.kem.keypair()?;
        let signature = self.sig.sign(kem_pk.as_ref(), &self.private_key)?;
        self.shared_secret_unresponded_requests
            .insert(peer_id, kem_sk);
        // A -> B: kem_pk, signature, pk
        Ok((kem_pk, signature, self.public_key.clone()))
    }

    pub fn receive_shared_secret(
        &mut self,
        peer_id: PeerId,
        kem_pk: kem::PublicKey,
        signature: sig::Signature,
        pk: sig::PublicKey,
    ) -> Result<(kem::Ciphertext, sig::Signature, sig::PublicKey), oqs::Error> {
        self.sig.verify(kem_pk.as_ref(), &signature, &pk)?;
        let (kem_ct, kem_ss) = self.kem.encapsulate(&kem_pk)?;
        let signature = self.sig.sign(kem_ct.as_ref(), &self.private_key)?;

        self.shared_secret.insert(peer_id, kem_ss);
        // B -> A: kem_ct, signature
        Ok((kem_ct, signature, self.public_key.clone()))
    }

    pub fn receive_shared_secret_response(
        &mut self,
        peer_id: PeerId,
        kem_ct: kem::Ciphertext,
        signature: sig::Signature,
        pk: sig::PublicKey,
    ) -> Result<SharedSecret, oqs::Error> {
        let kem_sk = self.shared_secret_unresponded_requests.get(&peer_id);
        let Some(kem_sk) = kem_sk else {
            // We didn't request a communication, most likely someone trying to find a bug
            return Err(oqs::Error::Error);
        };

        self.sig.verify(kem_ct.as_ref(), &signature, &pk)?;
        let shared_secret = self.kem.decapsulate(kem_sk, &kem_ct)?;
        self.shared_secret.insert(peer_id, shared_secret.clone());
        Ok(shared_secret)
    }
    pub fn encrypt(
        &self,
        peer_id: PeerId,
        message: &[u8],
    ) -> Result<([u8; 12], Vec<u8>), oqs::Error> {
        let kem_ss = self.shared_secret.get(&peer_id);
        let Some(kem_ss) = kem_ss else {
            // We don't have a shared secret with this peer, most likely someone trying to find a bug
            return Err(oqs::Error::Error);
        };

        let key = Key::<Aes256Gcm>::from_slice(&kem_ss.as_ref());
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, message)
            .map_err(|_| oqs::Error::Error)?;

        Ok((nonce_bytes, ciphertext))
    }
    pub fn decrypt(
        &self,
        peer_id: PeerId,
        nonce: [u8; 12],
        ciphertext: Vec<u8>,
    ) -> Result<Vec<u8>, oqs::Error> {
        let kem_ss = self.shared_secret.get(&peer_id);
        let Some(kem_ss) = kem_ss else {
            // We don't have a shared secret with this peer, most likely someone trying to find a bug
            return Err(oqs::Error::Error);
        };

        let key = Key::<Aes256Gcm>::from_slice(&kem_ss.as_ref());
        let cipher = Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&nonce);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| oqs::Error::Error)?;

        Ok(plaintext)
    }
}
