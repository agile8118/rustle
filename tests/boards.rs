mod common;

use common::{register_user, spawn};
use reqwest::Client;
use sqlx::PgPool;

fn fresh_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn create_list_get_update_delete_board(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "amy@example.com", "longpassword123").await;

    // create
    let board: serde_json::Value = app
        .client
        .post(app.url("/api/boards"))
        .json(&serde_json::json!({ "title": "My first board" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let board_id = board["id"].as_str().unwrap().to_string();
    assert_eq!(board["title"], "My first board");

    // list -> contains the board
    let list: serde_json::Value = app
        .client
        .get(app.url("/api/boards"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // get -> includes the three default columns
    let detail: serde_json::Value = app
        .client
        .get(app.url(&format!("/api/boards/{}", board_id)))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let cols = detail["columns"].as_array().unwrap();
    assert_eq!(cols.len(), 3);
    assert_eq!(cols[0]["title"], "To Do");

    // update title
    let updated: serde_json::Value = app
        .client
        .patch(app.url(&format!("/api/boards/{}", board_id)))
        .json(&serde_json::json!({ "title": "Renamed" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["title"], "Renamed");

    // delete
    let res = app
        .client
        .delete(app.url(&format!("/api/boards/{}", board_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // list -> empty
    let list2: serde_json::Value = app
        .client
        .get(app.url("/api/boards"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list2.as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn cross_user_isolation(pool: PgPool) {
    let app_a = spawn(pool.clone()).await;
    register_user(&app_a, "owner@example.com", "ownersecret11").await;
    let board: serde_json::Value = app_a
        .client
        .post(app_a.url("/api/boards"))
        .json(&serde_json::json!({ "title": "Owner board" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let board_id = board["id"].as_str().unwrap().to_string();

    // Different user with a fresh client + cookie jar
    let intruder = fresh_client();
    intruder
        .post(format!("{}/api/auth/register", app_a.address))
        .json(&serde_json::json!({
            "email": "intruder@example.com",
            "password": "intrudersecret11",
            "display_name": "intruder",
        }))
        .send()
        .await
        .unwrap();

    // Intruder cannot see the board
    let list: serde_json::Value = intruder
        .get(format!("{}/api/boards", app_a.address))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);

    // Intruder gets 404 on direct fetch
    let direct = intruder
        .get(format!("{}/api/boards/{}", app_a.address, board_id))
        .send()
        .await
        .unwrap();
    assert_eq!(direct.status(), 404);

    // Intruder cannot delete it
    let del = intruder
        .delete(format!("{}/api/boards/{}", app_a.address, board_id))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), 404);
}
