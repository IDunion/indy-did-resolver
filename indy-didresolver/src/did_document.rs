use crate::responses::Endpoint;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const LEGACY_INDY_SERVICE: &str = "endpoint";

#[derive(Serialize, PartialEq, Debug)]
pub struct DidDocumentJson {
    pub id: String,
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<Ed25519VerificationKey2018>,
    pub authentication: Vec<String>,
    pub service: Option<Vec<Service>>,
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Ed25519VerificationKey2018 {
    pub id: String,
    pub type_: String,
    pub controller: String,
    pub public_key_base58: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DidCommService {
    pub id: String,
    pub type_: String,
    pub recipient_keys: Vec<String>,
    pub routing_keys: Vec<String>,
}

impl DidCommService {
    pub fn new(id: String, recipient_keys: Vec<String>, routing_keys: Vec<String>) -> Self {
        Self {
            id,
            type_: "did-communication".to_string(),
            recipient_keys,
            routing_keys,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GenericService {
    pub id: String,
    pub type_: String,
    pub service_endpoint: String,
}

#[derive(Serialize, PartialEq, Debug)]
pub enum Service {
    GenericService(GenericService),
    DidCommService(DidCommService),
}

pub struct DidDocument {
    namespace: String,
    id: String,
    verkey: String,
    endpoint: Option<Endpoint>,
    diddoc_content: Option<Value>,
}

pub fn expand_verkey(id: &str, verkey: &str) -> String {
    if verkey.starts_with('~') {
        format!("{}{}", id, &verkey[1..])
    } else {
        verkey.to_string()
    }
}

impl DidDocument {
    pub fn new(
        namespace: &str,
        id: &str,
        verkey: &str,
        endpoint: Option<Endpoint>,
        diddoc_content: Option<Value>,
    ) -> Self {
        DidDocument {
            namespace: namespace.to_string(),
            id: id.to_string(),
            verkey: expand_verkey(id, verkey),
            endpoint,
            diddoc_content,
        }
    }

    pub fn to_string(&self) -> String {
        let mut doc = json!({
             "id": format!("did:indy:{}:{}", self.namespace, self.id),
            "verificationMethod": [Ed25519VerificationKey2018 {
                id: format!("did:indy:{}:{}#keys-1", self.namespace, self.id),
                type_: format!("Ed25519VerificationKey2018"),
                controller: format!("did:indy:{}:{}", self.namespace, self.id),
                public_key_base58: format!("{}", self.verkey),
            }],
            "authentication": [format!("did:indy:{}:{}#keys-1", self.namespace, self.id)],
        });

        if self.diddoc_content.is_some() {
            //TODO: merge base doc with diddoc content
        
        // Handling of legacy services
        } else if self.endpoint.is_some() {
            let mut services = Vec::new();
            let endpoints = self.endpoint.clone();
            for (service, service_endpoint) in endpoints.unwrap().endpoint.into_iter() {
                let s = match service.as_str() {
                    LEGACY_INDY_SERVICE => json!(DidCommService::new(
                        format!("did:indy:{}:{}#did-communication", self.namespace, self.id),
                        vec![format!("did:indy:{}:{}#keys-1", self.namespace, self.id)],
                        vec![],
                    )),
                    type_ => json!(GenericService {
                        id: format!("did:indy:{}:{}#{}", self.namespace, self.id, type_),
                        type_: type_.to_string(),
                        service_endpoint,
                    }),
                };
                services.push(s);
            }

            if let Value::Object(ref mut map) = doc {
                map.insert(
                    "service".to_string(),
                    serde_json::Value::Array(services)
                );
            }
        }

        serde_json::to_string_pretty(&doc).unwrap()
    }
}
