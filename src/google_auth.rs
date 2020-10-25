use std::str::FromStr;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleClaims {
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    exp: usize,
}

pub fn get_claim_from_google(token: &str) -> Option<GoogleClaims> {
    let resp = ureq::get("https://www.googleapis.com/oauth2/v3/certs").call();
    if let Some(err) = resp.synthetic_error() {
        error!("unable to make request. {}", err);
        return None
    }

    assert!(resp.ok());
    if let Ok(j) = resp.into_json() {
        for jwk in j["keys"].as_array().unwrap() {
            if let Ok(alg) = Algorithm::from_str(jwk["alg"].as_str().unwrap()) {
                let token = decode::<GoogleClaims>(
                    &token,
                    &DecodingKey::from_rsa_components(
                        jwk["n"].as_str().unwrap(),
                        jwk["e"].as_str().unwrap(),
                    ),
                    &Validation::new(alg),
                );
                match token {
                    Ok(t) => return Some(t.claims),
                    Err(e) => debug!("unable to validate {:?}", e),
                }
            }
        }
    } else {
        error!("unable to deserialize json")
    }
    None
}
