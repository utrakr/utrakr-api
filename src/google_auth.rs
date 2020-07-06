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
          "alg": "RS256",
          "use": "sig",
          "kid": "a41a3570b8e3ae1b72caabcaa7b8d2db2065d7c1",
          "e": "AQAB",
          "kty": "RSA",
          "n": "kricWoM-lZq1WJzz8hfcnQ_k_mzT5yHnApncngSgXHOeTw5UhUeqzUFr5Ok8qmdJqNh6_UeKykv1r1AR89_6ASBWlaHAZPzj5VmIry9QA385pywp3vHKQx1__Kj2ySkLhqpjY7Yjys62kSDkFQJy-LbvCmNxMoe1D8275KEMatTsCj6MjvDw8vul4Owkb_83rdONec9Vy32mk1KpmG_quuYzDlTfKD5ktKsraAuv2Fai9Obsl9oevULw6zH4Uks64W4-SMGJkEhXfAc1tnXEPGcfR3Uwo-jcN0SLou56TGF-TjTAOYHu9Np4UI8_-1vVfLsG7csN8rjd15wCABq7Vw"
        },
        {
          "e": "AQAB",
          "alg": "RS256",
          "n": "5749CAb1LariZ0YoufKRHW08VZUDS1KqC35jbBrTgfcWfPr-zVbKdxtLYMtZ9m3jxxcGp-C-P1Cx7Q38tCicj1GiYSTe7Nm0oioJPRxn3R48XgRo78i5yYOYXgpJSPIEWAT6bBSqkH5TYH4GWcoaOu4rRb368ygshTqcjyQ_64Yg2Zp2Ce1InJIzSfwlcayE53ng2GJYv7Wmvu6_YGriu7oVVr41TeOovZvcqcakTCPE1nRMOvE5vyv7rXv5Sxzsk3tmyRBG0LBZqRLvM4WhIvfvBF2CT3brlfNuqx3q7MYKna5n8xRl3JK8DucstXGXkV99nl1W81_OvbxcT268Yw",
          "use": "sig",
          "kty": "RSA",
          "kid": "65b3feaad9db0f38b1b4ab94553ff17de4dd4d49"
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
