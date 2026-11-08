use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::{
    Shared,
    error::AppError,
    inspect, pipeline, portfolio,
    solana::Pubkey,
    stream::ndjson,
    types::{
        ConfirmResponse, InspectRequest, MarketResponse, MarketToken, OwnerQuery, OwnerRequest, PlanRequest, SignatureQuery,
        TamperRequest,
    },
};

fn failure(err: AppError) -> Response {
    let status = if err.is_user() { StatusCode::BAD_REQUEST } else { StatusCode::INTERNAL_SERVER_ERROR };
    if !err.is_user() {
        eprintln!("request failed: {}", err.message());
    }
    (status, Json(json!({ "error": err.message() }))).into_response()
}

fn invalid_owner() -> Response {
    failure(AppError::User("That wallet address isn't valid.".to_string()))
}

pub async fn health(State(st): State<Shared>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "ai": st.cfg.anthropic_key.is_some(),
        "model": st.cfg.model,
        "jupiterKey": st.cfg.jup_key.is_some(),
        "guardProgram": st.cfg.guard_program.map(|p| p.to_string()),
        "rpc": st.cfg.rpc_host(),
    }))
}

pub async fn portfolio(State(st): State<Shared>, Query(query): Query<OwnerQuery>) -> Response {
    let Some(owner) = Pubkey::parse(&query.owner) else {
        return invalid_owner();
    };
    if query.refresh.unwrap_or(false) {
        portfolio::invalidate(&st, &query.owner);
    }
    match portfolio::load(&st, &owner).await {
        Ok(portfolio) => Json(portfolio).into_response(),
        Err(err) => failure(err),
    }
}

pub async fn plan(State(st): State<Shared>, Json(request): Json<PlanRequest>) -> Response {
    ndjson(move |em| async move {
        let _permit = st.limiter.clone().acquire_owned().await.map_err(anyhow::Error::from)?;
        pipeline::run(st, request, em).await
    })
}

pub async fn inspect(State(st): State<Shared>, Json(request): Json<InspectRequest>) -> Response {
    ndjson(move |em| async move {
        let _permit = st.limiter.clone().acquire_owned().await.map_err(anyhow::Error::from)?;
        inspect::run(st, request, em).await
    })
}

pub async fn sample(State(st): State<Shared>, Json(request): Json<OwnerRequest>) -> Response {
    let Some(owner) = Pubkey::parse(&request.owner) else {
        return invalid_owner();
    };
    match inspect::sample(&st, &owner).await {
        Ok(sample) => Json(sample).into_response(),
        Err(err) => failure(err),
    }
}

pub async fn tamper(State(st): State<Shared>, Json(request): Json<TamperRequest>) -> Response {
    match inspect::tamper(&st, &request).await {
        Ok(result) => Json(result).into_response(),
        Err(err) => failure(err),
    }
}

const MARKET_MINTS: &[&str] = &[
    "So11111111111111111111111111111111111111112",
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
    "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
    "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL",
];

pub async fn market(State(st): State<Shared>) -> Response {
    if let Ok(cache) = st.market.lock() {
        if let Some((at, value)) = cache.as_ref() {
            if at.elapsed() < std::time::Duration::from_secs(15) {
                return Json(value.clone()).into_response();
            }
        }
    }
    let mints: Vec<String> = MARKET_MINTS.iter().map(|m| m.to_string()).collect();
    let (network, tokens) = tokio::join!(st.rpc.network(), st.jup.tokens(&mints));
    let (slot, tps, slot_time) = match network {
        Ok(value) => value,
        Err(err) => return failure(err.into()),
    };
    let found = tokens.unwrap_or_default();
    let tokens = mints
        .iter()
        .filter_map(|mint| found.get(mint))
        .map(|token| {
            let meta = crate::tokens::meta(token);
            MarketToken {
                symbol: meta.symbol,
                mint: meta.mint,
                icon: meta.icon,
                usd_price: meta.usd_price,
                change_24h: token.stats_24h.as_ref().and_then(|s| s.price_change),
                verified: meta.verified,
            }
        })
        .collect();
    let value = MarketResponse {
        slot,
        tps,
        slot_time,
        tokens,
        updated_at: crate::time::now_ms(),
    };
    if let Ok(mut cache) = st.market.lock() {
        *cache = Some((std::time::Instant::now(), value.clone()));
    }
    Json(value).into_response()
}

pub async fn confirm(State(st): State<Shared>, Query(query): Query<SignatureQuery>) -> Response {
    let valid = bs58::decode(query.sig.trim()).into_vec().map(|b| b.len() == 64).unwrap_or(false);
    if !valid {
        return failure(AppError::User("Invalid transaction signature.".to_string()));
    }
    match st.rpc.signature_status(query.sig.trim()).await {
        Ok(status) => {
            if matches!(status.status.as_str(), "confirmed" | "finalized") {
                if let Some(owner) = &query.owner {
                    portfolio::invalidate(&st, owner);
                }
            }
            Json(ConfirmResponse {
                status: status.status,
                err: status.err,
                slot: status.slot,
            })
            .into_response()
        }
        Err(err) => failure(err.into()),
    }
}
