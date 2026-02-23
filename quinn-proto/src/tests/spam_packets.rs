//! Generators for crafted QUIC-like datagrams that exercise specific
//! rejection paths in [`Endpoint::handle()`].
//!
//! Each function returns a `BytesMut` ready to feed into `handle()`.
//! The corresponding [`DatagramStats`] counter is noted in each doc comment.

use bytes::BytesMut;

/// Too short to parse as any valid QUIC packet.
///
/// Hits: `DatagramStats::malformed_header`
pub(super) fn malformed() -> BytesMut {
    BytesMut::from(&[0xFF][..])
}

/// Long header with a version not in `supported_versions`.
/// A server endpoint responds with a Version Negotiation packet.
///
/// Hits: `DatagramStats::version_negotiation`
pub(super) fn bad_version() -> BytesMut {
    let mut buf = vec![0u8; 21];
    buf[0] = 0x80; // Long header
    buf[1..5].copy_from_slice(&0x0a1a_2a3au32.to_be_bytes()); // bogus version
    buf[5] = 4; // DCID length
    // bytes 6..10: DCID (zeros)
    buf[10] = 4; // SCID length
    // bytes 11..15: SCID (zeros)
    buf[15..21].fill(0); // minimal payload
    BytesMut::from(&buf[..])
}

/// Handshake-type long header with valid QUIC v1 version but an unknown CID.
/// Forces full long-header parsing before rejection.
///
/// Hits: `DatagramStats::unknown_connection_long_header`
pub(super) fn handshake_unknown_cid() -> BytesMut {
    let mut buf = vec![0u8; 64];
    buf[0] = 0xE3; // Long header, Handshake type, PN length 4
    buf[1..5].copy_from_slice(&1u32.to_be_bytes()); // QUIC v1
    buf[5] = 8; // DCID length
    // bytes 6..14: zero DCID (no matching connection)
    buf[14] = 0; // SCID length
    let payload_len = (64 - 17) as u16;
    buf[15] = 0x40 | ((payload_len >> 8) as u8);
    buf[16] = (payload_len & 0xFF) as u8;
    BytesMut::from(&buf[..])
}

/// Valid QUIC v1 Initial header but under the 1200-byte minimum datagram size.
/// Rejected before any crypto operations.
///
/// Hits: `DatagramStats::initial_too_short`
pub(super) fn initial_too_short() -> BytesMut {
    let len = 100usize;
    let mut buf = vec![0u8; len];
    buf[0] = 0xC3; // Long header, Initial type, PN length 4
    buf[1..5].copy_from_slice(&1u32.to_be_bytes());
    buf[5] = 8; // DCID length
    buf[14] = 0; // SCID length
    buf[15] = 0; // Token length (1-byte varint 0)
    let payload_len = (len - 18) as u16;
    buf[16] = 0x40 | ((payload_len >> 8) as u8);
    buf[17] = (payload_len & 0xFF) as u8;
    BytesMut::from(&buf[..])
}

/// Full 1200-byte QUIC v1 Initial with garbage payload.
/// Forces key derivation from the DCID and AEAD decryption attempt.
/// Depending on the derived keys, this hits either `initial_decrypt_failed`
/// (AEAD tag mismatch) or `initial_reserved_bits` (header unprotection
/// produces invalid reserved bits before AEAD runs).
///
/// Hits: `DatagramStats::initial_decrypt_failed` or `DatagramStats::initial_reserved_bits`
pub(super) fn initial_garbage_payload() -> BytesMut {
    let mut buf = vec![0u8; 1200];
    buf[0] = 0xC3;
    buf[1..5].copy_from_slice(&1u32.to_be_bytes());
    buf[5] = 8;
    buf[6..14].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    buf[14] = 0; // SCID length
    buf[15] = 0; // Token length
    let payload_len = 1182u16;
    buf[16] = 0x40 | ((payload_len >> 8) as u8);
    buf[17] = (payload_len & 0xFF) as u8;
    // Fill payload with pseudo-random data
    for (i, b) in buf[18..].iter_mut().enumerate() {
        *b = (i.wrapping_mul(7).wrapping_add(13)) as u8;
    }
    BytesMut::from(&buf[..])
}

/// Short header with a CID that fails `HashedConnectionIdGenerator` validation.
/// Requires the endpoint to use `HashedConnectionIdGenerator`.
///
/// Hits: `DatagramStats::invalid_cid`
pub(super) fn short_header_invalid_cid() -> BytesMut {
    let mut buf = vec![0u8; 64];
    buf[0] = 0x43; // Short header, fixed bit set
    buf[1..9].copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
    BytesMut::from(&buf[..])
}

/// Short header with no CID bytes. Requires the endpoint to use a 0-length CID
/// generator (`RandomConnectionIdGenerator::new(0)`), so the parser extracts an
/// empty CID that passes validation but matches no connection.
///
/// Hits: `DatagramStats::empty_cid_unrecognized`
pub(super) fn short_header_empty_cid() -> BytesMut {
    // Short header, fixed bit set. With 0-length CIDs, byte 1+ is payload.
    BytesMut::from(&[0x40u8; 32][..])
}

/// 1200-byte QUIC v1 Initial with a DCID shorter than the RFC9000 §7.2 minimum
/// of 8 bytes. Passes crypto key derivation and AEAD, but `early_validate_first_packet`
/// rejects it with CONNECTION_REFUSED before decryption is attempted.
///
/// Hits: `DatagramStats::initial_refused`
pub(super) fn initial_short_dcid() -> BytesMut {
    let mut buf = vec![0u8; 1200];
    buf[0] = 0xC3; // Long header, Initial type, PN length 4
    buf[1..5].copy_from_slice(&1u32.to_be_bytes()); // QUIC v1
    buf[5] = 4; // DCID length — only 4 bytes, below the 8-byte minimum
    // bytes 6..10: short DCID
    buf[6..10].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    buf[10] = 0; // SCID length
    buf[11] = 0; // Token length
    // Payload length: 1200 - 14 header bytes = 1186
    let payload_len = 1186u16;
    buf[12] = 0x40 | ((payload_len >> 8) as u8);
    buf[13] = (payload_len & 0xFF) as u8;
    // Payload is garbage — doesn't matter, rejected before decryption
    for (i, b) in buf[14..].iter_mut().enumerate() {
        *b = (i.wrapping_mul(11).wrapping_add(3)) as u8;
    }
    BytesMut::from(&buf[..])
}

/// Short header with a CID that passes `RandomConnectionIdGenerator` validation
/// (any 8-byte CID passes). Routes to the stateless reset path for unknown connections.
///
/// Hits: `DatagramStats::stateless_reset_sent` (first use) or
///       `DatagramStats::stateless_reset_suppressed` (within rate limit interval)
pub(super) fn short_header_valid_cid() -> BytesMut {
    BytesMut::from(&[0x40u8; 1024][..])
}
