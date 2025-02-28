// Copyright (c) 2022, Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use base64ct::{Base64, Encoding};
use ed25519_consensus::{batch, VerificationKeyBytes};
use once_cell::sync::OnceCell;
use serde::{de, Deserialize, Serialize};
use signature::{rand_core::OsRng, Signature, Signer, Verifier};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

use crate::{
    pubkey_bytes::PublicKeyBytes,
    serde_helpers::keypair_decode_base64,
    traits::{
        AggregateAuthenticator, Authenticator, EncodeDecodeBase64, KeyPair, SigningKey,
        ToFromBytes, VerifyingKey,
    },
};

pub const ED25519_PRIVATE_KEY_LENGTH: usize = 32;
pub const ED25519_PUBLIC_KEY_LENGTH: usize = 32;
pub const ED25519_SIGNATURE_LENGTH: usize = 64;

///
/// Define Structs
///

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519PublicKey(pub ed25519_consensus::VerificationKey);

pub type Ed25519PublicKeyBytes = PublicKeyBytes<Ed25519PublicKey, { Ed25519PublicKey::LENGTH }>;

#[derive(Debug)]
pub struct Ed25519PrivateKey(pub ed25519_consensus::SigningKey);

// There is a strong requirement for this specific impl. in Fab benchmarks
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")] // necessary so as not to deser under a != type
pub struct Ed25519KeyPair {
    name: Ed25519PublicKey,
    secret: Ed25519PrivateKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519Signature {
    pub sig: ed25519_consensus::Signature,
    pub bytes: OnceCell<[u8; ED25519_SIGNATURE_LENGTH]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Ed25519AggregateSignature(pub Vec<ed25519_consensus::Signature>);

///
/// Implement VerifyingKey
///

impl<'a> From<&'a Ed25519PrivateKey> for Ed25519PublicKey {
    fn from(secret: &'a Ed25519PrivateKey) -> Self {
        Ed25519PublicKey(secret.0.verification_key())
    }
}

impl VerifyingKey for Ed25519PublicKey {
    type PrivKey = Ed25519PrivateKey;
    type Sig = Ed25519Signature;
    const LENGTH: usize = ED25519_PUBLIC_KEY_LENGTH;

    fn verify_batch(msg: &[u8], pks: &[Self], sigs: &[Self::Sig]) -> Result<(), signature::Error> {
        let mut batch = batch::Verifier::new();

        for i in 0..sigs.len() {
            let vk_bytes = VerificationKeyBytes::try_from(pks[i].as_ref()).unwrap();
            batch.queue((vk_bytes, sigs[i].sig, msg))
        }
        batch
            .verify(&mut OsRng)
            .map_err(|_| signature::Error::new())
    }
}

impl Verifier<Ed25519Signature> for Ed25519PublicKey {
    // Compliant to ZIP215: https://zips.z.cash/protocol/protocol.pdf#concreteed25519
    fn verify(&self, msg: &[u8], signature: &Ed25519Signature) -> Result<(), signature::Error> {
        self.0
            .verify(&signature.sig, msg)
            .map_err(|_| signature::Error::new())
    }
}

impl ToFromBytes for Ed25519PublicKey {
    fn from_bytes(bytes: &[u8]) -> Result<Self, signature::Error> {
        ed25519_consensus::VerificationKey::try_from(bytes)
            .map(Ed25519PublicKey)
            .map_err(|_| signature::Error::new())
    }
}

impl AsRef<[u8]> for Ed25519PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Default for Ed25519PublicKey {
    fn default() -> Self {
        Ed25519PublicKey::from_bytes(&[0u8; 32]).unwrap()
    }
}

impl Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", Base64::encode_string(self.0.as_bytes()))
    }
}

/// Missing in ed25519_consensus
#[allow(clippy::derive_hash_xor_eq)] // ed25519_consensus's PartialEq is compatible
impl std::hash::Hash for Ed25519PublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state);
    }
}

impl PartialOrd for Ed25519PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.as_bytes().partial_cmp(other.0.as_bytes())
    }
}

impl Ord for Ed25519PublicKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

// There is a strong requirement for this specific impl. in Fab benchmarks
impl Serialize for Ed25519PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = self.encode_base64();
        serializer.serialize_newtype_struct("Ed25519PublicKey", &str)
    }
}

