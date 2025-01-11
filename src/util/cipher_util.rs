use dotenv::dotenv;
use once_cell::sync::Lazy;
use serde::ser::SerializeStruct;
use serde::Serialize;
use sha2::{Digest, Sha256, Sha512};
use std::convert::TryInto;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
                    "desp",
                    format!("Expired {} seconds ago.", &duration.as_secs()).as_str(),
                )?;
            }
            DecodeTokenError::InvalidContent(token) => {
                state.serialize_field("type", "InvalidContent")?;
                state.serialize_field(
                    "desp",
                    format!("Invalid register token {}.", &token).as_str(),
                )?;
            }
            DecodeTokenError::InvalidFormat => {
                state.serialize_field("type", "InvalidFormat")?;
                state.serialize_field(
                    "desp",
                    "Register token should be a 64-digit hexadecimal number.",
                )?;
            }
            DecodeTokenError::Unknown => {
                state.serialize_field("type", "Unknown")?;
                state.serialize_field("desp", "An unknown error occurred.")?;
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

pub fn get_salt<const N: usize>() -> [u8; N] {
    let mut salt = [0u8; N];
    OsRng.fill_bytes(&mut salt);
    salt
}

pub fn gen_salted_password(password: &str, token: &str) -> (String, String) {
    let salt = get_salt::<32>();

    let mut hasher = Sha256::new();
    hasher.update(token);
    hasher.update(password);
    hasher.update(salt);
    let calculated_hash = hasher.finalize();

    (hex::encode(salt), hex::encode(calculated_hash.as_slice()))
}

pub fn check_salted_password<'a>(
    user: &'a User,
    password_input: &str,
    token: &str,
) -> Option<&'a User> {
    let mut salt = [0u8; 32];
    hex::decode_to_slice(&user.salt, &mut salt).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(token);
    hasher.update(password_input);
    hasher.update(salt);

    let calculated_hash = hasher.finalize();

    let mut expected_hash = [0u8; 32];
    hex::decode_to_slice(&user.password, &mut expected_hash).ok()?;

    if calculated_hash.as_slice() == &expected_hash[..] {
        Some(user)
    } else {
        None
    }
}

use actix_web::cookie::Key;

use crate::models::User;

pub fn gen_cookie_key(cookie_token: &str) -> Key {
    let mut hasher = Sha512::new();
    hasher.update(cookie_token);
    Key::from(hasher.finalize().as_slice())
}

static VERIFY_TOKEN: Lazy<String> = Lazy::new(|| {
    dotenv().ok();
    env::var("VERIFY_TOKEN").expect("Environment variable VERIFY_TOKEN not set")
});

fn current_totp_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
        / 120
}

fn totp(token_id: &str, time: u64) -> String {
    let mut hasher = Sha512::new();
    hasher.update(token_id);
    hasher.update(VERIFY_TOKEN.as_str());
    hasher.update(time.to_le_bytes());

    hex::encode(hasher.finalize().as_slice())
}

pub fn gen_totp(identity: &str) -> String {
    totp(identity, current_totp_time())
}

// For user, identity is the open_id
// For group, identiy is the salt
pub fn verify_totp(identity: &str, veri_code: &str) -> bool {
    let time = current_totp_time();

    // Allowing 2 minutes of error
    veri_code.len() == crate::VERICODE_LENGTH
        && (totp(identity, time).starts_with(veri_code)
            || totp(identity, time - 1).starts_with(veri_code)
            || totp(identity, time + 1).starts_with(veri_code))
}

pub fn check_answer(answer: &str, key: &str, submission: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(answer);

    let mut buffer = [0u8; 32];
    hex::decode_to_slice(submission, &mut buffer)
        .is_ok_and(|()| hasher.finalize().as_slice() == buffer)
}

pub fn prepare_hashed_answer(answer: &str, key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(answer);
    hex::encode(hasher.finalize().as_slice())
}
