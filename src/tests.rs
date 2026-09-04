use crate::*;
use rust_decimal::Decimal;
use std::str::FromStr;

fn decimal(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}

fn catalog_json() -> &'static str {
    r#"{
        "zeta": {
            "id": "zeta",
            "name": "Zeta",
            "website": "https://zeta.example",
            "apiBaseUrl": "https://api.zeta.example",
            "env": ["ZETA_API_KEY"],
            "models": {
                "unknown-model": {
                    "id": "unknown-model",
                    "name": "Unknown Model",
                    "status": "sunset",
                    "modalities": {"input": ["text", "brainwave"], "output": ["text"]},
                    "reasoning_options": [{"type": "future", "value": true}],
                    "interleaved": {"field": "future_content"},
                    "limit": {"context": 2048}
                }
            }
        },
        "openai": {
            "id": "openai",
            "name": "OpenAI",
            "website": "https://openai.com",
            "api": "https://api.openai.com/v1",
            "doc": "https://developers.openai.com",
            "npm": "openai",
            "env": ["OPENAI_API_KEY"],
            "models": {
                "shared-model": {
                    "id": "shared-model",
                    "name": "OpenAI Shared",
                    "family": "shared",
                    "status": "beta",
                    "attachment": false,
                    "reasoning": true,
                    "tool_call": true,
                    "structured_output": true,
                    "temperature": false,
                    "open_weights": false,
                    "cost": {"input": 2.5, "output": 10},
                    "limit": {"context": 200000, "input": 150000, "output": 50000},
                    "modalities": {"input": ["text", "image", "pdf"], "output": ["text"]}
                },
                "tiered-model": {
                    "id": "tiered-model",
                    "name": "Tiered",
                    "knowledge": "2026-01",
                    "release_date": "2026-02-01",
                    "last_updated": "2026-03-01",
                    "experimental": {"channel": "preview"},
                    "provider": {"region": "global"},
                    "cost": {
                        "input": 1,
                        "output": 2,
                        "reasoning": 0.125,
                        "cache_read": 0.1,
                        "cache_write": 0.2,
                        "input_audio": 3,
                        "output_audio": 4,
                        "tiers": [
                            {"tier": {"type": "context", "size": 100}, "input": 3, "output": 4},
                            {"tier": {"type": "context", "size": 200}, "input": 5, "output": 6}
                        ]
                    },
                    "limit": {"context": 300, "input": 250, "output": 80},
                    "modalities": {"input": ["text", "audio"], "output": ["text", "audio"]}
                },
                "exact-decimal": {
                    "id": "exact-decimal",
                    "cost": {"input": 0.1234567890123456789012345678, "output": 0.00000001},
                    "limit": {"context": 8192}
                },
                "missing-output-price": {
                    "id": "missing-output-price",
                    "cost": {"input": 1},
                    "limit": {"context": 4096}
                }
            }
        },
        "anthropic": {
            "id": "anthropic",
            "name": "Anthropic",
            "models": {
                "shared-model": {
                    "id": "shared-model",
                    "name": "Anthropic Shared",
                    "family": "shared",
                    "attachment": true,
                    "reasoning": false,
                    "tool_call": true,
                    "structured_output": false,
                    "temperature": true,
                    "open_weights": false,
                    "cost": {"input": 3, "output": 15},
                    "limit": {"context": 250000, "input": 200000, "output": 64000},
                    "modalities": {"input": ["text", "image"], "output": ["text"]}
                },
                "cheap-model": {
                    "id": "cheap-model",
                    "name": "Cheap",
                    "features": {"attachment": false, "reasoning": false, "tool_call": false, "structured_output": false, "temperature": true},
                    "cost": {"input": 0.25, "output": 1},
                    "limit": {"context": 100000, "input": 90000, "output": 20000},
                    "modalities": {"input": ["text"], "output": ["text"]}
                },
                "unpriced-model": {
                    "id": "unpriced-model",
                    "limit": {"context": 500000, "input": 400000, "output": 100000}
                }
            }
        }
    }"#
}

