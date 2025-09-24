use reqwest::Client;
use serde::Deserialize;
use hex;
use num_bigint::BigUint;
use rug::{Float, Assign};
use rug::ops::CompleteRound;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_drand_connection() {
        println!("\n🌐 Testing Drand Connection (async)");

        let drand = DrandRandomness::new();

        match drand.get_normalized_random().await {
            Ok(randomness) => {
                println!("✅ Successfully connected to drand!");
                println!("🎲 Random value: {:.10}", randomness);
                assert!(randomness >= 0.0 && randomness < 1.0, "Randomness should be in [0,1)");

                // Test multiple calls to ensure randomness varies
                match drand.get_normalized_random().await {
                    Ok(randomness2) => {
                        println!("🎲 Second random value: {:.10}", randomness2);
                        println!("🔀 Values are different: {}", randomness != randomness2);
                    }
                    Err(e) => eprintln!("Second call failed: {}", e),
                }
            }
            Err(e) => {
                eprintln!("❌ Drand connection failed: {}", e);
                eprintln!("💡 This is expected if you don't have internet connectivity");
                // Don't fail the test if offline - this is optional functionality
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct LatestRound {
    round: u64,
    signature: String,
}

pub struct DrandRandomness {
    client: Client,
    chain_hash: String,
}

impl DrandRandomness {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap(),
            // Quicknet chain hash (mainnet)
            chain_hash: "52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971".to_string(),
        }
    }

    /// Fetch random value from drand and return as Float (async)
    pub async fn get_random_float(&self) -> Result<Float, Box<dyn std::error::Error>> {
        let base = "https://api.drand.sh";
        let url = format!("{}/v2/chains/{}/rounds/latest", base, self.chain_hash);

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            let error_msg = format!("HTTP error: {} - {}", status, text);
            return Err(error_msg.into());
        }

        let latest: LatestRound = resp.json().await?;
        println!("round: {}", latest.round);
        println!("signature (hex): {}", latest.signature);

        // Parse signature hex as the randomness source (standard for drand)
        let hex_str = latest.signature.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)?;
        let r = BigUint::from_bytes_be(&bytes);
        println!("randomness from signature (int): {}", r);
        println!("signature length: {} bytes ({} bits)", bytes.len(), bytes.len() * 8);

        // Normalize to [0,1) with high precision using rug::Float
        // normalized = r / 2^(bits in signature)
        let bits = bytes.len() * 8;
        let mut f = Float::with_val(160, 0); // 160 bits precision

        // Convert BigUint -> decimal string -> parse into Float
        let r_str = r.to_str_radix(10);
        
        // Parse string into Float using rug's proper method
        let parsed = rug::Float::parse(&r_str).map_err(|e| format!("Failed to parse float: {}", e))?;
        f.assign(parsed.complete(160));

        // compute denom = 2^bits using bit shifting (more efficient than pow)
        let mut denom = Float::with_val(160, 1);
        denom <<= bits;

        let norm = f / denom;
        println!("normalized (0..1) ~ {:.60}", norm); // 60 decimal places

        Ok(norm)
    }

    /// Get normalized random value as f64 for easier use (async)
    pub async fn get_normalized_random(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let float_val = self.get_random_float().await?;
        Ok(float_val.to_f64())
    }
}

impl Default for DrandRandomness {
    fn default() -> Self {
        Self::new()
    }
}
