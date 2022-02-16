use chrono::{DateTime, Utc};

use futures_executor::block_on;
use indy_vdr::utils::Qualifiable;
use serde_json::Value;

use super::did::{DidUrl, LedgerObject, QueryParameter};
use super::did_document::{DidDocument, LEGACY_INDY_SERVICE};
use super::error::{DidIndyError, DidIndyResult};
use super::responses::{Endpoint, GetNymResultV1};

use indy_vdr::common::error::VdrResult;
use indy_vdr::ledger::constants;
use indy_vdr::ledger::identifiers::{CredentialDefinitionId, RevocationRegistryId, SchemaId};
use indy_vdr::ledger::RequestBuilder;
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

        let builder = self.pool.get_request_builder();
        let request = build_request(&did_url, &builder)?;

        let ledger_data = handle_request(&self.pool, &request)?;
        let data = parse_ledger_data(&ledger_data)?;

        let result = match request.txn_type.as_str() {
            constants::GET_NYM => {
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
}

fn build_request(did: &DidUrl, builder: &RequestBuilder) -> DidIndyResult<PreparedRequest> {
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
            LedgerObject::RevRegEntry(rev_reg_def) => {
                if did.query.contains_key(&QueryParameter::From) {
                    let from = did.query.get(&QueryParameter::From)
                        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
                        .and_then(|d| Some(d.timestamp()));
                
                    let to = parse_or_now(did.query.get(&QueryParameter::From))?;

                    builder.build_get_revoc_reg_delta_request(
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
                        from,
                        to,
                    )
                } else {
                    let timestamp = parse_or_now(did.query.get(&QueryParameter::VersionTime))?;

                    builder.build_get_revoc_reg_request(
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
                        timestamp,
                    )
                }
            }
        }
    } else {
        // TODO: Use with new get nym request

        let _seq_no: Option<i64> = did
            .query
            .get(&QueryParameter::VersionId)
            .and_then(|v| v.parse().ok());
        let _timestamp: Option<i64> = did
            .query
            .get(&QueryParameter::VersionTime)
            .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
            .and_then(|d| Some(d.timestamp()));

        builder.build_get_nym_request(Option::None, &did.id)
    };
    request.map_err(|e| DidIndyError::from(e))
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

fn parse_or_now(datetime: Option<&String>) -> DidIndyResult<i64> {
    match datetime {
        Some(datetime) => {
            let dt = DateTime::parse_from_rfc3339(datetime)?;
            Ok(dt.timestamp())
        }
        None => Ok(Utc::now().timestamp()),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rstest::*;
    use serde_json::json;

    use indy_vdr::pool::ProtocolVersion;

    const REQ_ID: u64 = 1585221529670242337;
    const TYPE: &str = constants::GET_NYM;

    fn _identifier() -> DidValue {
        DidValue(String::from("V4SGRU86Z58d6TV7PBUe6f"))
    }

    fn _dest() -> DidValue {
        DidValue(String::from("VsKV7grR1BUE29mG2Fm2kX"))
    }

    fn _protocol_version() -> usize {
        ProtocolVersion::Node1_4.to_id()
    }

    #[fixture]
    fn request() -> serde_json::Value {
        json!({
            "identifier": _identifier(),
            "operation":{
                "dest": _dest(),
                "type": TYPE
            },
            "protocolVersion": _protocol_version(),
            "reqId": REQ_ID
        })
    }

    #[fixture]
    fn request_json(request: serde_json::Value) -> String {
        request.to_string()
    }

    #[fixture]
    fn prepared_request(request_json: String) -> PreparedRequest {
        PreparedRequest::from_request_json(&request_json).unwrap()
    }

    #[fixture]
    fn request_builder() -> RequestBuilder {
        RequestBuilder::new(ProtocolVersion::Node1_4)
    }

    #[rstest]
    fn build_get_revoc_reg_request_from_version_time(request_builder: RequestBuilder) {
        let datetime_as_str = "2020-12-20T19:17:47Z";
        let did_url_as_str = format!("did:indy:idunion:Dk1fRRTtNazyMuK2cr64wp/anoncreds/v0/REV_REG_ENTRY/104/revocable/a4e25e54?versionTime={}",datetime_as_str);
        let did_url = DidUrl::from_str(&did_url_as_str).unwrap();
        let request = build_request(&did_url, &request_builder).unwrap();
        let timestamp = (*(request.req_json).get("operation").unwrap())
            .get("timestamp")
            .unwrap()
            .as_u64()
            .unwrap() as i64;
        assert_eq!(constants::GET_REVOC_REG, request.txn_type);

        assert_eq!(
            DateTime::parse_from_rfc3339(datetime_as_str)
                .unwrap()
                .timestamp(),
            timestamp
        );
    }

    #[rstest]
    fn build_get_revoc_reg_delta_request_with_from_to(request_builder: RequestBuilder) {
        let from_as_str = "2019-12-20T19:17:47Z";
        let to_as_str = "2020-12-20T19:17:47Z";
        let did_url_as_str = format!("did:indy:idunion:Dk1fRRTtNazyMuK2cr64wp/anoncreds/v0/REV_REG_ENTRY/104/revocable/a4e25e54?from={}&to={}",from_as_str, to_as_str);
        let did_url = DidUrl::from_str(&did_url_as_str).unwrap();
        let request = build_request(&did_url, &request_builder).unwrap();
        assert_eq!(
            request.txn_type,
            constants::GET_REVOC_REG_DELTA
        );
    }

    #[rstest]
    fn build_get_revoc_reg_delta_request_with_from_only(request_builder: RequestBuilder) {
        let from_as_str = "2019-12-20T19:17:47Z";
        let did_url_as_str = format!("did:indy:idunion:Dk1fRRTtNazyMuK2cr64wp/anoncreds/v0/REV_REG_ENTRY/104/revocable/a4e25e54?from={}",from_as_str);
        let did_url = DidUrl::from_str(&did_url_as_str).unwrap();
        let request = build_request(&did_url, &request_builder).unwrap();
        assert_eq!(
            request.txn_type,
            constants::GET_REVOC_REG_DELTA
        );
    }
}
