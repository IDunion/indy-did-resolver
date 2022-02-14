use std::str::FromStr;

use futures_executor::block_on;
use serde_json::Value;

use super::did::{did_parse, LedgerObject};
use super::did_document::{DidDocument, LEGACY_INDY_SERVICE};
use super::error::DidIndyError;
use super::responses::{Endpoint, GetNymResultV1};

use indy_vdr::common::error::{VdrError, VdrErrorKind, VdrResult};
use indy_vdr::ledger::constants::GET_NYM;
use indy_vdr::ledger::identifiers::SchemaId;
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
        let did = did_parse(did.as_str())?;

        // The path variable identifies the requested ledger object
        // If there is no path, then we return a DID document
        let path = if did_url.path_abempty != "" {
            Some(did_url.path_abempty.as_str())
        } else {
            None
        };

        let did_value = DidValue::new(&did.id, Option::None);

        let request = self.build_request(&did_value, path)?;

        let ledger_data = handle_request(&self.pool, &request)?;
        let data = parse_ledger_data(&ledger_data)?;

        let result = match request.txn_type.as_str() {
            GET_NYM => {
                let get_nym_result: GetNymResultV1 = serde_json::from_str(data.as_str().unwrap())?;

                println!("{:#?}", get_nym_result);

                let endpoint: Option<Endpoint> = if get_nym_result.diddoc_content.is_none() {
                    // Legacy: Try to find an attached ATTRIBUTE transacation with raw endpoint
                    self.fetch_legacy_endpoint(&did_value).ok()
                } else {
                    None
                };

                let did_document = DidDocument::new(
                    &did.namespace,
                    &get_nym_result.dest,
                    &get_nym_result.verkey,
                    endpoint,
                    None,
                );
                did_document.to_string()?
            }
            _ => data.to_string(),
        };

        Ok(result)
    }

    fn fetch_legacy_endpoint(&self, did: &DidValue) -> Result<Endpoint, DidIndyError> {
        let builder = self.pool.get_request_builder();
        let request = builder.build_get_attrib_request(
            None,
            did,
            Some(String::from(LEGACY_INDY_SERVICE)),
            None,
            None,
        )?;
        let ledger_data = handle_request(&self.pool, &request)?;
        let endpoint_data = parse_ledger_data(&ledger_data)?;
        let endpoint_data: Endpoint = serde_json::from_str(endpoint_data.as_str().unwrap())?;
        Ok(endpoint_data)
    }

    fn build_request(
        &self,
        did: &DidValue,
        path: Option<&str>,
    ) -> Result<PreparedRequest, DidIndyError> {
        let builder = self.pool.get_request_builder();
        let request = match path {
            Some(path) => match LedgerObject::from_str(path)? {
                LedgerObject::Schema(schema) => builder.build_get_schema_request(
                    Option::None,
                    &SchemaId::new(&did, &schema.name, &schema.version),
                ),
                LedgerObject::ClaimDef(_) => Err(VdrError::new(
                    VdrErrorKind::Incompatible,
                    Some(String::from("Not implemented")),
                    None,
                )),
                LedgerObject::RevRegDef(_) => Err(VdrError::new(
                    VdrErrorKind::Incompatible,
                    Some(String::from("Not implemented")),
                    None,
                )),
                LedgerObject::RevRegEntry(_) => Err(VdrError::new(
                    VdrErrorKind::Incompatible,
                    Some(String::from("Not implemented")),
                    None,
                )),
            },
            None => builder.build_get_nym_request(Option::None, &did),
        };
        request.map_err(|e| DidIndyError::from(e))
    }
}

fn handle_request<T: Pool>(pool: &T, request: &PreparedRequest) -> Result<String, DidIndyError> {
    let (result, _timing) = block_on(request_transaction(pool, &request))?;
    match result {
        RequestResult::Reply(data) => Ok(data),
        RequestResult::Failed(error) => {
            println!("Error requesting data from ledger, {}", error.to_string());
            Err(DidIndyError::Unknown)
        }
    }
}

async fn request_transaction<T: Pool>(
    pool: &T,
    request: &PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}

fn parse_ledger_data(ledger_data: &str) -> Result<Value, DidIndyError> {
    let v: Value = serde_json::from_str(&ledger_data)?;
    let data: &Value = &v["result"]["data"];
    if *data == Value::Null {
        return Err(DidIndyError::Unknown);
    }
    Ok(data.to_owned())
}
