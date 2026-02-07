use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crypto_box::aead::{Aead, AeadCore, OsRng};
use crypto_box::{PublicKey as X25519PublicKey, SalsaBox, SecretKey as X25519SecretKey};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_cbor::Value;

pub const MAX_CLOCK_SKEW_MS: u64 = 30_000;
pub const MAX_ID_TIMESTAMP_DELTA_MS: u64 = 1_000;

pub const TYPE_PING: u8 = 0x01;
pub const TYPE_PONG: u8 = 0x02;
pub const TYPE_ACK: u8 = 0x03;
pub const TYPE_MESSAGE: u8 = 0x10;
pub const TYPE_HELLO: u8 = 0x70;
pub const TYPE_HELLO_ACK: u8 = 0x71;
pub const TYPE_HELLO_REJECT: u8 = 0x72;
pub const MAX_FRAME_SIZE: usize = 8 * 1024 * 1024;
pub const TRANSPORT_WRAPPER_VERSION_V1: u64 = 1;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_millis() as u64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmpError {
    pub code: u16,
    pub name: &'static str,
    pub detail: String,
}

impl AmpError {
    pub fn invalid_message(detail: impl Into<String>) -> Self {
        Self {
            code: 1001,
            name: "INVALID_MESSAGE",
            detail: detail.into(),
        }
    }

    pub fn invalid_signature(detail: impl Into<String>) -> Self {
        Self {
            code: 1002,
            name: "INVALID_SIGNATURE",
            detail: detail.into(),
        }
    }

    pub fn invalid_timestamp(detail: impl Into<String>) -> Self {
        Self {
            code: 1003,
            name: "INVALID_TIMESTAMP",
            detail: detail.into(),
        }
    }

    pub fn unsupported_version(detail: impl Into<String>) -> Self {
        Self {
            code: 1004,
            name: "UNSUPPORTED_VERSION",
            detail: detail.into(),
        }
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self {
            code: 3001,
            name: "UNAUTHORIZED",
            detail: detail.into(),
        }
    }
}

impl fmt::Display for AmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.code, self.name, self.detail)
    }
}

impl std::error::Error for AmpError {}

#[derive(Debug, Clone)]
pub struct AgentKeys {
    pub did: String,
    pub signing_key: SigningKey,
    pub signing_public_key: VerifyingKey,
    pub key_agreement_secret: X25519SecretKey,
    pub key_agreement_public: X25519PublicKey,
}

impl AgentKeys {
    pub fn from_seeds(
        did: impl Into<String>,
        sign_seed: [u8; 32],
        key_agreement_seed: [u8; 32],
    ) -> Self {
        let signing_key = SigningKey::from_bytes(&sign_seed);
        let signing_public_key = signing_key.verifying_key();
        let key_agreement_secret = X25519SecretKey::from(key_agreement_seed);
        let key_agreement_public = key_agreement_secret.public_key();

        Self {
            did: did.into(),
            signing_key,
            signing_public_key,
            key_agreement_secret,
            key_agreement_public,
        }
    }

