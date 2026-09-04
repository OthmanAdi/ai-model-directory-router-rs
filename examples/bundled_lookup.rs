//! Inspect one provider-qualified offering in the bundled models.dev snapshot.

#[cfg(feature = "bundled")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ai_model_directory_router::RouterStore;

    let store = RouterStore::bundled()?;
    let metadata = store.catalog_metadata();
    println!(
        "loaded {} offerings from {} providers",
        metadata.model_count, metadata.provider_count
    );
    println!("snapshot SHA-256: {:?}", metadata.sha256);

    match store.find_model_in("alibaba", "qwen3.8-flash") {
        Some(model) => {
            println!("offering: {}", model.key());
            println!("family: {:?}", model.family);
            println!("status: {:?}", model.status);
            println!(
                "context: {:?}",
                model.limit.as_ref().and_then(|limit| limit.context)
            );
            println!(
                "input USD per million tokens: {:?}",
                model
                    .pricing
                    .as_ref()
                    .and_then(|pricing| pricing.rates.input)
            );
        }
        None => println!("alibaba/qwen3.8-flash is not present in this snapshot"),
    }

    Ok(())
}

#[cfg(not(feature = "bundled"))]
fn main() {
    eprintln!("rerun with the bundled feature enabled");
}
