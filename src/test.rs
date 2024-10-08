use crate::{
    api,
    common::config::Settings,
    frontend::{self, Base},
    proxy, ShoelaceData,
};
use axum::{http::StatusCode, Router};
use axum_test::TestServer;
use spools::{Post, User};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
use tokio::sync::Mutex;

static TEST_APP_DATA: LazyLock<ShoelaceData> = LazyLock::new(|| ShoelaceData {
    store: crate::proxy::Keystore::None,
    base: Base::new().unwrap(),
    config: Settings::new().unwrap(),
});

#[tokio::test]
async fn user_fe() {
    let app = Router::new()
        .merge(frontend::routes::attach(true))
        .with_state(Arc::new(TEST_APP_DATA.clone()));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/@zuck").await;

    println!("{:#?}", response);
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn post_fe() {
    let app = Router::new()
        .merge(frontend::routes::attach(true))
        .with_state(Arc::new(TEST_APP_DATA.clone()));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/t/C2QBoRaRmR1").await;

    println!("{:#?}", response);
    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn user_api() {
    let app = Router::new()
        .nest("/api/", api::attach(true))
        .with_state(Arc::new(TEST_APP_DATA.clone()));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/user/zuck").await;
    println!("{:#?}", response);

    let user: User = response.json();
    assert_eq!(user.id, 314216)
}

#[tokio::test]
async fn post_api() {
    let app = Router::new()
        .nest("/api/", api::attach(true))
        .with_state(Arc::new(TEST_APP_DATA.clone()));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/post/C2QBoRaRmR1").await;
    println!("{:#?}", response);

    let post: Post = response.json();
    assert_eq!(post.id, "3283131293873103989")
}

#[tokio::test]
async fn proxy() {
    let app = Router::new()
        .nest("/api/", api::attach(true))
        .nest("/proxy/", proxy::attach())
        .with_state(Arc::new(ShoelaceData {
            store: crate::proxy::Keystore::Internal(Arc::new(Mutex::new(HashMap::new()))),
            base: Base::new().unwrap(),
            config: Settings::new().unwrap(),
        }));
    let server = TestServer::new(app).unwrap();

    // In order to test proxy functionality, we need to generate a media hash to check
    let api = server.get("/api/user/zuck").await;
    println!("{:#?}", api);

    let user: User = api.json();
    assert_eq!(user.id, 314216);

    let response = server.get(&user.pfp).await;

    println!("{:#?}", response);
    assert_eq!(response.status_code(), StatusCode::OK);
}
