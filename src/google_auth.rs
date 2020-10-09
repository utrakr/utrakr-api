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
          "kty": "RSA",
          "alg": "RS256",
          "n": "teG3wvigoU_KPbPAiEVERFmlGeHWPsnqbEk1pAhz69B0kGHJXU8l8tPHpTw0Gy_M9BJ5WAe9FvXL41xSFbqMGiJ7DIZ32ejlncrf2vGkMl26C5p8OOvuS6ThFjREUzWbV0sYtJL0nNjzmQNCQeb90tDQDZW229ZeUNlM2yN0QRisKlGFSK7uL8X0dRUbXnfgS6eI4mvSAK6tqq3n8IcPA0PxBr-R81rtdG70C2zxlPQ4Wp_MJzjb81d-RPdcYd64loOMhhHFbbfq2bTS9TSn_Y16lYA7gyRGSPhwcsdqOH2qqon7QOiF8gtrvztwd9TpxecPd7mleGGWVFlN6pTQYQ",
          "use": "sig",
          "e": "AQAB",
          "kid": "5effa76ef33ecb5e346bd512d7d89b30e47d8e98"
        },
        {
          "alg": "RS256",
          "kty": "RSA",
          "use": "sig",
          "kid": "7da7863e8637d669bc2a12622cede2a8813d11b1",
          "n": "pnY4f3x0zKSW62RDzpA10AcNfowL07LMVjlRUVmNzqNsv79QuhWBlRefBT0UxbhGzZBwF8XoLBDe-54QXlNsQ8VtIrr8oPnuUR_3ZKGpGiT95HRf_hLoWu7CsyJMF4MOJsa7k4kE6X-4v7KG0hTNe0IMafXI62uU1DoBHyigHUBgZdMv6Do7VDSP-ijGFcp2fPS07aKFaltopM1r-M7FW_pHP4tTtS6_eKLohG1NOSZwPgHKGQai5kc5gwZneNBdJsgRjMHF-NIA9H_vFUoMEQL1JgcbZmSAuhfdhJBKOQGv4pkkbz7Uc9bbIpwdJzgLC1S9hXfbnt-39dZPGN0yHQ",
          "e": "AQAB"
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
