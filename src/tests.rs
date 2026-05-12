use crate::*;
use rust_decimal::Decimal;
use std::str::FromStr;

macro_rules! dec {
    ($s:literal) => {
        Decimal::from_str($s).unwrap()
    };
}

const SAMPLE_JSON: &str = r#"{
  "test-provider": {
    "id": "test-provider",
    "name": "Test Provider",
    "website": "https://example.com",
    "apiBaseUrl": "https://api.example.com/v1",
    "models": {
      "model-small": {
        "id": "model-small",
        "name": "Small Model",
        "release_date": "1729036800",
        "features": {
          "attachment": false,
          "reasoning": false,
          "tool_call": true,
          "structured_output": false
        },
        "pricing": {
          "input": 0.50,
          "output": 1.50
        },
        "limit": {
          "context": 4096,
          "output": 2048
        },
        "modalities": {
          "input": ["text"],
          "output": ["text"]
        }
      },
      "model-large": {
        "id": "model-large",
        "name": "Large Model",
        "open_weights": true,
        "features": {
          "attachment": true,
          "reasoning": true,
          "tool_call": true,
          "structured_output": true
        },
        "pricing": {
          "input": 3.00,
          "output": 6.00,
          "cache_read": 0.50,
          "cache_write": 1.00
        },
        "limit": {
          "context": 128000,
          "input": 120000,
          "output": 16384
        },
        "modalities": {
          "input": ["text", "image", "file"],
          "output": ["text", "image"]
        }
      },
      "model-audio": {
        "id": "model-audio",
        "name": "Audio Model",
        "features": {
          "attachment": false,
          "reasoning": false,
          "tool_call": false,
          "structured_output": false
        },
        "pricing": {
          "input": 1.00,
          "output": 2.00,
          "input_audio": 5.00,
          "output_audio": 10.00
        },
        "limit": {
          "context": 32000
        },
        "modalities": {
          "input": ["text", "audio"],
          "output": ["text", "audio"]
        }
      }
    }
  },
  "other-provider": {
    "id": "other-provider",
    "name": "Other Provider",
    "apiBaseUrl": "https://other.example.com/v1",
    "models": {
      "other-cheap": {
        "id": "other-cheap",
        "name": "Cheap Other",
        "open_weights": true,
        "features": {
          "attachment": false,
          "reasoning": false,
          "tool_call": true,
          "structured_output": false
        },
        "pricing": {
          "input": 0.10,
          "output": 0.20
        },
        "limit": {
          "context": 8192,
          "output": 4096
        },
        "modalities": {
          "input": ["text"],
          "output": ["text"]
        }
      },
      "other-big": {
        "id": "other-big",
        "name": "Big Other",
        "features": {
          "attachment": true,
          "reasoning": true,
          "tool_call": true,
          "structured_output": true,
          "temperature": true
        },
        "pricing": {
          "input": 10.00,
          "output": 30.00
        },
        "limit": {
          "context": 200000,
          "output": 8192
        },
        "modalities": {
          "input": ["text", "image"],
          "output": ["text"]
        }
      }
    }
  }
}"#;

fn test_store() -> RouterStore {
    RouterStore::from_json(SAMPLE_JSON).unwrap()
}

#[test]
fn store_from_json_parses_all_models() {
    let store = test_store();
    assert_eq!(store.flat_models().len(), 5);
}

#[test]
fn store_find_model_returns_correct_model() {
    let store = test_store();
    let m = store.find_model("model-large").unwrap();
    assert_eq!(m.id, "model-large");
    assert_eq!(m.name.as_deref(), Some("Large Model"));
    assert_eq!(m.provider, "test-provider");
    assert_eq!(m.open_weights, Some(true));
}

#[test]
fn store_find_model_returns_none_for_unknown() {
    let store = test_store();
    assert!(store.find_model("nonexistent").is_none());
}

