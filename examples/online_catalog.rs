//! Fetch the fixed public models.dev provider catalog and inspect one offering.

#[cfg(feature = "online")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ai_model_directory_router::RouterStore;

    let store = RouterStore::from_models_dev_live()?;
    let metadata = store.catalog_metadata();
    println!(
        "loaded {} live offerings from {} providers",
        metadata.model_count, metadata.provider_count
    );
    println!("source: {:?}", metadata.source_url);
    println!("retrieved at: {:?}", metadata.retrieved_at);
    println!("ETag: {:?}", metadata.etag);
    println!("SHA-256: {:?}", metadata.sha256);

    match store.find_model_in("alibaba", "qwen3.8-flash") {
        Some(model) => println!("found provider-qualified offering {}", model.key()),
        None => println!("alibaba/qwen3.8-flash is not present in the live catalog"),
    }

    Ok(())
}

#[cfg(not(feature = "online"))]
fn main() {
    eprintln!("rerun with --features online");
}