fn store() -> RouterStore {
    RouterStore::from_models_dev_json(catalog_json()).unwrap()
}

fn model_keys(models: &[FlatModel]) -> Vec<ModelKey> {
    models.iter().map(FlatModel::key).collect()
}

#[test]
fn store_parses_provider_metadata_and_current_flattened_schema() {
    let store = store();
    let provider = store.find_provider("openai").unwrap();
    assert_eq!(provider.name, "OpenAI");
    assert_eq!(provider.api.as_deref(), Some("https://api.openai.com/v1"));
    assert_eq!(provider.env, ["OPENAI_API_KEY"]);

    let model = store.find_model_in("openai", "tiered-model").unwrap();
    assert_eq!(model.knowledge_cutoff.as_deref(), Some("2026-01"));
    assert_eq!(model.release_date.as_deref(), Some("2026-02-01"));
    assert_eq!(model.last_updated.as_deref(), Some("2026-03-01"));
    assert_eq!(model.experimental.as_ref().unwrap()["channel"], "preview");
    assert_eq!(model.provider_options.as_ref().unwrap()["region"], "global");
    assert_eq!(
        model.pricing.as_ref().unwrap().rates.reasoning,
        Some(decimal("0.125"))
    );
    assert_eq!(
        model.pricing.as_ref().unwrap().rates.input_audio,
        Some(decimal("3"))
    );
}

#[test]
fn unknown_enum_values_are_forward_compatible() {
    let model = store()
        .find_model_in("zeta", "unknown-model")
        .unwrap()
        .clone();
    assert!(matches!(model.status, Some(ModelStatus::Unknown)));
    assert!(matches!(
        model.modalities.as_ref().unwrap().input.as_ref().unwrap()[1],
        ModelModality::Unknown
    ));
    assert!(matches!(
        model.reasoning_options.as_ref().unwrap()[0],
        ReasoningOption::Unknown
    ));
    assert!(matches!(
        model.interleaved,
        Some(InterleavedReasoning::Field {
            field: InterleavedReasoningField::Unknown
        })
    ));
}

#[test]
fn provider_qualified_lookup_reports_bare_id_ambiguity() {
    let store = store();
    let offerings = store.find_models_by_id("shared-model");
    assert_eq!(offerings.len(), 2);
    assert_eq!(offerings[0].provider, "anthropic");
    assert_eq!(offerings[1].provider, "openai");
    assert_eq!(
        store
            .find_model_in("openai", "shared-model")
            .unwrap()
            .name
            .as_deref(),
        Some("OpenAI Shared")
    );
    assert!(store.find_model("shared-model").is_none());
    match store.resolve_model("shared-model") {
        Err(RouterError::AmbiguousModel {
            model_id,
            providers,
        }) => {
            assert_eq!(model_id, "shared-model");
            assert_eq!(providers, ["anthropic", "openai"]);
        }
        result => panic!("unexpected result: {result:?}"),
    }
}

#[test]
fn store_and_default_route_order_are_deterministic() {
    let store = store();
    let expected = vec![
        ModelKey::new("anthropic", "cheap-model"),
        ModelKey::new("anthropic", "shared-model"),
        ModelKey::new("anthropic", "unpriced-model"),
        ModelKey::new("openai", "exact-decimal"),
        ModelKey::new("openai", "missing-output-price"),
        ModelKey::new("openai", "shared-model"),
        ModelKey::new("openai", "tiered-model"),
        ModelKey::new("zeta", "unknown-model"),
    ];
    assert_eq!(model_keys(store.flat_models()), expected);
    let routed = route(&store, &RouteQuery::default());
    assert_eq!(model_keys(&routed.models), expected);
}