    pub fn from_sign_seed(did: impl Into<String>, sign_seed: [u8; 32]) -> Self {
        Self::from_seeds(did, sign_seed, sign_seed)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DidResolver {
    signing: HashMap<String, VerifyingKey>,
    key_agreement: HashMap<String, X25519PublicKey>,
    trusted_relays: HashSet<String>,
}

impl DidResolver {
    pub fn add_agent(&mut self, agent: &AgentKeys) {
        self.signing
            .insert(agent.did.clone(), agent.signing_public_key);
        self.key_agreement
            .insert(agent.did.clone(), agent.key_agreement_public.clone());
    }

    pub fn add_trusted_relay(&mut self, relay_did: impl Into<String>) {
        self.trusted_relays.insert(relay_did.into());
    }

    pub fn signing_key_for(&self, did: &str) -> Option<VerifyingKey> {
        self.signing.get(did).cloned()
    }

    pub fn key_agreement_for(&self, did: &str) -> Option<X25519PublicKey> {
        self.key_agreement.get(did).cloned()
    }

    pub fn is_trusted_relay(&self, did: &str) -> bool {
        self.trusted_relays.contains(did)
    }
}

pub const DEMO_ALICE_DID: &str = "did:web:example.com:agent:alice";
pub const DEMO_BOB_DID: &str = "did:web:example.com:agent:bob";
pub const DEMO_RELAY_DID: &str = "did:web:example.com:relay:main";

#[derive(Debug, Clone)]
pub struct DemoAgents {
    pub alice: AgentKeys,
    pub bob: AgentKeys,
    pub relay: AgentKeys,
}

impl DemoAgents {
    pub fn resolver(&self) -> DidResolver {
        let mut resolver = DidResolver::default();
        resolver.add_agent(&self.alice);
        resolver.add_agent(&self.bob);
        resolver.add_agent(&self.relay);
        resolver.add_trusted_relay(self.relay.did.clone());
        resolver
    }

    pub fn by_name(&self, name: &str) -> Option<AgentKeys> {
        match name {
            "alice" => Some(self.alice.clone()),
            "bob" => Some(self.bob.clone()),
            "relay" => Some(self.relay.clone()),
            _ => None,
        }
    }

    pub fn did_for_alias(&self, alias_or_did: &str) -> String {
        match alias_or_did {
            "alice" => self.alice.did.clone(),
            "bob" => self.bob.did.clone(),
            "relay" => self.relay.did.clone(),
            _ => alias_or_did.to_string(),
        }
    }
}

pub fn demo_agents() -> DemoAgents {
    let alice = AgentKeys::from_seeds(DEMO_ALICE_DID, [1_u8; 32], [11_u8; 32]);
    let bob = AgentKeys::from_seeds(DEMO_BOB_DID, [2_u8; 32], [12_u8; 32]);
    let relay = AgentKeys::from_seeds(DEMO_RELAY_DID, [3_u8; 32], [13_u8; 32]);
    DemoAgents { alice, bob, relay }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Recipients {
    One(String),
    Many(Vec<String>),
}

impl Recipients {
    pub fn contains(&self, did: &str) -> bool {
        match self {
            Recipients::One(v) => v == did,
            Recipients::Many(vs) => vs.iter().any(|v| v == did),
        }
    }

    pub fn as_vec(&self) -> Vec<String> {
        match self {
            Recipients::One(v) => vec![v.clone()],
            Recipients::Many(vs) => vs.clone(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Recipients::One(_) => 1,
            Recipients::Many(vs) => vs.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageMeta {
    pub v: u64,
    pub id: [u8; 16],
    pub typ: u8,
    pub ts_ms: u64,
    pub ttl_ms: u64,
    pub from: String,
    pub to: Recipients,
    pub reply_to: Option<[u8; 16]>,
    pub thread_id: Option<Vec<u8>>,
}

impl MessageMeta {
    pub fn is_handshake(&self) -> bool {
        matches!(self.typ, TYPE_HELLO | TYPE_HELLO_ACK | TYPE_HELLO_REJECT)
    }
}

#[derive(Debug, Clone)]
pub struct RoutingEnvelope {
    pub id: [u8; 16],
    pub typ: u8,
    pub from: String,
    pub to: Vec<String>,
    pub reply_to: Option<[u8; 16]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireEncryptedPayload {
    alg: String,
    mode: String,
    nonce: ByteBuf,
    ciphertext: ByteBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WirePlainMessage {
    v: u64,
    id: ByteBuf,
    typ: u8,
    ts: u64,
    ttl: u64,
    from: String,
    to: Recipients,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<ByteBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_id: Option<ByteBuf>,
    sig: ByteBuf,
    body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireEncryptedMessage {
    v: u64,
    id: ByteBuf,
    typ: u8,
    ts: u64,
    ttl: u64,
    from: String,
    to: Recipients,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<ByteBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_id: Option<ByteBuf>,
    sig: ByteBuf,
    enc: WireEncryptedPayload,
}

#[derive(Debug, Clone)]
enum InboundWire {
    Plain(WirePlainMessage),
    Encrypted(WireEncryptedMessage),
}

#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub meta: MessageMeta,
    pub sig: Vec<u8>,
    pub body_bytes: Vec<u8>,
}

impl ReceivedMessage {
    pub fn decode_body<T: DeserializeOwned>(&self) -> Result<T, AmpError> {
        serde_cbor::from_slice(&self.body_bytes)
            .map_err(|e| AmpError::invalid_message(format!("body decode failed: {e}")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AckSource {
    Relay,
    Recipient,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckBody {
    pub ack_source: AckSource,
    pub received_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HelloBody {
    pub versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextMessageBody {
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PollResponse {
    pub messages: Vec<ByteBuf>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferMode {
    Single,
    Dual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayForward {
    pub fwd_v: u64,
    pub message: ByteBuf,
    pub from_did: String,
    pub recipient_did: String,
    pub relay_path: Vec<String>,
    pub hop_limit: u64,
    pub upstream_relay: String,
    pub transfer_mode: TransferMode,
}

pub fn make_message_id(ts_ms: u64, random_tail: u64) -> [u8; 16] {
    let mut id = [0_u8; 16];
    id[..8].copy_from_slice(&ts_ms.to_be_bytes());
    id[8..].copy_from_slice(&random_tail.to_be_bytes());
    id
}

pub fn message_id_timestamp_ms(id: &[u8; 16]) -> u64 {
    let mut ts_bytes = [0_u8; 8];
    ts_bytes.copy_from_slice(&id[..8]);
    u64::from_be_bytes(ts_bytes)
}

pub fn build_plain_signed<T: Serialize>(
    sender: &AgentKeys,
    mut meta: MessageMeta,
    body: &T,
) -> Result<Vec<u8>, AmpError> {
    meta.from = sender.did.clone();
    validate_meta(&meta, meta.ts_ms)?;

    let body_bytes = to_cbor_deterministic(body)?;
    let body_value: Value = serde_cbor::from_slice(&body_bytes)
        .map_err(|e| AmpError::invalid_message(format!("body -> value failed: {e}")))?;
    let sig_input = sig_input_bytes(&meta, &body_bytes)?;
    let sig = sender.signing_key.sign(&sig_input).to_bytes().to_vec();

    let wire = WirePlainMessage {
        v: meta.v,
        id: ByteBuf::from(meta.id.to_vec()),
        typ: meta.typ,
        ts: meta.ts_ms,
        ttl: meta.ttl_ms,
        from: meta.from,
        to: meta.to,
        reply_to: meta.reply_to.map(|v| ByteBuf::from(v.to_vec())),
        thread_id: meta.thread_id.map(ByteBuf::from),
        sig: ByteBuf::from(sig),
        body: body_value,
    };

    to_cbor_deterministic(&wire)
}

pub fn build_authcrypt_signed<T: Serialize>(
    sender: &AgentKeys,
    recipient_did: &str,
    mut meta: MessageMeta,
    body: &T,
    resolver: &DidResolver,
) -> Result<Vec<u8>, AmpError> {
    meta.from = sender.did.clone();
    validate_meta(&meta, meta.ts_ms)?;

    let recipient_pk = resolver
        .key_agreement_for(recipient_did)
        .ok_or_else(|| AmpError::unauthorized("recipient keyAgreement key not found"))?;

    let body_bytes = to_cbor_deterministic(body)?;
    let sig_input = sig_input_bytes(&meta, &body_bytes)?;
    let sig = sender.signing_key.sign(&sig_input).to_bytes().to_vec();

    let cipher = SalsaBox::new(&recipient_pk, &sender.key_agreement_secret);
    let nonce = SalsaBox::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, body_bytes.as_ref())
        .map_err(|_| AmpError::unauthorized("authcrypt encryption failed"))?;

    let wire = WireEncryptedMessage {
        v: meta.v,
        id: ByteBuf::from(meta.id.to_vec()),
        typ: meta.typ,
        ts: meta.ts_ms,
        ttl: meta.ttl_ms,
        from: meta.from,
        to: meta.to,
        reply_to: meta.reply_to.map(|v| ByteBuf::from(v.to_vec())),
        thread_id: meta.thread_id.map(ByteBuf::from),
        sig: ByteBuf::from(sig),
        enc: WireEncryptedPayload {
            alg: "X25519-XSalsa20-Poly1305".to_string(),
            mode: "authcrypt".to_string(),
            nonce: ByteBuf::from(nonce.to_vec()),
            ciphertext: ByteBuf::from(ciphertext),
        },
    };

    to_cbor_deterministic(&wire)
}

pub fn receive_and_verify(
    recipient: &AgentKeys,
    wire_bytes: &[u8],
    resolver: &DidResolver,
    now_ms: u64,
) -> Result<ReceivedMessage, AmpError> {
    match parse_wire(wire_bytes)? {
        InboundWire::Plain(wire) => {
            let meta = wire_plain_to_meta(&wire)?;
            validate_meta(&meta, now_ms)?;
            ensure_recipient_matches(&meta.to, &recipient.did)?;

            let body_bytes = to_cbor_deterministic(&wire.body)?;
            if matches!(meta.typ, TYPE_PING | TYPE_PONG) && body_bytes != [0xF6] {
                return Err(AmpError::invalid_message(
                    "PING/PONG body must be CBOR null (0xF6)",
                ));
            }

            verify_signature(&meta, wire.sig.as_ref(), &body_bytes, resolver)?;

            Ok(ReceivedMessage {
                meta,
                sig: wire.sig.into_vec(),
                body_bytes,
            })
        }
        InboundWire::Encrypted(wire) => {
            let meta = wire_encrypted_to_meta(&wire)?;
            validate_meta(&meta, now_ms)?;
            ensure_recipient_matches(&meta.to, &recipient.did)?;
            validate_encrypted_fields(&wire.enc)?;

            let sender_kak = resolver
                .key_agreement_for(&meta.from)
                .ok_or_else(|| AmpError::unauthorized("sender keyAgreement key not found"))?;

            let nonce = crypto_box::aead::generic_array::GenericArray::from_slice(wire.enc.nonce.as_ref());
            let cipher = SalsaBox::new(&sender_kak, &recipient.key_agreement_secret);
            let body_bytes = cipher
                .decrypt(nonce, wire.enc.ciphertext.as_ref())
                .map_err(|_| AmpError::unauthorized("authcrypt decrypt failed"))?;

            // Encrypted path uses decrypted bytes directly for signature verification.
            verify_signature(&meta, wire.sig.as_ref(), &body_bytes, resolver)?;

            // Fail if decrypted bytes are not valid CBOR.
            let _: Value = serde_cbor::from_slice(&body_bytes)
                .map_err(|e| AmpError::invalid_message(format!("decrypted body is not CBOR: {e}")))?;

            Ok(ReceivedMessage {
                meta,
                sig: wire.sig.into_vec(),
                body_bytes,
            })
        }
    }
}

pub fn validate_ack_semantics(
    ack: &ReceivedMessage,
    original_to: &[String],
    resolver: &DidResolver,
) -> Result<(), AmpError> {
    if ack.meta.typ != TYPE_ACK {
        return Err(AmpError::invalid_message("message typ is not ACK"));
    }

    let body: AckBody = ack.decode_body()?;
    match body.ack_source {
        AckSource::Recipient => {
            if !original_to.iter().any(|did| did == &ack.meta.from) {
                return Err(AmpError::invalid_message(
                    "ack_source=recipient requires from in original to",
                ));
            }
        }
        AckSource::Relay => {
            if !resolver.is_trusted_relay(&ack.meta.from) {
                return Err(AmpError::unauthorized(
                    "ack_source=relay requires trusted relay DID",
                ));
            }
        }
    }

    if original_to.len() > 1 && body.ack_target.is_none() {
        return Err(AmpError::invalid_message(
            "multi-recipient ACK should include ack_target",
        ));
    }

    Ok(())
}

pub fn select_compatible_version(local_supported: &[String], peer_versions: &[String]) -> Option<String> {
    let peer_majors: HashSet<u64> = peer_versions
        .iter()
        .filter_map(|v| major_of_semver(v))
        .collect();

    local_supported
        .iter()
        .find(|v| major_of_semver(v).is_some_and(|m| peer_majors.contains(&m)))
        .cloned()
}

pub fn major_of_semver(version: &str) -> Option<u64> {
    let major = version.split('.').next()?;
    if major.is_empty() {
        return None;
    }
    major.parse::<u64>().ok()
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

pub fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    if payload.len() > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "payload exceeds MAX_FRAME_SIZE",
        ));
    }

    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

pub fn read_frame<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut len_buf = [0_u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame exceeds MAX_FRAME_SIZE",
        ));
    }

    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

pub fn peek_routing(wire_bytes: &[u8]) -> Result<RoutingEnvelope, AmpError> {
    match parse_wire(wire_bytes)? {
        InboundWire::Plain(wire) => {
            let meta = wire_plain_to_meta(&wire)?;
            Ok(RoutingEnvelope {
                id: meta.id,
                typ: meta.typ,
                from: meta.from,
                to: meta.to.as_vec(),
                reply_to: meta.reply_to,
            })
        }
        InboundWire::Encrypted(wire) => {
            let meta = wire_encrypted_to_meta(&wire)?;
            Ok(RoutingEnvelope {
                id: meta.id,
                typ: meta.typ,
                from: meta.from,
                to: meta.to.as_vec(),
                reply_to: meta.reply_to,
            })
        }
    }
}

pub fn decode_ws_binary_message_unit(payload: &[u8]) -> Result<RoutingEnvelope, AmpError> {
    if payload.is_empty() {
        return Err(AmpError::invalid_message(
            "websocket binary payload must not be empty",
        ));
    }
    peek_routing(payload)
}

pub fn reject_ws_text_message() -> AmpError {
    AmpError::invalid_message("websocket text messages are not supported for AMP")
}

pub fn decode_poll_response(bytes: &[u8]) -> Result<PollResponse, AmpError> {
    let wrapper: PollResponse = serde_cbor::from_slice(bytes)
        .map_err(|e| AmpError::invalid_message(format!("invalid poll wrapper: {e}")))?;

    for (idx, raw_msg) in wrapper.messages.iter().enumerate() {
        parse_wire(raw_msg.as_ref()).map_err(|e| {
            AmpError::invalid_message(format!(
                "poll wrapper messages[{idx}] is not a valid AMP message: {e}"
            ))
        })?;
    }

    Ok(wrapper)
}

pub fn decode_relay_forward(bytes: &[u8]) -> Result<RelayForward, AmpError> {
    let wrapper: RelayForward = serde_cbor::from_slice(bytes)
        .map_err(|e| AmpError::invalid_message(format!("invalid relay-forward wrapper: {e}")))?;

    if wrapper.fwd_v != TRANSPORT_WRAPPER_VERSION_V1 {
        return Err(AmpError::unsupported_version(format!(
            "unsupported relay-forward fwd_v={}, expected {}",
            wrapper.fwd_v, TRANSPORT_WRAPPER_VERSION_V1
        )));
    }
    if wrapper.hop_limit == 0 {
        return Err(AmpError::invalid_message(
            "relay-forward hop_limit must be > 0",
        ));
    }
    if wrapper.upstream_relay.is_empty() {
        return Err(AmpError::unauthorized(
            "relay-forward upstream_relay is required",
        ));
    }

    let routing = peek_routing(wrapper.message.as_ref())?;
    if routing.from != wrapper.from_did {
        return Err(AmpError::invalid_message(format!(
            "relay-forward from_did mismatch: wrapper={} message={}",
            wrapper.from_did, routing.from
        )));
    }
    if !routing.to.iter().any(|did| did == &wrapper.recipient_did) {
        return Err(AmpError::invalid_message(format!(
            "relay-forward recipient_did {} not found in message to field",
            wrapper.recipient_did
        )));
    }

    Ok(wrapper)
}

fn parse_wire(bytes: &[u8]) -> Result<InboundWire, AmpError> {
    if let Ok(wire) = serde_cbor::from_slice::<WirePlainMessage>(bytes) {
        return Ok(InboundWire::Plain(wire));
    }
    if let Ok(wire) = serde_cbor::from_slice::<WireEncryptedMessage>(bytes) {
        return Ok(InboundWire::Encrypted(wire));
    }

    Err(AmpError::invalid_message(
        "wire bytes are neither plaintext nor encrypted AMP message",
    ))
}

fn wire_plain_to_meta(wire: &WirePlainMessage) -> Result<MessageMeta, AmpError> {
    Ok(MessageMeta {
        v: wire.v,
        id: to_fixed_16("id", wire.id.as_ref())?,
        typ: wire.typ,
        ts_ms: wire.ts,
        ttl_ms: wire.ttl,
        from: wire.from.clone(),
        to: wire.to.clone(),
        reply_to: wire
            .reply_to
            .as_ref()
            .map(|v| to_fixed_16("reply_to", v.as_ref()))
            .transpose()?,
        thread_id: wire.thread_id.as_ref().map(|v| v.to_vec()),
    })
}

fn wire_encrypted_to_meta(wire: &WireEncryptedMessage) -> Result<MessageMeta, AmpError> {
    Ok(MessageMeta {
        v: wire.v,
        id: to_fixed_16("id", wire.id.as_ref())?,
        typ: wire.typ,
        ts_ms: wire.ts,
        ttl_ms: wire.ttl,
        from: wire.from.clone(),
        to: wire.to.clone(),
        reply_to: wire
            .reply_to
            .as_ref()
            .map(|v| to_fixed_16("reply_to", v.as_ref()))
            .transpose()?,
        thread_id: wire.thread_id.as_ref().map(|v| v.to_vec()),
    })
}

fn verify_signature(
    meta: &MessageMeta,
    sig_bytes: &[u8],
    body_bytes: &[u8],
    resolver: &DidResolver,
) -> Result<(), AmpError> {
    let verifying_key = resolver
        .signing_key_for(&meta.from)
        .ok_or_else(|| AmpError::unauthorized("sender signing key not found"))?;

    if sig_bytes.len() != 64 {
        return Err(AmpError::invalid_signature("ed25519 signature must be 64 bytes"));
    }

    let signature = Signature::from_slice(sig_bytes)
        .map_err(|e| AmpError::invalid_signature(format!("invalid signature bytes: {e}")))?;

    let sig_input = sig_input_bytes(meta, body_bytes)?;
    verifying_key
        .verify(&sig_input, &signature)
        .map_err(|_| AmpError::invalid_signature("signature verification failed"))
}

fn validate_meta(meta: &MessageMeta, now_ms: u64) -> Result<(), AmpError> {
    if meta.v == 0 {
        return Err(AmpError::unsupported_version("v=0 is invalid"));
    }
    if meta.is_handshake() && meta.v != 1 {
        return Err(AmpError::unsupported_version(
            "HELLO/HELLO_ACK/HELLO_REJECT must use v=1",
        ));
    }
    if meta.from.is_empty() {
        return Err(AmpError::invalid_message("from is required"));
    }
    if meta.to.len() == 0 {
        return Err(AmpError::invalid_message("to must not be empty"));
    }
    if meta.ts_ms > now_ms.saturating_add(MAX_CLOCK_SKEW_MS) {
        return Err(AmpError::invalid_timestamp(
            "message ts is too far in the future",
        ));
    }
    if now_ms > meta.ts_ms.saturating_add(meta.ttl_ms) {
        return Err(AmpError::invalid_timestamp("message ttl expired"));
    }

    let id_ts = message_id_timestamp_ms(&meta.id);
    if abs_diff_u64(id_ts, meta.ts_ms) > MAX_ID_TIMESTAMP_DELTA_MS {
        return Err(AmpError::invalid_message(
            "message-id timestamp mismatch (>1s)",
        ));
    }

    Ok(())
}

fn validate_encrypted_fields(enc: &WireEncryptedPayload) -> Result<(), AmpError> {
    if enc.alg != "X25519-XSalsa20-Poly1305" {
        return Err(AmpError::invalid_message(
            "enc.alg must be X25519-XSalsa20-Poly1305",
        ));
    }
    if enc.mode != "authcrypt" {
        return Err(AmpError::invalid_message("enc.mode must be authcrypt"));
    }
    if enc.nonce.len() != 24 {
        return Err(AmpError::invalid_message("enc.nonce must be 24 bytes"));
    }
    if enc.ciphertext.len() < 17 {
        return Err(AmpError::invalid_message(
            "enc.ciphertext must include at least 16-byte tag + 1-byte payload",
        ));
    }

    Ok(())
}

fn ensure_recipient_matches(to: &Recipients, recipient_did: &str) -> Result<(), AmpError> {
    if !to.contains(recipient_did) {
        return Err(AmpError::unauthorized(
            "recipient DID not listed in to field",
        ));
    }
    Ok(())
}

fn sig_input_bytes(meta: &MessageMeta, body_bytes: &[u8]) -> Result<Vec<u8>, AmpError> {
    #[derive(Serialize)]
    struct SigHeaders {
        id: ByteBuf,
        to: Recipients,
        ts: u64,
        ttl: u64,
        typ: u8,
        from: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reply_to: Option<ByteBuf>,
        #[serde(skip_serializing_if = "Option::is_none")]
        thread_id: Option<ByteBuf>,
    }

    let headers = SigHeaders {
        id: ByteBuf::from(meta.id.to_vec()),
        to: meta.to.clone(),
        ts: meta.ts_ms,
        ttl: meta.ttl_ms,
        typ: meta.typ,
        from: meta.from.clone(),
        reply_to: meta.reply_to.map(|v| ByteBuf::from(v.to_vec())),
        thread_id: meta.thread_id.clone().map(ByteBuf::from),
    };

    let sig_input = (
        "AMP-v1",
        1_u8,
        headers,
        ByteBuf::from(body_bytes.to_vec()),
    );

    to_cbor_deterministic(&sig_input)
}

fn to_cbor_deterministic<T: Serialize>(value: &T) -> Result<Vec<u8>, AmpError> {
    serde_cbor::to_vec(value)
        .map_err(|e| AmpError::invalid_message(format!("cbor encode failed: {e}")))
}

fn to_fixed_16(name: &str, bytes: &[u8]) -> Result<[u8; 16], AmpError> {
    if bytes.len() != 16 {
        return Err(AmpError::invalid_message(format!("{name} must be 16 bytes")));
    }

    let mut out = [0_u8; 16];
    out.copy_from_slice(bytes);
    Ok(out)
}

fn abs_diff_u64(a: u64, b: u64) -> u64 {
    a.abs_diff(b)
}

pub fn cbor_map_string_pairs(pairs: &[(&str, Value)]) -> Value {
    let mut map = BTreeMap::new();
    for (k, v) in pairs {
        map.insert(Value::Text((*k).to_string()), v.clone());
    }
    Value::Map(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn setup() -> (AgentKeys, AgentKeys, DidResolver) {
        let alice = AgentKeys::from_sign_seed("did:web:example.com:agent:alice", [1_u8; 32]);
        let bob = AgentKeys::from_sign_seed("did:web:example.com:agent:bob", [2_u8; 32]);

        let mut resolver = DidResolver::default();
        resolver.add_agent(&alice);
        resolver.add_agent(&bob);
        (alice, bob, resolver)
    }

    #[test]
    fn e2e_plain_hello() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_201_000_u64;

        let hello = HelloBody {
            versions: vec!["0.30.0".to_string(), "1.0.0".to_string()],
        };
        let meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 1),
            typ: TYPE_HELLO,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(bob.did.clone()),
            reply_to: None,
            thread_id: None,
        };

        let wire = build_plain_signed(&alice, meta, &hello).expect("build hello");
        let received = receive_and_verify(&bob, &wire, &resolver, ts + 10).expect("receive hello");
        let parsed: HelloBody = received.decode_body().expect("decode hello body");
        assert_eq!(parsed.versions[0], "0.30.0");
    }

    #[test]
    fn e2e_authcrypt_message() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_202_000_u64;

        let body = TextMessageBody {
            msg: "secret payload".to_string(),
        };
        let meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 2),
            typ: TYPE_MESSAGE,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(bob.did.clone()),
            reply_to: None,
            thread_id: None,
        };

        let wire = build_authcrypt_signed(&alice, &bob.did, meta, &body, &resolver).expect("build authcrypt");
        let received = receive_and_verify(&bob, &wire, &resolver, ts + 10).expect("receive encrypted");
        let parsed: TextMessageBody = received.decode_body().expect("decode body");
        assert_eq!(parsed.msg, "secret payload");

        // Encrypted wire should not carry plaintext body field.
        let as_value: Value = serde_cbor::from_slice(&wire).expect("decode cbor value");
        let Value::Map(map) = as_value else {
            panic!("wire must decode to map");
        };
        assert!(!map.contains_key(&Value::Text("body".to_string())));
        assert!(map.contains_key(&Value::Text("enc".to_string())));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_203_000_u64;

        let body = TextMessageBody {
            msg: "top-secret".to_string(),
        };
        let meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 3),
            typ: TYPE_MESSAGE,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(bob.did.clone()),
            reply_to: None,
            thread_id: None,
        };

        let wire = build_authcrypt_signed(&alice, &bob.did, meta, &body, &resolver).expect("build authcrypt");
        let mut tampered: WireEncryptedMessage = serde_cbor::from_slice(&wire).expect("decode encrypted wire");
        tampered.enc.ciphertext[0] ^= 0x01;
        let tampered_wire = to_cbor_deterministic(&tampered).expect("encode tampered wire");

        let err = receive_and_verify(&bob, &tampered_wire, &resolver, ts + 10).unwrap_err();
        assert_eq!(err.code, 3001);
    }

    #[test]
    fn ack_semantics_recipient() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_204_000_u64;

        let ack = AckBody {
            ack_source: AckSource::Recipient,
            received_at: ts,
            ack_target: None,
        };
        let ack_meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 4),
            typ: TYPE_ACK,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(alice.did.clone()),
            reply_to: Some(make_message_id(ts - 1_000, 9)),
            thread_id: None,
        };

        let ack_wire = build_plain_signed(&bob, ack_meta, &ack).expect("build ack");
        let ack_received = receive_and_verify(&alice, &ack_wire, &resolver, ts + 10).expect("receive ack");

        validate_ack_semantics(&ack_received, &[bob.did.clone()], &resolver).expect("ack semantics");
    }

    #[test]
    fn frame_roundtrip() {
        let payload = b"amp-frame-test".to_vec();
        let mut buf = Vec::new();
        write_frame(&mut buf, &payload).expect("write frame");

        let mut cursor = Cursor::new(buf);
        let read = read_frame(&mut cursor).expect("read frame");
        assert_eq!(read, payload);
    }

    #[test]
    fn peek_routing_from_encrypted_message() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_205_000_u64;
        let meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 5),
            typ: TYPE_MESSAGE,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(bob.did.clone()),
            reply_to: None,
            thread_id: None,
        };
        let body = TextMessageBody {
            msg: "peek".to_string(),
        };

        let wire = build_authcrypt_signed(&alice, &bob.did, meta, &body, &resolver).expect("build");
        let routing = peek_routing(&wire).expect("peek");
        assert_eq!(routing.from, alice.did);
        assert_eq!(routing.to, vec![bob.did]);
        assert_eq!(routing.typ, TYPE_MESSAGE);
    }

    #[test]
    fn tc_transport_002_tcp_frame_boundary_checks() {
        let payload = b"amp-transport-002".to_vec();
        let mut framed = Vec::new();
        write_frame(&mut framed, &payload).expect("write frame");
        framed.pop();

        let mut cursor = Cursor::new(framed);
        let err = read_frame(&mut cursor).expect_err("truncated frame must fail");
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn tc_transport_003_websocket_mapping_rules() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_206_000_u64;
        let body = TextMessageBody {
            msg: "ws-binary".to_string(),
        };
        let meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 6),
            typ: TYPE_MESSAGE,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(bob.did.clone()),
            reply_to: None,
            thread_id: None,
        };

