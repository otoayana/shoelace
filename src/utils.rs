extern crate reqwest;

use anyhow::Result;
use rand::distributions::{Alphanumeric, DistString};
use reqwest::{
    header::{CONTENT_TYPE, USER_AGENT},
    Client,
};
use serde_json::Value;

#[tokio::main]
pub async fn query(variables: &str, doc_id: &str) -> Result<Value> {
    let lsd = Alphanumeric.sample_string(&mut rand::thread_rng(), 11);
    let params = [
        ("lsd", lsd.as_str()),
        ("variables", &format!("{{{},\"__relay_internal__pv__BarcelonaIsLoggedInrelayprovider\":false,\"__relay_internal__pv__BarcelonaIsOriginalPostPillEnabledrelayprovider\":false,\"__relay_internal__pv__BarcelonaIsThreadContextHeaderEnabledrelayprovider\":false,
	\"__relay_internal__pv__BarcelonaIsSableEnabledrelayprovider\":false,\"__relay_internal__pv__BarcelonaUseCometVideoPlaybackEnginerelayprovider\":false,\"__relay_internal__pv__BarcelonaOptionalCookiesEnabledrelayprovider\":true,\"__relay_internal__pv__BarcelonaShouldShowFediverseM075Featuresrelayprovider\":false}}", variables)),
        ("doc_id", doc_id),
    ];

    let client: Client = Client::new();
    let resp = client
        .post("https://www.threads.net/api/graphql")
        .form(&params)
        .header(USER_AGENT, "Mozilla")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("Sec-Fetch-Site", "same-origin")
        .header("X-FB-LSD", lsd)
        .send()
        .await?;

    let deser = resp.json::<Value>().await?;
    Ok(deser)
}

#[tokio::main]
pub async fn post_id(username: &str, id: &str) -> Result<String> {
    let client: reqwest::Client = reqwest::Client::new();
    let resp = client
        .get(format!("https://www.threads.net/@{}/post/{}", username, id))
        .header(
            USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0",
        )
        .header("Sec-Fetch-Mode", "navigate")
        .send()
        .await?
        .text()
        .await?;

    let id_location = resp.find("post_id").unwrap();
    let mut cur = id_location + 10;
    let mut curchar = resp.as_bytes()[cur] as char;
    let mut id = String::new();

    while curchar != '\"' {
        id.push(curchar);
        cur += 1;
        curchar = resp.as_bytes()[cur] as char;
    }

    Ok(id)
}