#[test]
fn route_pagination_handles_boundaries_and_overflow() {
    let store = store();
    let page = route(
        &store,
        &RouteQuery {
            offset: Some(2),
            limit: Some(3),
            ..RouteQuery::default()
        },
    );
    assert_eq!(page.total, 8);
    assert_eq!(page.models.len(), 3);
    assert!(page.has_more);

    let zero = route(
        &store,
        &RouteQuery {
            limit: Some(0),
            ..RouteQuery::default()
        },
    );
    assert!(zero.models.is_empty());
    assert!(!zero.has_more);

    let overflow = route(
        &store,
        &RouteQuery {
            offset: Some(usize::MAX),
            limit: Some(usize::MAX),
            ..RouteQuery::default()
        },
    );
    assert!(overflow.models.is_empty());
    assert!(!overflow.has_more);
}

#[test]
fn route_filters_false_features_exactly() {
    let result = route(
        &store(),
        &RouteQuery {
            features: Some(ModelFeatures {
                attachment: Some(false),
                reasoning: Some(false),
                tool_call: Some(false),
                structured_output: Some(false),
                temperature: Some(true),
            }),
            ..RouteQuery::default()
        },
    );
    assert_eq!(
        model_keys(&result.models),
        [ModelKey::new("anthropic", "cheap-model")]
    );
}

#[test]
fn route_filters_provider_identity_family_status_and_modalities() {
    let result = route(
        &store(),
        &RouteQuery {
            provider: Some("OPENAI".to_owned()),
            model_id: Some("shared-model".to_owned()),
            family: Some("SHARED".to_owned()),
            status: Some(ModelStatus::Beta),
            input_modalities: Some(vec![ModelModality::Text, ModelModality::Pdf]),
            output_modalities: Some(vec![ModelModality::Text]),
            open_weights: Some(false),
            ..RouteQuery::default()
        },
    );
    assert_eq!(
        model_keys(&result.models),
        [ModelKey::new("openai", "shared-model")]
    );
}

#[test]
fn missing_numeric_values_sort_last_in_both_directions() {
    let store = store();
    for order in [SortOrder::Asc, SortOrder::Desc] {
        let result = route(
            &store,
            &RouteQuery {
                sort: Some(SortField::InputPrice),
                order: Some(order),
                ..RouteQuery::default()
            },
        );
        let unpriced = result
            .models
            .iter()
            .position(|model| model.id == "unpriced-model")
            .unwrap();
        let unknown = result
            .models
            .iter()
            .position(|model| model.id == "unknown-model")
            .unwrap();
        assert!(unpriced >= result.models.len() - 2);
        assert!(unknown >= result.models.len() - 2);
    }
}

#[test]
fn decimal_price_lexemes_remain_exact() {
    let store = store();
    let price = store
        .find_model_in("openai", "exact-decimal")
        .unwrap()
        .pricing
        .as_ref()
        .unwrap()
        .rates
        .input;
    assert_eq!(price, Some(decimal("0.1234567890123456789012345678")));
    assert_eq!(
        estimate_request_cost(price, Some(decimal("0.00000001")), 1_000_000, 1_000_000),
        Some(decimal("0.1234567990123456789012345678"))
    );
}

#[test]
fn cost_uses_highest_tier_whose_boundary_has_started() {
    let model = store()
        .find_model_in("openai", "tiered-model")
        .unwrap()
        .clone();
    let cases = [
        (99, None, decimal("3")),
        (100, Some(100), decimal("7")),
        (199, Some(100), decimal("7")),
        (200, Some(200), decimal("11")),
        (250, Some(200), decimal("11")),
    ];
    for (context_tokens, applied_tier, expected_total) in cases {
        let result = calculate_cost_for_model(
            &model,
            &CostRequest {
                input_tokens: 1_000_000,
                output_tokens: 1_000_000,
                context_tokens: Some(context_tokens),
                ..CostRequest::default()
            },
        )
        .unwrap();
        assert_eq!(result.applied_tier, applied_tier);
        assert_eq!(result.total, expected_total);
    }
}

