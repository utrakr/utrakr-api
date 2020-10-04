use std::str::FromStr;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// https://www.googleapis.com/oauth2/v3/certs
// TODO: these keys are rotated
lazy_static! {
    static ref KEYS: Value = json!({
      "keys": [
        {
          "n": "o9K9HWGBMwnvSJWGYXEtKgu9p7Kqx6qH5t5WozS1kD79pMy4yiGyUXBwHbIs7KxnxwjBz65_QuLtiFQGhpKke8aqdj-v2TQUPQUHz0Q62nWM0f636JLocMJhRRBWZsEQhad5xK-Vy8gvuNKpby37dc5gxyjHKJx5Dt-XQHyP5qlXQw84lrGNGJg3d8j3frAi6cMJSEKr70xeaoAWNl4NOKT94fepKOuxKdVzZI1RiqgyZPA190BkK5UcnjiMzg_odgYSZWqp9pTNBKj0CHsFql3ynUiSo1jcXA6KCDW2AMNtOOLuzg0fq1UY-SmVnU94DKktD9DEtMFewP0F8C6PRQ",
          "kty": "RSA",
          "alg": "RS256",
          "e": "AQAB",
          "use": "sig",
          "kid": "3f332b3e9b928bfe51bcf4f8da543cc4bd9d4724"
        },
        {
          "e": "AQAB",
          "alg": "RS256",
          "kid": "4b83f18023a855587f942e75102251120887f725",
          "use": "sig",
          "kty": "RSA",
          "n": "qOpAAmY20iOCNu8c913YoMv01U817A_SrTsN6Ocgejp2CoBs9OeibGCzH6TibjxGbHPlC6LOk4dHDrqGkbhXaWPaISVlaqplzRAxpeEAkJhfuzFqqDtyN3wJPfj0skDn3TeTqmEydwLbexlwLMh8Pzsj-YwDQsEvono2y9Yq5jb3qNe2SsJUMpAm2lcM49EHdbvcwLx6taVBcs_UVbqurGvYp4AbfzNLlDoGe3lZBZ55OjDRcfxsOJsw-dCx4mTr-UGJe50LFUfG_bkZ18TTbGxHiJmqYUrnmM9LVyihM3rd_aQa5I_zBtwbMo6_ntDhiF4klYr_xgXhvGlxog0dEw"
        }
      ]
    });
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleClaims {
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    exp: usize,
}

pub fn get_claim_from_google(token: &str) -> Option<GoogleClaims> {
    for jwk in KEYS["keys"].as_array().unwrap() {
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
    None
}
