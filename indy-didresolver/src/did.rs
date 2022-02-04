use regex::Regex;
use crate::error::{DidIndyError, DidIndyResult};

//TODO: add other Did requests for SCHEMA ...
#[derive(Debug, PartialEq)]
pub enum Type {
    Nym,
}

#[derive(Debug, PartialEq)]
pub struct Did {
    pub namespace: String,
    pub id: String,
    pub request_type: Type,
}

pub fn did_parse(did: &str) -> DidIndyResult<Did> {
    //TODO: change regex to exclude O,0,I,l
    let did_regex = Regex::new("did:indy:([a-zA-Z]+|[a-zA-Z]+:[a-zA-Z]+):([a-zA-Z0-9]{21,22})$").expect("Error in the DID Regex Syntax");

    let captures = did_regex.captures(did.trim());
    return match captures {
        Some(cap) => {
            let did = Did {
                namespace : cap.get(1).unwrap().as_str().to_string(),
                id: cap.get(2).unwrap().as_str().to_string(),
                request_type: Type::Nym,
            };
            Ok(did)
        }
        None => {
            println!("Requested DID does not match the W3C DID-core standard.");
            Err(DidIndyError)
        }
    }
}

#[cfg(test)]
mod tests {
    mod did_syntax_tests {
        use crate::did::{Did, did_parse, Type};

        #[test]
        fn did_syntax_tests() {
            let _err = did_parse("did:indy:onlynamespace").unwrap_err();

            assert_eq!(
                did_parse("did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE").unwrap(),
                Did { namespace: String::from("idunion"), id: String::from("BDrEcHc8Tb4Lb2VyQZWEDE"), request_type: Type::Nym }
            );

            assert_eq!(
                did_parse("did:indy:sovrin:staging:6cgbu8ZPoWTnR5Rv5JcSMB").unwrap(),
                Did { namespace: String::from("sovrin:staging"), id: String::from("6cgbu8ZPoWTnR5Rv5JcSMB"), request_type: Type::Nym }
            );

            let _err = did_parse("did:indy:illegal:third:namespace:1111111111111111111111").unwrap_err();

            let _err = did_parse("did:indy:test:12345678901234567890").unwrap_err();
            let _err = did_parse("did:indy:test:12345678901234567890123").unwrap_err();
            //TODO: add Test to fail with namespace-identifer with O,0,I,l
        }
    }
}
