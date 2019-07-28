use openssl::rsa::Rsa;
use std::{env, error::Error, fs::File, io::Write, path::Path};

/// If a private key does not already exist,
/// generate one for the refresh and access tokens.
fn ensure_rsa_key() -> Result<(), Box<dyn Error>> {
    let path = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("src/.db_key");

    if !path.exists() {
        let key = Rsa::generate(4096)?.private_key_to_pem()?;
        File::create(&path)?.write_all(&key)?;
    }

    Ok(())
}

fn cfg_debug_release() -> Result<(), Box<dyn Error>> {
    println!("cargo:rustc-cfg={}", env::var("PROFILE")?);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    ensure_rsa_key()?;
    cfg_debug_release()?;

    Ok(())
}
