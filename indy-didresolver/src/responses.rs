use indy_vdr::utils::did::DidValue;
use serde::Deserialize;

pub enum ResponseTypes {
    GetNymResult(GetNymResult),
    GetSchemaResult(GetSchemaResult),
    GetClaimDefResult(GetClaimDefResult),
}

#[derive(Deserialize, Eq, PartialEq, Debug)]
pub struct GetNymResult {
    pub identifier: DidValue,
    pub dest: DidValue,
    pub role: Option<String>,
    pub verkey: String,
}

#[derive(Deserialize, Eq, PartialEq, Debug)]
pub struct GetSchemaResult {
    pub attr_names: Vec<String>,
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetClaimDefResult {
    pub ref_schema_attributes: Vec<String>,
    pub ref_schema_from: String,
    pub ref_schema_id: String,
    pub ref_schema_name: String,
    pub ref_schema_txn_seq_no: u32,
    pub ref_schema_txn_time: String,
    pub ref_schema_version: String,
}
