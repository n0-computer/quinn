#[cfg(all(feature = "aws-lc-rs", not(feature = "ring")))]
use aws_lc_rs::{aead, error, hkdf, hmac};
#[cfg(feature = "ring")]
use ring::{aead, error, hkdf, hmac};

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

/// Implements retry token encryption using HKDF followed by AES-256-GCM.
///
/// This construction was originally defined in https://github.com/quinn-rs/quinn/issues/783
/// The goal is to produce tokens that look random to clients, but contain decryptable
/// information for the server.
///
/// The problem is the 12-byte nonce in AES-GCM: I suspect the original authors of this
/// code didn't like the fact that it limits you to ~2^32 safe encryptions before you're getting
/// into "dangerous" nonce-reuse territory with randomly-generated nonces.
/// You can't use sequence numbers either, though, because that doesn't look random to
/// clients and leaks information.
pub(crate) struct RetryTokenKey(hkdf::Prk);

impl RetryTokenKey {
    pub(crate) fn new(rng: &mut impl rand::Rng) -> Self {
        let mut master_key = [0u8; 64];
        rng.fill_bytes(&mut master_key);
        let master_key = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]).extract(&master_key);
        Self(master_key)
    }

    fn derive_aead(&self, token_nonce: u128) -> aead::LessSafeKey {
        let nonce_bytes = token_nonce.to_le_bytes();
        let info = &[&nonce_bytes[..]];
        let okm = self.0.expand(info, hkdf::HKDF_SHA256).unwrap();

        let mut key_buffer = [0u8; 32];
        okm.fill(&mut key_buffer).unwrap();

        let key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_buffer).unwrap();
        aead::LessSafeKey::new(key)
    }
}

impl crypto::HandshakeTokenKey for RetryTokenKey {
    fn seal(&self, token_nonce: u128, data: &mut Vec<u8>) -> Result<(), CryptoError> {
        let aead_key = self.derive_aead(token_nonce);
        let nonce = aead::Nonce::assume_unique_for_key([0u8; 12]); // See docs for RetryTokenKey
        let aad = aead::Aad::empty();
        aead_key.seal_in_place_append_tag(nonce, aad, data)?;
        Ok(())
    }

    fn open<'a>(&self, token_nonce: u128, data: &'a mut [u8]) -> Result<&'a [u8], CryptoError> {
        let aead_key = self.derive_aead(token_nonce);
        let aad = aead::Aad::empty();
        let nonce = aead::Nonce::assume_unique_for_key([0u8; 12]); // See docs for RetryTokenKey
        Ok(aead_key.open_in_place(nonce, aad, data)?)
    }
}

impl From<error::Unspecified> for CryptoError {
    fn from(_: error::Unspecified) -> Self {
        Self
    }
}
