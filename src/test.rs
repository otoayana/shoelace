use crate::{api, front, proxy, ShoelaceData};
use actix_web::{test, web, App};
use spools::{Post, User};
use std::collections::HashMap;
use tokio::sync::Mutex;

const TEST_APP_DATA: ShoelaceData = ShoelaceData {
    store: crate::proxy::Keystore::None,
    log_cdn: false,
    base_url: String::new(),
    rev: String::new(),
    rss: false,
};

// Tests user frontend
#[actix_web::test]
async fn user_fe() {
    // Creates an environment using the user frontend and no proxy
    let app = test::init_service(
        App::new()
            .service(front::user)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    // Fetches a user
    let req = test::TestRequest::get().uri("/@zuck").to_request();
    let resp = test::call_service(&app, req).await;

    // Checks if the service sent a successful response
    println!("{:#?}", resp);
    assert!(resp.status().is_success());
}

// Tests post frontend
#[actix_web::test]
async fn post_fe() {
    // Creates an environment using the post frontend and no proxy
    let app = test::init_service(
        App::new()
            .service(front::post)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    // Fetches a post
    let req = test::TestRequest::get().uri("/t/C2QBoRaRmR1").to_request();
    let resp = test::call_service(&app, req).await;

    // Checks if the service sent a successful response
    println!("{:#?}", resp);
    assert!(resp.status().is_success());
}

// Tests user API
#[actix_web::test]
async fn user_api() {
    // Creates an environment using the user API and no proxy
    let app = test::init_service(
        App::new()
            .service(api::user)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    // Fetches a user
    let req = test::TestRequest::get().uri("/user/zuck").to_request();
    let resp: User = test::call_and_read_body_json(&app, req).await;

    // Determines if the user ID is correct
    println!("{:#?}", resp);
    assert_eq!(resp.id, 314216)
}

// Tests post API
#[actix_web::test]
async fn post_api() {
    // Creates an environment using the post API and no proxy
    let app = test::init_service(
        App::new()
            .service(api::post)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    // Fetches a post
    let req = test::TestRequest::get()
        .uri("/post/C2QBoRaRmR1")
        .to_request();
    let resp: Post = test::call_and_read_body_json(&app, req).await;

    // Determines if the post's author is correct
    println!("{:#?}", resp);
    assert_eq!(resp.id, "3283131293873103989")
}

// Tests proxy
#[actix_web::test]
async fn proxy() {
    // Creates an environment using the user API and the proxy with an internal backend
    let app = test::init_service(
        App::new()
            .service(api::user)
            .service(web::scope("/proxy").service(proxy::serve))
            .app_data(web::Data::new(ShoelaceData {
                store: crate::proxy::Keystore::Internal(Mutex::new(HashMap::new())),
                log_cdn: false,
                base_url: "".to_string(),
                rev: String::new(),
                rss: false,
            })),
    )
    .await;

    // Requests a user through the API, in order to generate a media hash to check
    let api_req = test::TestRequest::get().uri("/user/zuck").to_request();
    let api_resp: User = test::call_and_read_body_json(&app, api_req).await;

    // Asserts if response is ok, and has content
    println!("{:#?}", api_resp);
    assert_eq!(api_resp.id, 314216);

    // Fetches the profile picture from the proxy
    let pfp = api_resp.pfp;
    let req = test::TestRequest::get().uri(&pfp).to_request();
    let resp = test::call_service(&app, req).await;

    // Asserts if response is ok
    println!("{:#?}", resp);
    assert!(resp.status().is_success());
}
