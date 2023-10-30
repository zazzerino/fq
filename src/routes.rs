use crate::auth::{Claims, KEYS};
use crate::{
    auth,
    auth::{AuthBody, AuthError},
    game, theory,
    user::{self, User, UserId},
};
use anyhow::Result;
use askama_axum::{IntoResponse, Template};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::{headers, Json, TypedHeader};
use futures::{sink::SinkExt, stream::StreamExt};
use jsonwebtoken::{decode, encode, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{Error, Pool, Sqlite};
use std::borrow::Cow;
use std::future::Future;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use tower_cookies::{Cookie, Cookies};
use tower_sessions::Session;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

pub async fn index_page(State(pool): State<Pool<Sqlite>>, cookies: Cookies) -> IndexTemplate {
    tracing::debug!("cookies: {:?}", cookies);
    IndexTemplate {}
}

#[derive(Template)]
#[template(path = "user.html")]
pub struct UserTemplate {
    id: UserId,
    name: String,
}

pub async fn user_page(
    State(pool): State<Pool<Sqlite>>,
    cookies: Cookies,
) -> Result<UserTemplate, StatusCode> {
    match cookies.get(auth::USER_COOKIE) {
        Some(cookie) => {
            let claims = decode::<Claims>(cookie.value(), &KEYS.decoding, &Validation::default())
                .map_err(|_| StatusCode::UNAUTHORIZED)?
                .claims;

            Ok(UserTemplate {
                id: claims.sub,
                name: claims.name,
            })
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
    // match user::fetch_user(&pool, id).await {
    //     Ok(User { name, .. }) => Ok(UserTemplate { name }),
    //     _ => Err(StatusCode::NOT_FOUND),
    // }
}

#[derive(Template)]
#[template(path = "game.html")]
pub struct GameTemplate {
    id: game::GameId,
    status: String,
    note: String,
    player_ids: String,
}

// async fn get_or_create_user(pool: &Pool<Sqlite>, session_id: &str) -> Result<user::UserId> {
//     match user::fetch_user_by_session_id(pool, session_id).await {
//         Ok(user) => Ok(user.id),
//         _ => Ok(user::create_user(pool, session_id)
//             .await?
//             .last_insert_rowid()),
//     }
// }

// fn get_user_cookie(cookies: &Cookies) -> Option<i64> {
//     cookies
//         .get(USER_COOKIE)
//         .and_then(|c| c.value().parse::<_>().ok())
// }

// pub async fn index_page(State(pool): State<Pool<Sqlite>>, cookies: Cookies) -> IndexTemplate {
//     if cookies.get(USER_COOKIE).is_none() {
//         let cookie = create_user_and_cookie(&pool).await.unwrap();
//         cookies.add(cookie);
//     }

//     IndexTemplate {}
// }

// async fn create_user_and_cookie<'a>(pool: &Pool<Sqlite>) -> Result<Cookie<'a>, Error> {
//     let user_id = user::db::create_user(pool).await?.last_insert_rowid();
//     let cookie = Cookie::new(USER_COOKIE, user_id.to_string());
//     Ok(cookie)
// }

// pub async fn game_page(
//     State(pool): State<Pool<Sqlite>>,
//     Path(game_id): Path<i64>,
// ) -> Result<GameTemplate, StatusCode> {
//     match game::db::fetch_game(&pool, game_id).await {
//         Ok(game) => Ok(GameTemplate {
//             id: game.id.unwrap(),
//             status: game.status.to_string(),
//             note: game
//                 .curr_round()
//                 .map(|r| r.note_to_guess.string_repr())
//                 .unwrap_or_default(),
//             player_ids: game.player_ids.iter().map(|id| id.to_string()).collect(),
//         }),
//         _ => Err(StatusCode::NOT_FOUND),
//     }
// }

// pub async fn handle_game_create(
//     State(pool): State<Pool<Sqlite>>,
//     cookies: Cookies,
// ) -> Result<impl IntoResponse, StatusCode> {
//     match get_user_cookie(&cookies) {
//         Some(user_id) => {
//             let game = Game::new(user_id);

//             if let Ok(game_id) = game::db::insert_game(&pool, game).await {
//                 let game_url = format!("/games/{}", game_id);
//                 Ok(Redirect::to(game_url.as_str()))
//             } else {
//                 Err(StatusCode::INTERNAL_SERVER_ERROR)
//             }
//         }
//         _ => Err(StatusCode::BAD_REQUEST),
//     }
// }

// pub async fn handle_game_start(
//     Path(game_id): Path<i64>,
//     State(pool): State<Pool<Sqlite>>,
//     cookies: Cookies,
// ) -> Result<impl IntoResponse, StatusCode> {
//     match get_user_cookie(&cookies) {
//         Some(user_id) => {
//             let mut game = fetch_game(&pool, game_id).await.unwrap();
//
//             if user_id == game.host_id.unwrap() {
//                 game.status = Status::Playing;
//                 Ok(Redirect::to("/"))
//             } else {
//                 Err(StatusCode::UNAUTHORIZED)
//             }
//         }
//         _ => Err(StatusCode::EXPECTATION_FAILED),
//     }
// }
