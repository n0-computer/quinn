#[cfg(all(feature = "aws-lc-rs", not(feature = "ring")))]
use aws_lc_rs::{aead, error, hkdf, hmac};
#[cfg(feature = "ring")]
use ring::{aead, error, hmac};

use crate::crypto::{self, CryptoError};

impl crypto::HmacKey for hmac::Key {
    fn sign(&self, data: &[u8], out: &mut [u8]) {
        out.copy_from_slice(hmac::sign(self, data).as_ref());
    }

    fn signature_len(&self) -> usize {
        32
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        Ok(hmac::verify(self, data, signature)?)
    }
}

impl crypto::HandshakeTokenKey for aead::LessSafeKey {
    fn seal(&self, token_nonce: u128, data: &mut Vec<u8>) -> Result<(), CryptoError> {
        let nonce: [u8; aead::NONCE_LEN] = *token_nonce
            .to_le_bytes()
            .first_chunk()
            .expect("u128 is bigger than NONCE_LEN");
        // Security: The nonce is randomly generated.
        // At the point where we're encrypting 2^32 tokens with the same key,
        // then we'll have a 2^-33 chance of an attacker that obtained all 2^32 of these
        // frames to decrypt all tokens.
        // This is outside standard state-of-the-art encryption, but it's the best you can
        // do with AES-GCM, and still very much reasonably secure.
        let nonce = aead::Nonce::assume_unique_for_key(nonce);
        let aad = aead::Aad::empty();
        self.seal_in_place_append_tag(nonce, aad, data)?;
        Ok(())
    }

    fn open<'a>(&self, token_nonce: u128, data: &'a mut [u8]) -> Result<&'a mut [u8], CryptoError> {
        let nonce: [u8; aead::NONCE_LEN] = *token_nonce
            .to_le_bytes()
            .first_chunk()
            .expect("u128 is bigger than NONCE_LEN");
        let aad = aead::Aad::empty();
        let nonce = aead::Nonce::assume_unique_for_key(nonce);
        Ok(self.open_in_place(nonce, aad, data)?)
    }
}

impl From<error::Unspecified> for CryptoError {
    fn from(_: error::Unspecified) -> Self {
        Self
    }
}
