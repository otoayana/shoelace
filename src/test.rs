use crate::{api, frontend, proxy, ShoelaceData};
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

#[actix_web::test]
async fn user_fe() {
    let app = test::init_service(
        App::new()
            .service(frontend::user)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    let req = test::TestRequest::get().uri("/@zuck").to_request();
    let resp = test::call_service(&app, req).await;

    println!("{:#?}", resp);
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn post_fe() {
    let app = test::init_service(
        App::new()
            .service(frontend::post)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    let req = test::TestRequest::get().uri("/t/C2QBoRaRmR1").to_request();
    let resp = test::call_service(&app, req).await;

    println!("{:#?}", resp);
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn user_api() {
    let app = test::init_service(
        App::new()
            .service(api::user)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    let req = test::TestRequest::get().uri("/user/zuck").to_request();
    let resp: User = test::call_and_read_body_json(&app, req).await;

    println!("{:#?}", resp);
    assert_eq!(resp.id, 314216)
}

#[actix_web::test]
async fn post_api() {
    let app = test::init_service(
        App::new()
            .service(api::post)
            .app_data(web::Data::new(TEST_APP_DATA)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/post/C2QBoRaRmR1")
        .to_request();
    let resp: Post = test::call_and_read_body_json(&app, req).await;

    println!("{:#?}", resp);
    assert_eq!(resp.id, "3283131293873103989")
}

#[actix_web::test]
async fn proxy() {
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

    // In order to test proxy functionality, we need to generate a media hash to check
    let api_req = test::TestRequest::get().uri("/user/zuck").to_request();
    let api_resp: User = test::call_and_read_body_json(&app, api_req).await;

    println!("{:#?}", api_resp);
    assert_eq!(api_resp.id, 314216);

    let pfp = api_resp.pfp;
    let req = test::TestRequest::get().uri(&pfp).to_request();
    let resp = test::call_service(&app, req).await;

    println!("{:#?}", resp);
    assert!(resp.status().is_success());
}
