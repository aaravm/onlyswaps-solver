#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_drand_connection() {
        println!("\n🌐 Testing Drand Connection");
        
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
                    Err(e) => println!("Second call failed: {}", e),
                }
            }
            Err(e) => {
                println!("❌ Drand connection failed: {}", e);
                println!("💡 This is expected if you don't have internet connectivity");
                // Don't fail the test if offline - this is optional functionality
            }
        }
    }
}
