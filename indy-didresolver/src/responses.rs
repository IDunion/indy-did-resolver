use indy_vdr::utils::did::DidValue;
use serde::{Deserialize};

#[derive(Deserialize, Eq, PartialEq, Debug)]
pub struct GetNymResult {
    pub identifier: DidValue,
    pub dest: DidValue,
    pub role: Option<String>,
    pub verkey: String
}