// There is a strong requirement for this specific impl. in Fab benchmarks
impl<'de> Deserialize<'de> for Ed25519PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = Self::decode_base64(&s).map_err(|e| de::Error::custom(e.to_string()))?;
        Ok(value)
    }
}

///
/// Implement SigningKey
///

impl SigningKey for Ed25519PrivateKey {
    type PubKey = Ed25519PublicKey;
    type Sig = Ed25519Signature;
    const LENGTH: usize = ED25519_PRIVATE_KEY_LENGTH;
}

impl ToFromBytes for Ed25519PrivateKey {
    fn from_bytes(bytes: &[u8]) -> Result<Self, signature::Error> {
        ed25519_consensus::SigningKey::try_from(bytes)
            .map(Ed25519PrivateKey)
            .map_err(|_| signature::Error::new())
    }
}

// There is a strong requirement for this specific impl. in Fab benchmarks
impl Serialize for Ed25519PrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = self.encode_base64();
        serializer.serialize_newtype_struct("Ed25519PublicKey", &str)
    }
}

// There is a strong requirement for this specific impl. in Fab benchmarks
impl<'de> Deserialize<'de> for Ed25519PrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = Self::decode_base64(&s).map_err(|e| de::Error::custom(e.to_string()))?;
        Ok(value)
    }
}

///
/// Implement Authenticator
///

impl Authenticator for Ed25519Signature {
    type PubKey = Ed25519PublicKey;
    type PrivKey = Ed25519PrivateKey;
    const LENGTH: usize = ED25519_SIGNATURE_LENGTH;
}

impl AsRef<[u8]> for Ed25519PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Signature for Ed25519Signature {
    fn from_bytes(bytes: &[u8]) -> Result<Self, signature::Error> {
        ed25519_consensus::Signature::try_from(bytes)
            .map(|sig| Ed25519Signature {
                sig,
                bytes: OnceCell::new(),
            })
            .map_err(|_| signature::Error::new())
    }
}

impl AsRef<[u8]> for Ed25519Signature {
    fn as_ref(&self) -> &[u8] {
        self.bytes
            .get_or_try_init::<_, eyre::Report>(|| Ok(self.sig.to_bytes()))
            .expect("OnceCell invariant violated")
    }
}

impl Display for Ed25519Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", Base64::encode_string(self.as_ref()))
    }
}

impl Default for Ed25519Signature {
    fn default() -> Self {
        <Ed25519Signature as Signature>::from_bytes(&[1u8; ED25519_SIGNATURE_LENGTH]).unwrap()
    }
}

impl Serialize for Ed25519Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            base64ct::Base64::encode_string(self.as_ref()).serialize(serializer)
        } else {
            self.as_ref().serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for Ed25519Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            base64ct::Base64::decode_vec(&s).map_err(|e| de::Error::custom(e.to_string()))?
        } else {
            Vec::deserialize(deserializer)?
        };
        <Ed25519Signature as signature::Signature>::from_bytes(&bytes)
            .map_err(|e| de::Error::custom(e.to_string()))
    }
}

///
/// Implement AggregateAuthenticator
///

impl Display for Ed25519AggregateSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{:?}",
            self.0
                .iter()
                .map(|x| Base64::encode_string(&x.to_bytes()))
                .collect::<Vec<_>>()
        )
    }
}

impl AggregateAuthenticator for Ed25519AggregateSignature {
    type Sig = Ed25519Signature;
    type PrivKey = Ed25519PrivateKey;
    type PubKey = Ed25519PublicKey;

    /// Parse a key from its byte representation
    fn aggregate(signatures: Vec<Self::Sig>) -> Result<Self, signature::Error> {
        Ok(Self(signatures.iter().map(|s| s.sig).collect()))
    }

    fn add_signature(&mut self, signature: Self::Sig) -> Result<(), signature::Error> {
        self.0.push(signature.sig);
        Ok(())
    }

    fn add_aggregate(&mut self, mut signature: Self) -> Result<(), signature::Error> {
        self.0.append(&mut signature.0);
        Ok(())
    }