#[test]
fn cost_rejects_missing_price_for_nonzero_usage() {
    let model = store()
        .find_model_in("openai", "missing-output-price")
        .unwrap()
        .clone();
    let error = calculate_cost_for_model(
        &model,
        &CostRequest {
            input_tokens: 0,
            output_tokens: 1,
            ..CostRequest::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        RouterError::MissingPriceComponent {
            component: CostComponent::Output,
            ..
        }
    ));

    let zero = calculate_cost_for_model(&model, &CostRequest::default()).unwrap();
    assert_eq!(zero.total, Decimal::ZERO);
}

#[test]
fn context_fit_checks_overflow_and_each_limit_separately() {
    let store = store();
    assert!(matches!(
        check_context_fit_for_provider(&store, "openai", "tiered-model", u64::MAX, Some(1)),
        Err(RouterError::TokenOverflow)
    ));

    let input_failure =
        check_context_fit_for_provider(&store, "openai", "tiered-model", 251, Some(1)).unwrap();
    assert!(input_failure.context_fits);
    assert!(!input_failure.input_fits);
    assert!(input_failure.output_fits);
    assert!(!input_failure.fits);
    assert!(input_failure.should_compact);

    let output_failure =
        check_context_fit_for_provider(&store, "openai", "tiered-model", 1, Some(81)).unwrap();
    assert!(output_failure.context_fits);
    assert!(output_failure.input_fits);
    assert!(!output_failure.output_fits);
    assert!(!output_failure.should_compact);

    let context_failure =
        check_context_fit_for_provider(&store, "openai", "tiered-model", 250, Some(80)).unwrap();
    assert!(!context_failure.context_fits);
    assert!(context_failure.input_fits);
    assert!(context_failure.output_fits);
    assert_eq!(context_failure.overhead, -30);
}

#[test]
fn bare_context_and_fallback_calls_reject_ambiguity() {
    let store = store();
    assert!(matches!(
        check_context_fit(&store, "shared-model", 1, None),
        Err(RouterError::AmbiguousModel { .. })
    ));
    assert!(matches!(
        fallback_chain(&store, "shared-model", &FallbackOptions::default()),
        Err(RouterError::AmbiguousModel { .. })
    ));
}

#[test]
fn fallback_and_best_context_selection_are_deterministic() {
    let store = store();
    let chain = fallback_chain_for_provider(
        &store,
        "anthropic",
        "cheap-model",
        &FallbackOptions {
            limit: Some(2),
            ..FallbackOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        chain.original.key(),
        ModelKey::new("anthropic", "cheap-model")
    );
    assert!(chain.models.len() <= 2);
    let best = find_best_context_model(&store, 80_000, None).unwrap();
    assert_eq!(best.key(), ModelKey::new("anthropic", "cheap-model"));
}

#[test]
fn comparison_uses_provider_qualified_keys_without_overwrite() {
    let store = store();
    let keys = [
        ModelKey::new("openai", "shared-model"),
        ModelKey::new("anthropic", "shared-model"),
    ];
    let comparison = compare_models(&store, &keys).unwrap();
    assert_eq!(comparison.models.len(), 2);
    let prices = comparison
        .fields
        .iter()
        .find(|field| field.field == "input_price")
        .unwrap();
    assert_eq!(prices.values.len(), 2);
    assert_eq!(prices.winners, [ModelKey::new("openai", "shared-model")]);
    assert!(matches!(
        compare(&store, &["shared-model"]),
        Err(RouterError::AmbiguousModel { .. })
    ));
}

#[test]
fn overlay_requires_exact_provider_and_model_identity() {
    let base = r#"{
        "alibaba": {"id":"alibaba","name":"Alibaba","models":{"qwen":{"id":"qwen","name":"Global","cost":{"input":1}}}},
        "alibaba-cn": {"id":"alibaba-cn","name":"Alibaba China","models":{"qwen":{"id":"qwen","name":"China","cost":{"input":2}}}}
    }"#;
    let overlay = r#"{
        "alibaba-cn": {"id":"alibaba-cn","name":"Alibaba China","models":{"qwen":{"id":"qwen","name":"China Updated","cost":{"input":9}}}}
    }"#;
    let mut store = RouterStore::from_models_dev_json(base).unwrap();
    let report = store
        .apply_overlay_from_json(overlay, OverlayMode::PreferOverlay)
        .unwrap();
    assert_eq!(report.models_touched, 1);
    assert_eq!(report.models_unmatched, 1);
    assert_eq!(
        store
            .find_model_in("alibaba", "qwen")
            .unwrap()
            .name
            .as_deref(),
        Some("Global")
    );
    assert_eq!(
        store
            .find_model_in("alibaba", "qwen")
            .unwrap()
            .pricing
            .as_ref()
            .unwrap()
            .rates
            .input,
        Some(decimal("1"))
    );
    assert_eq!(
        store
            .find_model_in("alibaba-cn", "qwen")
            .unwrap()
            .name
            .as_deref(),
        Some("China Updated")
    );
    assert_eq!(
        store
            .find_model_in("alibaba-cn", "qwen")
            .unwrap()
            .pricing
            .as_ref()
            .unwrap()
            .rates
            .input,
        Some(decimal("9"))
    );
}