#[test]
fn store_find_models_by_provider_filters_correctly() {
    let store = test_store();
    let tp = store.find_models_by_provider("test-provider");
    assert_eq!(tp.len(), 3);
    let op = store.find_models_by_provider("other-provider");
    assert_eq!(op.len(), 2);
}

#[test]
fn store_find_models_by_provider_is_case_insensitive() {
    let store = test_store();
    assert_eq!(store.find_models_by_provider("TEST-PROVIDER").len(), 3);
}

#[test]
fn store_provider_entry_deserializes_camel_case() {
    let store = test_store();
    let m = store.find_model("model-small").unwrap();
    assert_eq!(m.provider, "test-provider");
    let provider_models = store.find_models_by_provider("test-provider");
    assert_eq!(provider_models.len(), 3);
}

#[test]
fn cost_basic_calculation() {
    let store = test_store();
    let model = store.find_model("model-small").unwrap();
    let req = CostRequest {
        input_tokens: 1_000_000,
        output_tokens: 1_000_000,
        reasoning_tokens: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
        input_audio_tokens: None,
        output_audio_tokens: None,
    };
    let breakdown = calculate_cost_for_model(model, &req);
    assert_eq!(breakdown.input, dec!("0.50"));
    assert_eq!(breakdown.output, dec!("1.50"));
    assert_eq!(breakdown.total, dec!("2.00"));
}

#[test]
fn cost_with_cache_tokens() {
    let store = test_store();
    let model = store.find_model("model-large").unwrap();
    let req = CostRequest {
        input_tokens: 500_000,
        output_tokens: 250_000,
        reasoning_tokens: None,
        cache_read_tokens: Some(200_000),
        cache_write_tokens: Some(100_000),
        input_audio_tokens: None,
        output_audio_tokens: None,
    };
    let breakdown = calculate_cost_for_model(model, &req);
    assert_eq!(breakdown.input, dec!("1.50"));
    assert_eq!(breakdown.output, dec!("1.50"));
    assert_eq!(breakdown.cache_read, dec!("0.10"));
    assert_eq!(breakdown.cache_write, dec!("0.10"));
}

#[test]
fn cost_with_audio_tokens() {
    let store = test_store();
    let model = store.find_model("model-audio").unwrap();
    let req = CostRequest {
        input_tokens: 100_000,
        output_tokens: 50_000,
        reasoning_tokens: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
        input_audio_tokens: Some(10_000),
        output_audio_tokens: Some(5_000),
    };
    let breakdown = calculate_cost_for_model(model, &req);
    assert_eq!(breakdown.input, dec!("0.10"));
    assert_eq!(breakdown.output, dec!("0.10"));
    assert_eq!(breakdown.input_audio, dec!("0.05"));
    assert_eq!(breakdown.output_audio, dec!("0.05"));
}

#[test]
fn cost_zero_tokens() {
    let store = test_store();
    let model = store.find_model("model-small").unwrap();
    let req = CostRequest {
        input_tokens: 0,
        output_tokens: 0,
        reasoning_tokens: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
        input_audio_tokens: None,
        output_audio_tokens: None,
    };
    let breakdown = calculate_cost_for_model(model, &req);
    assert_eq!(breakdown.total, Decimal::ZERO);
}

#[test]
fn estimate_request_cost_basic() {
    let cost = estimate_request_cost(Some(1.0), Some(2.0), 1_000_000, 500_000);
    assert_eq!(cost, dec!("2"));
}

#[test]
fn route_no_filters_returns_all() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 5);
    assert_eq!(result.models.len(), 5);
    assert!(!result.has_more);
}

#[test]
fn route_filter_by_provider() {
    let store = test_store();
    let query = RouteQuery {
        provider: Some("test-provider".to_string()),
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 3);
}

#[test]
fn route_filter_by_open_weights() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: Some(true),
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 2);
    for m in &result.models {
        assert_eq!(m.open_weights, Some(true));
    }
}