    fn verify(
        &self,
        pks: &[<Self::Sig as Authenticator>::PubKey],
        message: &[u8],
    ) -> Result<(), signature::Error> {
        if pks.len() != self.0.len() {
            return Err(signature::Error::new());
        }
        let mut batch = batch::Verifier::new();

        for (i, pk) in pks.iter().enumerate() {
            let vk_bytes = VerificationKeyBytes::try_from(pk.0).unwrap();
            batch.queue((vk_bytes, self.0[i], message));
        }

        batch.verify(OsRng).map_err(|_| signature::Error::new())
    }

    fn batch_verify(
        sigs: &[Self],
        pks: &[&[Self::PubKey]],
        messages: &[&[u8]],
    ) -> Result<(), signature::Error> {
        if pks.len() != messages.len() || messages.len() != sigs.len() {
            return Err(signature::Error::new());
        }
        let mut batch = batch::Verifier::new();

        for i in 0..pks.len() {
            if pks[i].len() != sigs[i].0.len() {
                return Err(signature::Error::new());
            }
            for j in 0..pks[i].len() {
                let vk_bytes = VerificationKeyBytes::from(pks[i][j].0);
                batch.queue((vk_bytes, sigs[i].0[j], messages[i]));
            }
        }
        batch.verify(OsRng).map_err(|_| signature::Error::new())
    }
}

///
/// Implement KeyPair
///

impl From<Ed25519PrivateKey> for Ed25519KeyPair {
    fn from(secret: Ed25519PrivateKey) -> Self {
        let name = Ed25519PublicKey::from(&secret);
        Ed25519KeyPair { name, secret }
    }
}

impl EncodeDecodeBase64 for Ed25519KeyPair {
    fn decode_base64(value: &str) -> Result<Self, eyre::Report> {
        keypair_decode_base64(value)
    }

    fn encode_base64(&self) -> String {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.secret.as_ref());
        bytes.extend_from_slice(self.name.as_ref());
        base64ct::Base64::encode_string(&bytes[..])
    }
}

impl KeyPair for Ed25519KeyPair {
    type PubKey = Ed25519PublicKey;
    type PrivKey = Ed25519PrivateKey;
    type Sig = Ed25519Signature;

    fn public(&'_ self) -> &'_ Self::PubKey {
        &self.name
    }

    fn private(self) -> Self::PrivKey {
        self.secret
    }

    #[cfg(feature = "copy_key")]
    fn copy(&self) -> Self {
        Self {
            name: Ed25519PublicKey::from_bytes(self.name.as_ref()).unwrap(),
            secret: Ed25519PrivateKey::from_bytes(self.secret.as_ref()).unwrap(),
        }
    }

    fn generate<R: rand::CryptoRng + rand::RngCore>(rng: &mut R) -> Self {
        let kp = ed25519_consensus::SigningKey::new(rng);
        Ed25519KeyPair {
            name: Ed25519PublicKey(kp.verification_key()),
            secret: Ed25519PrivateKey(kp),
        }
    }
}

impl FromStr for Ed25519KeyPair {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let kp = Self::decode_base64(s).map_err(|e| eyre::eyre!("{}", e.to_string()))?;
        Ok(kp)
    }
}

impl From<ed25519_consensus::SigningKey> for Ed25519KeyPair {
    fn from(kp: ed25519_consensus::SigningKey) -> Self {
        Ed25519KeyPair {
            name: Ed25519PublicKey(kp.verification_key()),
            secret: Ed25519PrivateKey(kp),
        }
    }
}

impl Signer<Ed25519Signature> for Ed25519KeyPair {
    fn try_sign(&self, msg: &[u8]) -> Result<Ed25519Signature, signature::Error> {
        Ok(Ed25519Signature {
            sig: self.secret.0.sign(msg),
            bytes: OnceCell::new(),
        })
    }
}

///
/// Implement VerifyingKeyBytes
///

impl TryFrom<Ed25519PublicKeyBytes> for Ed25519PublicKey {
    type Error = signature::Error;

    fn try_from(bytes: Ed25519PublicKeyBytes) -> Result<Ed25519PublicKey, Self::Error> {
        VerificationKeyBytes::try_from(bytes.as_ref())
            .and_then(ed25519_consensus::VerificationKey::try_from)
            .map(Ed25519PublicKey)
            .map_err(|_| signature::Error::new())
    }
}

impl From<&Ed25519PublicKey> for Ed25519PublicKeyBytes {
    fn from(pk: &Ed25519PublicKey) -> Self {
        Ed25519PublicKeyBytes::new(pk.0.to_bytes())
    }
}
