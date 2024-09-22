use std::{borrow::Borrow, sync::Arc};

use crate::{
    frontend::templates::{HomeView, PostView, UserView},
    req, Error, ShoelaceData,
};
use askama_axum::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
    routing::get,
    Router,
};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use spools::{Post, User};
use tower_serve_static::ServeDir;

static ASSETS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/static");

#[derive(Debug, Deserialize, Serialize)]
enum ResponseTypes {
    Post(Post),
    User(User),
}

pub fn attach(enabled: bool) -> Router<Arc<ShoelaceData>> {
    let assets = ServeDir::new(&ASSETS_DIR);
    let mut routed = Router::new();

    if enabled {
        routed = routed
            .route("/", get(home))
            .route("/@:id", get(user))
            .route("/t/:id", get(post))
            .route("/find", get(find))
            .route("/:any/post/:id", get(redirect))
            .nest_service("/static", assets);
    }

    routed
}

// For user find form
#[derive(Debug, Deserialize)]
struct Find {
    value: String,
}

// Landing page
async fn home(State(state): State<Arc<ShoelaceData>>) -> Result<Html<String>, Error> {
    let template = HomeView {
        base: state.base.clone(),
    }
    .render()?;

    Ok(Html(template))
}

// User frontend
async fn user(
    Path(user): Path<String>,
    State(state): State<Arc<ShoelaceData>>,
) -> Result<Html<String>, Error> {
    let mut base = state.base.clone();

    base.timer(true)?;
    let req = req::user(&user, state.borrow()).await?;
    base.timer(false)?;

    let template = UserView {
        base,
        input: &user,
        output: req,
    }
    .render()?;

    Ok(Html(template))
}

// Post frontend
async fn post(
    Path(post): Path<String>,
    State(state): State<Arc<ShoelaceData>>,
) -> Result<Html<String>, Error> {
    let mut base = state.base.clone();

    base.timer(true)?;
    let req = req::post(&post, state.borrow()).await?;
    base.timer(false)?;

    let template = PostView {
        base,
        input: &post,
        output: req,
    }
    .render()?;

    Ok(Html(template))
}

// User finder endpoint
async fn find(Query(request): Query<Find>) -> Redirect {
    Redirect::temporary(&format!("/@{}", request.value))
}

// Post redirect endpoint
async fn redirect(Path(request): Path<((), String)>) -> Redirect {
    Redirect::temporary(&format!("/t/{}", request.1))
}
