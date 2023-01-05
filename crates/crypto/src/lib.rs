
use anyhow::Result;
use sha2::Digest;
use ring::digest;
use crate::bridge::key::CryptoHash;
use rand::thread_rng;
use rand::Rng;

#[derive(Deserialize, Serialize, Debug)]
struct EncodeDigestArg {
    algorithm: String,
    data: ByteBuf,
    //   data: Vec<u8>,
}
#[derive(Deserialize, Serialize, Debug)]
struct GetRandomValuesArg {
    value: u32,
}

pub fn crypto_subtle_digest(
    algorithm: CryptoHash,
    // algorithm: String,
    data: Vec<u8>,
  ) -> Vec<u8> {

    let output = digest::digest(algorithm.into(), &data)
        .as_ref()
        .to_vec(); 
  
    output
}

pub fn crypto_random_uuid() -> String {
    let uuid = uuid::Uuid::new_v4();
  
    return uuid.to_string();
}

pub fn crypto_get_random_values(arr: &mut [u8] ) -> Result<Vec<u8>>  {
    
    // let mut arr = [0u8];
    let mut rng = thread_rng();
    rng.fill(&mut *arr);

    // println!("^^^^{:?}", arr);
    Ok(arr.to_vec())
  }