#[test]
fn overlay_fill_only_is_field_level_and_idempotent() {
    let base =
        r#"{"p":{"id":"p","name":"P","models":{"m":{"id":"m","name":"Base","cost":{"input":1}}}}}"#;
    let overlay = r#"{"p":{"id":"p","name":"P","models":{"m":{"id":"m","name":"Overlay","description":"Added","cost":{"input":9,"output":2}}}}}"#;
    let mut store = RouterStore::from_models_dev_json(base).unwrap();
    let first = store
        .apply_overlay_from_json(overlay, OverlayMode::FillOnly)
        .unwrap();
    assert_eq!(first.models_touched, 1);
    let model = store.find_model_in("p", "m").unwrap();
    assert_eq!(model.name.as_deref(), Some("Base"));
    assert_eq!(model.description.as_deref(), Some("Added"));
    assert_eq!(
        model.pricing.as_ref().unwrap().rates.input,
        Some(decimal("1"))
    );
    assert_eq!(
        model.pricing.as_ref().unwrap().rates.output,
        Some(decimal("2"))
    );
    assert_eq!(
        store
            .apply_overlay_from_json(overlay, OverlayMode::FillOnly)
            .unwrap()
            .fields_written,
        0
    );
}

#[test]
fn empty_overlay_objects_do_not_mutate_absent_fields() {
    let base = r#"{"p":{"id":"p","name":"P","models":{"m":{"id":"m"}}}}"#;
    let overlay = r#"{"p":{"id":"p","name":"P","models":{"m":{"id":"m","features":{},"cost":{},"limit":{},"modalities":{}}}}}"#;
    let mut store = RouterStore::from_models_dev_json(base).unwrap();
    let report = store
        .apply_overlay_from_json(overlay, OverlayMode::FillOnly)
        .unwrap();
    let model = store.find_model_in("p", "m").unwrap();
    assert_eq!(report.fields_written, 0);
    assert_eq!(report.models_touched, 0);
    assert!(model.features.is_none());
    assert!(model.pricing.is_none());
    assert!(model.limit.is_none());
    assert!(model.modalities.is_none());
}

#[test]
fn catalog_rejects_map_and_embedded_identity_mismatches() {
    let provider_mismatch = r#"{"outer":{"id":"inner","name":"P","models":{"m":{"id":"m"}}}}"#;
    assert!(matches!(
        RouterStore::from_models_dev_json(provider_mismatch),
        Err(RouterError::InvalidCatalogIdentity(_))
    ));

    let model_mismatch = r#"{"p":{"id":"p","name":"P","models":{"outer":{"id":"inner"}}}}"#;
    assert!(matches!(
        RouterStore::from_models_dev_json(model_mismatch),
        Err(RouterError::InvalidCatalogIdentity(_))
    ));

    let negative_price =
        r#"{"p":{"id":"p","name":"P","models":{"m":{"id":"m","cost":{"input":-1,"output":1}}}}}"#;
    assert!(matches!(
        RouterStore::from_models_dev_json(negative_price),
        Err(RouterError::InvalidCatalogValue(_))
    ));
}

