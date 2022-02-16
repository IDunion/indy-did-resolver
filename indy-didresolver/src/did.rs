use super::error::{DidIndyError, DidIndyResult};
use indy_vdr::utils::did::DidValue;
use regex::Regex;
use url::Url;

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum QueryParameter {
    VersionId,
    VersionTime,
    From,
    To,
}

impl QueryParameter {
    pub fn from_str(input: &str) -> DidIndyResult<QueryParameter> {
        match input {
            "versionId" => Ok(QueryParameter::VersionId),
            "versionTime" => Ok(QueryParameter::VersionTime),
            "From" => Ok(QueryParameter::From),
            "To" => Ok(QueryParameter::To),
            _ => Err(DidIndyError::QueryParameterNotSupported),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ObjectFamily {
    Anoncreds,
}

impl ObjectFamily {
    fn from_str(input: &str) -> DidIndyResult<ObjectFamily> {
        match input {
            "anoncreds" => Ok(ObjectFamily::Anoncreds),
            _ => Err(DidIndyError::ObjectFamilyNotSupported),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Anoncreds {
    AnoncredsV0,
}

impl Anoncreds {
    fn from_str(input: &str) -> DidIndyResult<Anoncreds> {
        match input {
            "v0" => Ok(Anoncreds::AnoncredsV0),
            _ => Err(DidIndyError::VersionNotSupported),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ObjectCodes {
    Attrib = 1,
    Schema = 2,
    ClaimDef = 3,
    RevRegDef = 4,
    RevRegEntry = 5,
}

#[derive(Debug, PartialEq)]
pub struct Schema {
    pub name: String,
    pub version: String,
    type_: u8,
}

impl Schema {
    fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            type_: ObjectCodes::Schema as u8,
        }
    }

    fn from_str(input: &str) -> DidIndyResult<Schema> {
        let re = Regex::new(r"^([a-zA-Z0-9_:]*)/?((?:\d\.){1,2}\d)").unwrap();

        let captures = re.captures(input);

        match captures {
            Some(cap) => Ok(Schema::new(
                cap.get(1)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string(),
                cap.get(2)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string(),
            )),
            _ => Err(DidIndyError::InvalidDidUrl),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ClaimDef {
    pub schema_seq_no: u32,
    pub name: String,
    type_: u8,
}

impl ClaimDef {
    fn new(schema_seq_no: u32, name: String) -> Self {
        Self {
            schema_seq_no,
            name,
            type_: ObjectCodes::ClaimDef as u8,
        }
    }

    fn from_str(input: &str) -> DidIndyResult<ClaimDef> {
        let re = Regex::new(r"^([0-9]*)/([a-zA-Z0-9_-]?*)").unwrap();

        let captures = re.captures(input);

        match captures {
            Some(cap) => Ok(ClaimDef::new(
                cap.get(1)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string()
                    .parse::<u32>()
                    .unwrap(),
                cap.get(2)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string(),
            )),
            _ => Err(DidIndyError::InvalidDidUrl),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RevRegDef {
    pub schema_seq_no: u32,
    pub claim_def_name: String,
    pub tag: String,
    type_: u8,
}

impl RevRegDef {
    fn new(schema_seq_no: u32, claim_def_name: String, tag: String) -> Self {
        Self {
            schema_seq_no,
            claim_def_name,
            tag,
            type_: ObjectCodes::RevRegDef as u8,
        }
    }

    fn from_str(input: &str) -> DidIndyResult<RevRegDef> {
        let re = Regex::new(r"^([0-9]*)/([a-zA-Z0-9_-]?*)/([a-zA-Z0-9._-]?*)$").unwrap();

        let captures = re.captures(input);

        match captures {
            Some(cap) => Ok(RevRegDef::new(
                cap.get(1)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string()
                    .parse::<u32>()
                    .unwrap(),
                cap.get(2)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string(),
                cap.get(3)
                    .ok_or(DidIndyError::InvalidDidUrl)?
                    .as_str()
                    .to_string(),
            )),
            _ => Err(DidIndyError::InvalidDidUrl),
        }
    }
}

// #[derive(Debug, PartialEq)]
// pub struct RevRegEntry {
//     rev_reg_name: String,
//     claim_def_name: String,
//     type_: u8,
// }

// #[allow(dead_code)]
// impl RevRegEntry {
//     fn new(rev_reg_name: String, claim_def_name: String) -> Self {
//         Self {
//             rev_reg_name,
//             claim_def_name,
//             type_: ObjectCodes::RevRegEntry as u8,
//         }
//     }
// }

#[derive(Debug, PartialEq)]
pub enum LedgerObject {
    Schema(Schema),
    ClaimDef(ClaimDef),
    RevRegDef(RevRegDef),
}

impl LedgerObject {
    pub fn from_str(input: &str) -> DidIndyResult<LedgerObject> {
        let re = Regex::new(
            r"^/([a-z]*)/([a-zA-Z0-9]*)/(SCHEMA|CLAIM_DEF|REV_REG_DEF|REV_REG_ENTRY)/(.*)?",
        )
        .unwrap();

        let captures = re.captures(input);

        if let Some(cap) = captures {
            let object_family_str = cap.get(1).ok_or(DidIndyError::InvalidDidUrl)?.as_str();
            let version = cap.get(2).ok_or(DidIndyError::InvalidDidUrl)?.as_str();

            let object_family = ObjectFamily::from_str(object_family_str)?;

            match object_family {
                ObjectFamily::Anoncreds => {
                    let object_family_versioned = Anoncreds::from_str(version)?;
                    match object_family_versioned {
                        Anoncreds::AnoncredsV0 => {
                            let ledger_object_type_str =
                                cap.get(3).ok_or(DidIndyError::InvalidDidUrl)?.as_str();
                            let ledger_object_type_specific_str =
                                cap.get(4).ok_or(DidIndyError::InvalidDidUrl)?.as_str();
                            match ledger_object_type_str {
                                "SCHEMA" => Ok(LedgerObject::Schema(Schema::from_str(
                                    ledger_object_type_specific_str,
                                )?)),
                                "CLAIM_DEF" => Ok(LedgerObject::ClaimDef(ClaimDef::from_str(
                                    ledger_object_type_specific_str,
                                )?)),
                                "REV_REG_DEF" => Ok(LedgerObject::RevRegDef(RevRegDef::from_str(
                                    ledger_object_type_specific_str,
                                )?)),
                                "REV_REG_ENTRY" => unimplemented!("Not yet completed"),

                                _ => Err(DidIndyError::InvalidDidUrl),
                            }
                        }
                    }
                }
            }
        } else {
            Err(DidIndyError::InvalidDidUrl)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DidUrl {
    pub namespace: String,
    pub id: DidValue,
    pub path: Option<String>,
    pub query: HashMap<QueryParameter, String>,
    pub url: String,
}

impl DidUrl {
    pub fn from_str(input: &str) -> DidIndyResult<DidUrl> {
        let did_regex =
            Regex::new("did:indy:([a-zA-Z]+|[a-zA-Z]+:[a-zA-Z]+):([a-zA-Z0-9]{21,22})(/.*)?$")
                .unwrap();

        let input_without_query = input.split("?").collect::<Vec<&str>>()[0];

        let url = Url::parse(input).map_err(|_| DidIndyError::InvalidDidUrl)?;
        let mut query_pairs: HashMap<QueryParameter, String> = HashMap::new();
        let _query_pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();

        for (k, v) in _query_pairs.iter() {
            let qp = QueryParameter::from_str(k)?;

            query_pairs.insert(qp, v.to_string());
        }

        let captures = did_regex.captures(input_without_query.trim());
        match captures {
            Some(cap) => {
                let did = DidUrl {
                    namespace: cap.get(1).unwrap().as_str().to_string(),
                    id: DidValue::new(cap.get(2).unwrap().as_str(), Option::None),
                    path: cap.get(3).and_then(|p| Some(p.as_str().to_string())),
                    query: query_pairs,
                    url: input.to_string(),
                };
                Ok(did)
            }
            None => Err(DidIndyError::InvalidDidUrl),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unknown_ledger_object_fails() {
        assert!(matches!(
            LedgerObject::from_str("/anoncreds/v0/PANTS/npdb/4.3.4"),
            Err(DidIndyError::InvalidDidUrl)
        ))
    }

    #[test]
    fn parse_unknown_object_family_fails() {
        assert!(matches!(
            LedgerObject::from_str("/othercreds/v0/SCHEMA/npdb/4.3.4"),
            Err(DidIndyError::ObjectFamilyNotSupported)
        ))
    }

    #[test]
    fn parse_unknown_anoncreds_version_fails() {
        assert!(matches!(
            LedgerObject::from_str("/anoncreds/v5/SCHEMA/npdb/4.3.4"),
            Err(DidIndyError::VersionNotSupported)
        ))
    }

    #[test]
    fn parse_to_schema() {
        assert_eq!(
            LedgerObject::from_str("/anoncreds/v0/SCHEMA/npdb/4.3.4").unwrap(),
            LedgerObject::Schema(Schema::new(String::from("npdb"), String::from("4.3.4")))
        )
    }

    #[test]
    fn parse_to_schema_two_digit_version() {
        assert_eq!(
            LedgerObject::from_str("/anoncreds/v0/SCHEMA/npdb/4.3").unwrap(),
            LedgerObject::Schema(Schema::new(String::from("npdb"), String::from("4.3")))
        )
    }

    #[test]
    fn parse_to_schema_without_version_fails() {
        assert!(matches!(
            LedgerObject::from_str("/anoncreds/v0/SCHEMA/npdb"),
            Err(DidIndyError::InvalidDidUrl)
        ))
    }

    #[test]
    fn parse_to_schema_with_one_digit_version_fails() {
        assert!(matches!(
            LedgerObject::from_str("/anoncreds/v0/SCHEMA/npdb/4"),
            Err(DidIndyError::InvalidDidUrl)
        ))
    }

    #[test]
    fn parse_to_claim_def() {
        assert_eq!(
            LedgerObject::from_str("/anoncreds/v0/CLAIM_DEF/23452/npdb").unwrap(),
            LedgerObject::ClaimDef(ClaimDef::new(23452, String::from("npdb")))
        )
    }

    #[test]
    fn parse_to_claim_def_without_seq_no_fails() {
        assert!(matches!(
            LedgerObject::from_str("/anoncreds/v0/CLAIM_DEF/npdb"),
            Err(DidIndyError::InvalidDidUrl)
        ))
    }

    #[test]
    fn parse_to_claim_def_with_seq_no_as_string_fails() {
        assert!(matches!(
            LedgerObject::from_str("/anoncreds/v0/CLAIM_DEF/myseqno/npdb"),
            Err(DidIndyError::InvalidDidUrl)
        ))
    }

    #[test]
    fn parse_to_rev_reg_def() {
        assert_eq!(
            LedgerObject::from_str("/anoncreds/v0/REV_REG_DEF/104/revocable/a4e25e54-e028-462b-a4d6-b1d1712d51a1").unwrap(),
            LedgerObject::RevRegDef(RevRegDef::new(104,String::from("revocable"), String::from("a4e25e54-e028-462b-a4d6-b1d1712d51a1")))
        )
    }
    mod did_syntax_tests {

        use super::*;

        #[test]
        fn did_syntax_tests() {
            let _err = DidUrl::from_str("did:indy:onlynamespace").unwrap_err();

            assert_eq!(
                DidUrl::from_str("did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE").unwrap(),
                DidUrl {
                    namespace: String::from("idunion"),
                    id: DidValue::new("BDrEcHc8Tb4Lb2VyQZWEDE", None),
                    path: None,
                    query: HashMap::new(),
                    url: String::from("did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE"),
                }
            );

            assert_eq!(
                DidUrl::from_str("did:indy:sovrin:staging:6cgbu8ZPoWTnR5Rv5JcSMB").unwrap(),
                DidUrl {
                    namespace: String::from("sovrin:staging"),
                    id: DidValue::new("BDrEcHc8Tb4Lb2VyQZWEDE", None),
                    path: None,
                    query: HashMap::new(),
                    url: String::from("did:indy:sovrin:staging:6cgbu8ZPoWTnR5Rv5JcSMB"),
                }
            );

            let _err = DidUrl::from_str("did:indy:illegal:third:namespace:1111111111111111111111")
                .unwrap_err();

            let _err = DidUrl::from_str("did:indy:test:12345678901234567890").unwrap_err();
            let _err = DidUrl::from_str("did:indy:test:12345678901234567890123").unwrap_err();
            //TODO: add Test to fail with namespace-identifer with O,0,I,l
        }

        #[test]
        fn parse_did_url_with_query_parameter() {
            let mut q = HashMap::new();
            q.insert(QueryParameter::VersionId, String::from("1"));

            assert_eq!(
                DidUrl::from_str("did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE?versionId=1&hello=world")
                    .unwrap(),
                DidUrl {
                    namespace: String::from("idunion"),
                    id: DidValue::new("BDrEcHc8Tb4Lb2VyQZWEDE", None),
                    path: None,
                    query: q,
                    url: String::from(
                        "did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE?versionId=1&hello=world"
                    ),
                }
            );
        }

        #[test]
        fn parse_did_url_fails_with_arbitrary_query_parameter() {
            assert!(matches!(
                DidUrl::from_str("did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE?hello=world"),
                Err(DidIndyError::QueryParameterNotSupported)
            ));
        }

        #[test]
        fn parse_did_url_with_path() {
            assert_eq!(
                DidUrl::from_str("did:indy:idunion:Dk1fRRTtNazyMuK2cr64wp/anoncreds/v0/REV_REG_DEF/104/revocable/a4e25e54-e028-462b-a4d6-b1d1712d51a1")
                    .unwrap(),
                DidUrl {
                    namespace: String::from("idunion"),
                    id: DidValue::new("Dk1fRRTtNazyMuK2cr64wp", None),
                    path: Some(String::from("/anoncreds/v0/REV_REG_DEF/104/revocable/a4e25e54-e028-462b-a4d6-b1d1712d51a1")),
                    query: HashMap::new(),
                    url: String::from(
                        "did:indy:idunion:Dk1fRRTtNazyMuK2cr64wp/anoncreds/v0/REV_REG_DEF/104/revocable/a4e25e54-e028-462b-a4d6-b1d1712d51a1"
                    ),
                }
            );
        }
    }
}