#[test]
fn route_filter_by_min_context() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: Some(100_000),
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 2);
    for m in &result.models {
        let ctx = m.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
        assert!(ctx >= 100_000);
    }
}

#[test]
fn route_filter_by_max_price() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: Some(1.0),
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 3);
}

#[test]
fn route_filter_by_input_modality() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: Some(vec![ModelModality::Text, ModelModality::Image]),
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 2);
}

#[test]
fn route_filter_by_features() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: Some(ModelFeatures {
            tool_call: Some(true),
            reasoning: Some(true),
            attachment: None,
            structured_output: None,
            temperature: None,
        }),
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    assert_eq!(result.total, 2);
}

#[test]
fn route_pagination() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: Some(SortField::Id),
        order: Some(SortOrder::Asc),
        limit: Some(2),
        offset: Some(0),
    };
    let result = route(&store, &query);
    assert_eq!(result.models.len(), 2);
    assert_eq!(result.total, 5);
    assert!(result.has_more);
}

#[test]
fn route_offset_beyond_total() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: None,
        order: None,
        limit: Some(10),
        offset: Some(100),
    };
    let result = route(&store, &query);
    assert_eq!(result.models.len(), 0);
    assert_eq!(result.total, 5);
    assert!(!result.has_more);
}

#[test]
fn route_sort_by_context_desc() {
    let store = test_store();
    let query = RouteQuery {
        provider: None,
        input_modalities: None,
        output_modalities: None,
        features: None,
        min_context: None,
        max_input_price: None,
        max_output_price: None,
        open_weights: None,
        sort: Some(SortField::Context),
        order: Some(SortOrder::Desc),
        limit: None,
        offset: None,
    };
    let result = route(&store, &query);
    let first_ctx = result.models[0]
        .limit
        .as_ref()
        .and_then(|l| l.context)
        .unwrap_or(0);
    let last_ctx = result.models[4]
        .limit
        .as_ref()
        .and_then(|l| l.context)
        .unwrap_or(0);
    assert!(first_ctx >= last_ctx);
}

#[test]
fn fallback_returns_alternatives() {
    let store = test_store();
    let chain =
        fallback_chain(&store, "model-small", &FallbackOptions::default()).unwrap();
    assert_eq!(chain.original.id, "model-small");
    assert!(!chain.models.is_empty());
    assert!(chain.models.iter().all(|m| m.id != "model-small"));
}

#[test]
fn fallback_model_not_found() {
    let store = test_store();
    let result = fallback_chain(&store, "nonexistent", &FallbackOptions::default());
    assert!(result.is_err());
}

#[test]
fn fallback_match_features() {
    let store = test_store();
    let chain = fallback_chain(
        &store,
        "model-small",
        &FallbackOptions {
            match_features: Some(true),
            ..FallbackOptions::default()
        },
    )
    .unwrap();
    for m in &chain.models {
        assert_eq!(m.features.as_ref().and_then(|f| f.tool_call), Some(true));
    }
}

#[test]
fn fallback_limits_results() {
    let store = test_store();
    let chain = fallback_chain(
        &store,
        "model-small",
        &FallbackOptions {
            limit: Some(2),
            ..FallbackOptions::default()
        },
    )
    .unwrap();
    assert!(chain.models.len() <= 2);
}

#[test]
fn fallback_prefers_same_provider() {
    let store = test_store();
    let chain =
        fallback_chain(&store, "model-small", &FallbackOptions::default()).unwrap();
    if chain.models.len() > 1 {
        let same_provider_count = chain
            .models
            .iter()
            .filter(|m| m.provider == "test-provider")
            .count();
        let other_provider_count = chain
            .models
            .iter()
            .filter(|m| m.provider != "test-provider")
            .count();
        assert!(same_provider_count >= other_provider_count);
    }
}

