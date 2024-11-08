use serde::ser::SerializeStruct;
use sha2::{Digest, Sha256, Sha512};

use std::convert::TryInto;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

const EXPIRE_MIINUTES: u64 = 5;

#[derive(Debug, Clone)]
pub enum DecodeTokenError {
    Expired(Duration),
    InvalidContent(String),
    InvalidFormat,
    Unknown,
}

impl Serialize for DecodeTokenError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("DecodeError", 2)?;

        match self {
            DecodeTokenError::Expired(duration) => {
                state.serialize_field("type", "Expired")?;
                state.serialize_field(
                    "description",
                    format!("Expired {} seconds ago.", &duration.as_secs()).as_str(),
                )?;
            }
            DecodeTokenError::InvalidContent(token) => {
                state.serialize_field("type", "InvalidContent")?;
                state.serialize_field(
                    "description",
                    format!("Invalid register token {}.", &token).as_str(),
                )?;
            }
            DecodeTokenError::InvalidFormat => {
                state.serialize_field("type", "InvalidFormat")?;
                state.serialize_field(
                    "description",
                    "Register token should be a 64-digit hexadecimal number.",
                )?;
            }
            DecodeTokenError::Unknown => {
                state.serialize_field("type", "Unknown")?;
                state.serialize_field("description", "An unknown error occurred.")?;
            }
        }

        state.end()
    }
}

pub fn decode_token(hex_input: &str, token: &str) -> Result<(u8, u8, String), DecodeTokenError> {
    let bytes = hex::decode(hex_input).map_err(|_| DecodeTokenError::InvalidFormat)?;
    if bytes.len() != 64 {
        return Err(DecodeTokenError::InvalidFormat);
    }

    let (encoded_token, received_hash) = bytes.split_at(32);

    let decoded_token: Vec<u8> = encoded_token
        .iter()
        .zip(received_hash.iter())
        .map(|(&a, &b)| a ^ b)
        .collect();

    let salt_bytes = token.as_bytes();

    let mut hasher = Sha256::new();
    hasher.update(&decoded_token);
    hasher.update(salt_bytes);
    let calculated_hash = hasher.finalize();

    if calculated_hash.as_slice() != received_hash {
        return Err(DecodeTokenError::InvalidContent(hex_input.to_owned()));
    }

    let version = decoded_token[0];
    let openid = hex::encode(&decoded_token[1..22]);
    let time = u32::from_be_bytes(
        decoded_token[22..26]
            .try_into()
            .map_err(|_| DecodeTokenError::InvalidContent(hex_input.to_owned()))?,
    ) as u64
        * 60;
    let mark = decoded_token[26];

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DecodeTokenError::Unknown)?
        .as_secs();

    if now - time > EXPIRE_MIINUTES * 60 {
        return Err(DecodeTokenError::Expired(Duration::from_secs(
            now - time - EXPIRE_MIINUTES * 60,
        )));
    }

    Ok((version, mark, openid))
}

use rand::rngs::OsRng;
use rand::RngCore;

pub fn gen_salted_password(password: &str, token: &str) -> (String, String) {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    let mut hasher = Sha256::new();
    hasher.update(token);
    hasher.update(password);
    hasher.update(salt);
    let calculated_hash = hasher.finalize();

    (hex::encode(salt), hex::encode(calculated_hash.as_slice()))
}

use actix_web::cookie::Key;

pub fn gen_cookie_key(cookie_token: &str) -> Key {
    let mut hasher = Sha512::new();
    hasher.update(cookie_token);
    Key::from(hasher.finalize().as_slice())
}
