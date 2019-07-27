use once_cell::sync::Lazy;
use openssl::rsa::{Padding, Rsa};

static KEY: Lazy<Rsa<openssl::pkey::Private>> =
    Lazy::new(|| Rsa::private_key_from_pem(include_bytes!("./.db_key")).unwrap());

/// Encrypt a string using a global key, returning the bitvec.
pub fn encrypt(payload: &str) -> Vec<u8> {
    let mut buffer = vec![0; KEY.size() as usize];

    KEY.public_encrypt(payload.as_bytes(), &mut buffer, Padding::PKCS1)
        .expect("unable to encrypt value");

    buffer
}

/// Given a bitarray, decrypt it using a global key and return the resulting string.
pub fn decrypt(encrypted: &[u8]) -> String {
    let mut decrypted = vec![0; KEY.size() as usize];

    KEY.private_decrypt(encrypted, &mut decrypted, Padding::PKCS1)
        .expect("unable to decrypt value");

    String::from_utf8(decrypted).unwrap()
}
