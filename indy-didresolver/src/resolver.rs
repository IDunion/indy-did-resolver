use futures_executor::block_on;
use indy_vdr::utils::Qualifiable;
use serde_json::Value;

use super::did::{DidUrl, LedgerObject};
use super::did_document::{DidDocument, LEGACY_INDY_SERVICE};
use super::error::{DidIndyError, DidIndyResult};
use super::responses::{Endpoint, GetNymResultV1};

use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::constants::GET_NYM;
use indy_vdr::ledger::identifiers::{CredentialDefinitionId, RevocationRegistryId, SchemaId};
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::pool::{Pool, PreparedRequest, RequestResult, TimingResult};
use indy_vdr::utils::did::DidValue;

pub struct Resolver<T: Pool> {
    pool: T,
}

impl<T: Pool> Resolver<T> {
    pub fn new(pool: T) -> Resolver<T> {
        Resolver { pool }
    }

    pub fn resolve(&self, did: &str) -> DidIndyResult<String> {
        let did_url = DidUrl::from_str(did)?;
        let request = self.build_request(&did_url)?;

        let ledger_data = handle_request(&self.pool, &request)?;
        let data = parse_ledger_data(&ledger_data)?;

        let result = match request.txn_type.as_str() {
            GET_NYM => {
                let get_nym_result: GetNymResultV1 = serde_json::from_str(data.as_str().unwrap())?;

                println!("{:#?}", get_nym_result);

                let endpoint: Option<Endpoint> = if get_nym_result.diddoc_content.is_none() {
                    // Legacy: Try to find an attached ATTRIBUTE transacation with raw endpoint
                    self.fetch_legacy_endpoint(&did_url.id).ok()
                } else {
                    None
                };

                let did_document = DidDocument::new(
                    &did_url.namespace,
                    &get_nym_result.dest,
                    &get_nym_result.verkey,
                    endpoint,
                    None,
                );
                did_document.to_string()?
            }
            // other ledger objects
            _ => data.to_string(),
        };

        Ok(result)
    }

    fn fetch_legacy_endpoint(&self, did: &DidValue) -> DidIndyResult<Endpoint> {
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

    fn build_request(&self, did: &DidUrl) -> DidIndyResult<PreparedRequest> {
        let builder = self.pool.get_request_builder();

        let request = if did.path.is_some() {
            match LedgerObject::from_str(did.path.as_ref().unwrap().as_str())? {
                LedgerObject::Schema(schema) => builder.build_get_schema_request(
                    None,
                    &SchemaId::new(&did.id, &schema.name, &schema.version),
                ),
                LedgerObject::ClaimDef(claim_def) => builder.build_get_cred_def_request(
                    None,
                    &CredentialDefinitionId::from_str(
                        format!(
                            "{}:3:CL:{}:{}",
                            &did.id, claim_def.schema_seq_no, claim_def.name
                        )
                        .as_str(),
                    )
                    .unwrap(),
                ),
                LedgerObject::RevRegDef(rev_reg_def) => builder.build_get_revoc_reg_def_request(
                    None,
                    &RevocationRegistryId::from_str(
                        format!(
                            "{}:4:{}:3:CL:{}:{}:CL_ACCUM:{}",
                            &did.id,
                            &did.id,
                            rev_reg_def.schema_seq_no,
                            rev_reg_def.claim_def_name,
                            rev_reg_def.tag
                        )
                        .as_str(),
                    )
                    .unwrap(),
                ),
                // LedgerObject::RevRegEntry(_) => Err(VdrError::new(
                //     VdrErrorKind::Incompatible,
                //     Some(String::from("Not implemented")),
                //     None,
                // )),
            }
        } else {
            builder.build_get_nym_request(Option::None, &did.id)
        };
        request.map_err(|e| DidIndyError::from(e))
    }
}

fn handle_request<T: Pool>(pool: &T, request: &PreparedRequest) -> DidIndyResult<String> {
    let (result, _timing) = block_on(request_transaction(pool, &request))?;
    match result {
        RequestResult::Reply(data) => Ok(data),
        RequestResult::Failed(error) => {
            println!("Error requesting data from ledger, {}", error.to_string());
            Err(DidIndyError::VdrError(error))
        }
    }
}

async fn request_transaction<T: Pool>(
    pool: &T,
    request: &PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}

fn parse_ledger_data(ledger_data: &str) -> DidIndyResult<Value> {
    let v: Value = serde_json::from_str(&ledger_data)?;
    let data: &Value = &v["result"]["data"];
    if *data == Value::Null {
        Err(DidIndyError::EmptyData)
    } else {
        Ok(data.to_owned())
    }
}