#[test]
fn extreme_decimal_inputs_do_not_panic_routing_helpers() {
    let json = r#"{
        "p": {
            "id": "p",
            "name": "P",
            "models": {
                "max": {
                    "id": "max",
                    "limit": {"context": 1},
                    "cost": {"input": 79228162514264337593543950335, "output": 0}
                },
                "candidate": {
                    "id": "candidate",
                    "limit": {"context": 2},
                    "cost": {"input": 1, "output": 0}
                }
            }
        }
    }"#;
    let store = RouterStore::from_models_dev_json(json).unwrap();
    let fit = check_context_fit_for_provider(&store, "p", "max", 2, None).unwrap();
    assert!(fit.better_alternatives.is_empty());
    let chain = fallback_chain_for_provider(
        &store,
        "p",
        "max",
        &FallbackOptions {
            max_price_multiplier: Some(Decimal::MAX),
            ..FallbackOptions::default()
        },
    )
    .unwrap();
    assert!(chain.models.is_empty());

    let model = store.find_model_in("p", "max").unwrap();
    assert!(matches!(
        calculate_cost_for_model(
            model,
            &CostRequest {
                input_tokens: 2,
                ..CostRequest::default()
            }
        ),
        Err(RouterError::TokenOverflow)
    ));
}

#[test]
fn unknown_price_tier_kinds_are_not_applied_as_context_tiers() {
    let json = r#"{
        "p": {
            "id": "p",
            "name": "P",
            "models": {
                "m": {
                    "id": "m",
                    "cost": {
                        "input": 1,
                        "output": 2,
                        "tiers": [
                            {"tier": {"type": "future_kind", "size": 0}, "input": 9, "output": 10}
                        ]
                    }
                }
            }
        }
    }"#;
    let store = RouterStore::from_models_dev_json(json).unwrap();
    let model = store.find_model_in("p", "m").unwrap();
    let cost = calculate_cost_for_model(
        model,
        &CostRequest {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            context_tokens: Some(1_000_000),
            ..CostRequest::default()
        },
    )
    .unwrap();
    assert_eq!(cost.input, decimal("1"));
    assert_eq!(cost.output, decimal("2"));
    assert_eq!(cost.applied_tier, None);
}

#[cfg(feature = "bundled")]
#[test]
fn bundled_catalog_matches_exported_provenance_and_parses_current_scale() {
    let store = RouterStore::bundled().unwrap();
    let metadata = store.catalog_metadata();
    assert_eq!(metadata.source, CatalogSource::ModelsDevBundled);
    assert_eq!(metadata.source_url.as_deref(), Some(MODELS_DEV_API_URL));
    assert_eq!(
        metadata.retrieved_at.as_deref(),
        Some(BUNDLED_MODELS_DEV_RETRIEVED_AT)
    );
    assert_eq!(metadata.sha256.as_deref(), Some(BUNDLED_MODELS_DEV_SHA256));
    assert_eq!(metadata.etag.as_deref(), Some(BUNDLED_MODELS_DEV_ETAG));
    assert_eq!(
        metadata.source_revision.as_deref(),
        Some(BUNDLED_MODELS_DEV_SOURCE_REVISION)
    );
    assert_eq!(metadata.byte_count, Some(BUNDLED_MODELS_DEV_BYTE_COUNT));
    assert_eq!(metadata.provider_count, store.providers().len());
    assert_eq!(metadata.model_count, store.flat_models().len());
    assert_eq!(metadata.provider_count, BUNDLED_MODELS_DEV_PROVIDER_COUNT);
    assert_eq!(metadata.model_count, BUNDLED_MODELS_DEV_MODEL_COUNT);
    assert!(metadata.provider_count >= 100);
    assert!(metadata.model_count >= 4_000);
    assert!(store.find_provider("openai").is_some());
    assert!(store.find_provider("alibaba").is_some());
    assert!(store.find_provider("anthropic").is_some());
}
