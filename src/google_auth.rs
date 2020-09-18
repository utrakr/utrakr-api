use fehler::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// https://www.googleapis.com/oauth2/v3/certs
// TODO: these keys are rotated
lazy_static! {
    static ref KEYS: Value = json!({
      "keys": [
        {
          "e": "AQAB",
          "kty": "RSA",
          "n": "o9K9HWGBMwnvSJWGYXEtKgu9p7Kqx6qH5t5WozS1kD79pMy4yiGyUXBwHbIs7KxnxwjBz65_QuLtiFQGhpKke8aqdj-v2TQUPQUHz0Q62nWM0f636JLocMJhRRBWZsEQhad5xK-Vy8gvuNKpby37dc5gxyjHKJx5Dt-XQHyP5qlXQw84lrGNGJg3d8j3frAi6cMJSEKr70xeaoAWNl4NOKT94fepKOuxKdVzZI1RiqgyZPA190BkK5UcnjiMzg_odgYSZWqp9pTNBKj0CHsFql3ynUiSo1jcXA6KCDW2AMNtOOLuzg0fq1UY-SmVnU94DKktD9DEtMFewP0F8C6PRQ",
          "use": "sig",
          "kid": "3f332b3e9b928bfe51bcf4f8da543cc4bd9d4724",
          "alg": "RS256"
        },
        {
          "n": "qOpAAmY20iOCNu8c913YoMv01U817A_SrTsN6Ocgejp2CoBs9OeibGCzH6TibjxGbHPlC6LOk4dHDrqGkbhXaWPaISVlaqplzRAxpeEAkJhfuzFqqDtyN3wJPfj0skDn3TeTqmEydwLbexlwLMh8Pzsj-YwDQsEvono2y9Yq5jb3qNe2SsJUMpAm2lcM49EHdbvcwLx6taVBcs_UVbqurGvYp4AbfzNLlDoGe3lZBZ55OjDRcfxsOJsw-dCx4mTr-UGJe50LFUfG_bkZ18TTbGxHiJmqYUrnmM9LVyihM3rd_aQa5I_zBtwbMo6_ntDhiF4klYr_xgXhvGlxog0dEw",
          "use": "sig",
          "e": "AQAB",
          "kty": "RSA",
          "kid": "4b83f18023a855587f942e75102251120887f725",
          "alg": "RS256"
        }
      ]
    });
}

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleClaims {
    email: String,
    email_verified: bool,
    name: String,
    exp: usize,
}

#[throws(anyhow::Error)]
pub fn get_claim_from_google(token: &str) -> GoogleClaims {
    let jwk = &KEYS["keys"][0];
    let token = decode::<GoogleClaims>(
        &token,
        &DecodingKey::from_rsa_components(jwk["n"].as_str().unwrap(), jwk["e"].as_str().unwrap()),
        &Validation::new(Algorithm::RS256),
    )?;
    token.claims
}
