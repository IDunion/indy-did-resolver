use serde::{Serialize};

#[derive(Serialize, PartialEq, Debug)]
pub struct DidDocument {
    pub id: String,
    #[serde(rename = "verification_method")]
    pub verification_method: Vec<Ed25519VerificationKey2018>,
    pub authentication: Vec<String>,
}

#[derive(Serialize, PartialEq, Debug)]
pub struct Ed25519VerificationKey2018 {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub controller: String,
    #[serde(rename = "publicKeyBase58")]
    pub public_key_base58: String,
}