#[test]
fn context_fit_model_fits() {
    let store = test_store();
    let fit = check_context_fit(&store, "model-large", 50_000, Some(10_000)).unwrap();
    assert!(fit.fits);
    assert_eq!(fit.available_context, 128_000);
    assert_eq!(fit.requested_tokens, 60_000);
    assert!(!fit.should_compact);
    assert!(fit.overhead > 0);
}

#[test]
fn context_fit_model_does_not_fit() {
    let store = test_store();
    let fit = check_context_fit(&store, "model-small", 5_000, None).unwrap();
    assert!(!fit.fits);
    assert!(fit.should_compact);
}

#[test]
fn context_fit_model_not_found() {
    let store = test_store();
    let result = check_context_fit(&store, "nonexistent", 100, None);
    assert!(result.is_err());
}

#[test]
fn context_fit_suggests_alternatives() {
    let store = test_store();
    let fit = check_context_fit(&store, "model-small", 100_000, None).unwrap();
    assert!(!fit.fits);
    assert!(!fit.better_alternatives.is_empty());
    for alt in &fit.better_alternatives {
        let ctx = alt.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
        assert!(ctx > 4096);
    }
}

#[test]
fn find_best_context_model_basic() {
    let store = test_store();
    let best = find_best_context_model(&store, 100_000, None);
    assert!(best.is_some());
    let m = best.unwrap();
    let ctx = m.limit.as_ref().and_then(|l| l.context).unwrap_or(0);
    assert!(ctx >= 100_000);
}

#[test]
fn find_best_context_model_with_provider() {
    let store = test_store();
    let best = find_best_context_model(&store, 5_000, Some("test-provider"));
    assert!(best.is_some());
    assert_eq!(best.as_ref().unwrap().provider, "test-provider");
}

#[test]
fn find_best_context_model_none_when_too_large() {
    let store = test_store();
    let best = find_best_context_model(&store, 999_999_999, None);
    assert!(best.is_none());
}

#[test]
fn find_best_context_model_cheapest_first() {
    let store = test_store();
    let best = find_best_context_model(&store, 1_000, None);
    assert!(best.is_some());
    let m = best.unwrap();
    let price = m.pricing.as_ref().and_then(|p| p.input).unwrap_or(f64::MAX);
    assert_eq!(price, 0.10);
}

#[test]
fn compare_two_models() {
    let store = test_store();
    let comp = compare(&store, &["model-small", "model-large"]);
    assert_eq!(comp.models.len(), 2);
    assert!(!comp.fields.is_empty());
    let context_field = comp.fields.iter().find(|f| f.field == "context").unwrap();
    assert_eq!(context_field.winner.as_deref(), Some("model-large"));
    let input_price_field = comp
        .fields
        .iter()
        .find(|f| f.field == "input_price")
        .unwrap();
    assert_eq!(input_price_field.winner.as_deref(), Some("model-small"));
}

#[test]
fn compare_single_model_returns_empty_fields() {
    let store = test_store();
    let comp = compare(&store, &["model-small"]);
    assert_eq!(comp.models.len(), 1);
    assert!(comp.fields.is_empty());
}

#[test]
fn compare_unknown_models_skipped() {
    let store = test_store();
    let comp = compare(&store, &["model-small", "nonexistent"]);
    assert_eq!(comp.models.len(), 1);
    assert!(comp.fields.is_empty());
}

#[test]
fn compare_three_models() {
    let store = test_store();
    let comp = compare(&store, &["model-small", "model-large", "other-cheap"]);
    assert_eq!(comp.models.len(), 3);
    let open_weights_field = comp
        .fields
        .iter()
        .find(|f| f.field == "open_weights")
        .unwrap();
    for (id, val) in &open_weights_field.values {
        if id == "model-large" || id == "other-cheap" {
            assert_eq!(*val, FieldValue::Bool(Some(true)));
        } else {
            assert_eq!(*val, FieldValue::Bool(None));
        }
    }
}
