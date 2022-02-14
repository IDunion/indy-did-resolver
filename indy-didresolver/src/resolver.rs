use std::str::FromStr;

use futures_executor::block_on;
use indy_vdr::utils::Qualifiable;
use serde_json::Value;

use super::did::{did_parse, LedgerObject};
use super::did_document::DidDocument;
use super::error::DidIndyError;
use super::responses::GetNymResult;

use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::constants::GET_NYM;
use indy_vdr::ledger::identifiers::{CredentialDefinitionId, SchemaId};
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::pool::{Pool, PreparedRequest, RequestResult, TimingResult};
use indy_vdr::utils::did::DidValue;

use ssi::did;

pub struct Resolver<T: Pool> {
    pool: T,
}

impl<T: Pool> Resolver<T> {
    pub fn new(pool: T) -> Resolver<T> {
        Resolver { pool }
    }

    pub fn resolve(&self, did: &str) -> Result<String, DidIndyError> {
        let did_url = did::DIDURL::try_from(String::from(did)).expect("Could not parse DID URL");

        let did = did_url.did;
        let did = match did_parse(did.as_str()) {
            Ok(did) => did,
            Err(DidIndyError) => {
                return Err(DidIndyError);
            }
        };

        // The path variable identifies the requested ledger object
        // If there is no path, then we return a DID document
        let path = if did_url.path_abempty != "" {
            Some(did_url.path_abempty.as_str())
        } else {
            None
        };

        println!("{:?}", path);

        let did_value = DidValue::new(&did.id, Option::None);

        let request = self.build_request(&did_value, path)?;

        let ledger_data = handle_request(&self.pool, &request)?;

        let v: Value = serde_json::from_str(&ledger_data).unwrap();
        println!("result: {:?}", v);
        let data: &Value = &v["result"]["data"];
        println!("data: {:?}", data);
        if *data == Value::Null {
            return Err(DidIndyError);
        }
        let result = match request.txn_type.as_str() {
            GET_NYM => {
                let get_nym_result: GetNymResult =
                    serde_json::from_str(data.as_str().unwrap()).unwrap();
                let did_document =
                    DidDocument::new(&did.namespace, &get_nym_result.dest, &get_nym_result.verkey);
                did_document.to_string()
            }
            _ => data.to_string(),
        };

        Ok(result)
    }

    fn build_request(
        &self,
        did: &DidValue,
        path: Option<&str>,
    ) -> Result<PreparedRequest, DidIndyError> {
        let builder = self.pool.get_request_builder();
        let request = match path {
            Some(path) => match LedgerObject::from_str(path)? {
                LedgerObject::ClaimDef(claim_def) => builder
                    .build_get_cred_def_request(
                        Option::None,
                        &(CredentialDefinitionId::new(
                            &did.to_unqualified(),
                            &SchemaId(claim_def.schema_id),
                            "CL",
                            &claim_def.name,
                        )),
                    )
                    .unwrap(),
                LedgerObject::Schema(schema) => builder
                    .build_get_schema_request(
                        Option::None,
                        &SchemaId(format!("{}{}", schema.name, schema.version)),
                    )
                    .unwrap(),
                LedgerObject::RevRegDef(_) => unimplemented!("Arm not implemented yet"),
                LedgerObject::RevRegEntry(_) => unimplemented!("Arm not implemented yet"),
            },
            None => builder
                .build_get_nym_request(Option::None, &did)
                .unwrap(),
        };
        Ok(request)
    }
}

fn handle_request<T: Pool>(pool: &T, request: &PreparedRequest) -> Result<String, DidIndyError> {
    let (result, _timing) = block_on(request_transaction(pool, &request)).unwrap();
    match result {
        RequestResult::Reply(data) => Ok(data),
        RequestResult::Failed(error) => {
            println!("Error requesting data from ledger, {}", error.to_string());
            Err(DidIndyError)
        }
    }
}

async fn request_transaction<T: Pool>(
    pool: &T,
    request: &PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}
