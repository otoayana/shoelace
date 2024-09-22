use std::{borrow::Borrow, sync::Arc};

use crate::{req, ShoelaceData};
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Json, Router,
};

pub fn attach(enabled: bool) -> Router<Arc<ShoelaceData>> {
    let mut routed = Router::new();

    if enabled {
        routed = routed
            .route("/user/:id", get(user))
            .route("/post/:id", get(post))
    }

    routed
}

// User API endpoint
async fn user(Path(user): Path<String>, State(store): State<Arc<ShoelaceData>>) -> Response {
    let resp = req::user(&user, store.borrow()).await;

    match resp {
        Ok(body) => (StatusCode::OK, Json(body)).into_response(),
        Err(error) => error.into_plaintext(),
    }
}

// // Post API endpoint
async fn post(Path(post): Path<String>, State(store): State<Arc<ShoelaceData>>) -> Response {
    let resp = req::post(&post, store.borrow()).await;

    match resp {
        Ok(body) => (StatusCode::OK, Json(body)).into_response(),
        Err(error) => error.into_plaintext(),
    }
}