        let wire = build_authcrypt_signed(&alice, &bob.did, meta, &body, &resolver).expect("build");
        let routing = decode_ws_binary_message_unit(&wire).expect("ws binary mapping");
        assert_eq!(routing.from, alice.did);
        assert_eq!(routing.to, vec![bob.did]);

        let err = reject_ws_text_message();
        assert_eq!(err.code, 1001);
    }

    #[test]
    fn tc_transport_004_http_wrapper_validation() {
        let (alice, bob, resolver) = setup();
        let ts = 1_707_055_207_000_u64;
        let body = TextMessageBody {
            msg: "http-wrapper".to_string(),
        };
        let meta = MessageMeta {
            v: 1,
            id: make_message_id(ts, 7),
            typ: TYPE_MESSAGE,
            ts_ms: ts,
            ttl_ms: 86_400_000,
            from: String::new(),
            to: Recipients::One(bob.did.clone()),
            reply_to: None,
            thread_id: None,
        };
        let wire = build_authcrypt_signed(&alice, &bob.did, meta, &body, &resolver).expect("build");

        let poll = PollResponse {
            messages: vec![ByteBuf::from(wire.clone())],
            next_cursor: Some("cur-1".to_string()),
            has_more: false,
        };
        let poll_bytes = serde_cbor::to_vec(&poll).expect("encode poll wrapper");
        let decoded_poll = decode_poll_response(&poll_bytes).expect("decode poll wrapper");
        assert_eq!(decoded_poll.messages.len(), 1);

        let relay_forward = RelayForward {
            fwd_v: TRANSPORT_WRAPPER_VERSION_V1,
            message: ByteBuf::from(wire.clone()),
            from_did: alice.did.clone(),
            recipient_did: bob.did.clone(),
            relay_path: vec!["did:web:example.com:relay:a".to_string()],
            hop_limit: 8,
            upstream_relay: "did:web:example.com:relay:a".to_string(),
            transfer_mode: TransferMode::Single,
        };
        let rf_bytes = serde_cbor::to_vec(&relay_forward).expect("encode relay-forward");
        let parsed = decode_relay_forward(&rf_bytes).expect("decode relay-forward");
        assert_eq!(parsed.fwd_v, TRANSPORT_WRAPPER_VERSION_V1);
        assert_eq!(parsed.recipient_did, bob.did);

        let mut unsupported = relay_forward;
        unsupported.fwd_v = 2;
        let unsupported_bytes = serde_cbor::to_vec(&unsupported).expect("encode unsupported");
        let err = decode_relay_forward(&unsupported_bytes).expect_err("fwd_v must be rejected");
        assert_eq!(err.code, 1004);
    }
}
