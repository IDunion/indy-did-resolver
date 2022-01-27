use serde::{Serialize};

#[derive(Serialize, PartialEq, Debug)]
pub struct DidDocumentJson {
    pub id: String,
    #[serde(rename = "verificationMethod")]
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

pub struct DidDocument {
    namespace : String,
    id : String,
    verkey : String,
}

pub fn expand_verkey(id: &str, verkey: &str) -> String {
    let mut verkey_cpy = verkey.to_string();
    if verkey_cpy.starts_with('~') {
        verkey_cpy.remove(0);
        return format!("{}{}", id.to_string(), verkey_cpy);
    }
    return verkey_cpy;
}

impl DidDocument {
    pub fn new(namespace: &str, id : &str, verkey : &str) -> Self {
        DidDocument {
            namespace: namespace.to_string(),
            id: id.to_string(),
            verkey: expand_verkey(id, verkey),
        }
    }

    pub fn to_string(&self) -> String{
        let did_document = DidDocumentJson {
            id: format!("did:indy:{}:{}", self.namespace, self.id),
            verification_method: vec![Ed25519VerificationKey2018 {
                id: format!("did:indy:{}:{}#keys-1", self.namespace, self.id),
                type_: format!("Ed25519VerificationKey2018"),
                controller: format!("did:indy:{}:{}", self.namespace, self.id),
                public_key_base58: format!("{}",self.verkey),
            }],
            authentication: vec![
                format!("did:indy:{}:{}#keys-1", self.namespace, self.id),
            ]
        };

        serde_json::to_string_pretty(&did_document).unwrap()
    }

}