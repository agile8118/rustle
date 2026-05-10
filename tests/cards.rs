mod common;

use common::{register_user, spawn};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn move_card_between_columns(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "frank@example.com", "frankpassword11").await;

    // Create board (gets default 3 columns)
    let board: serde_json::Value = app
        .client
        .post(app.url("/api/boards"))
        .json(&serde_json::json!({ "title": "Work" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let board_id = board["id"].as_str().unwrap().to_string();

    let detail: serde_json::Value = app
        .client
        .get(app.url(&format!("/api/boards/{}", board_id)))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let cols = detail["columns"].as_array().unwrap().clone();
    let todo_id = cols[0]["id"].as_str().unwrap().to_string();
    let doing_id = cols[1]["id"].as_str().unwrap().to_string();

    // Add three cards to To Do
    let mut card_ids = vec![];
    for t in ["A", "B", "C"] {
        let c: serde_json::Value = app
            .client
            .post(app.url(&format!("/api/columns/{}/cards", todo_id)))
            .json(&serde_json::json!({ "title": t }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        card_ids.push(c["id"].as_str().unwrap().to_string());
    }

    // Move B (index 1) to Doing at position 0
    let res = app
        .client
        .post(app.url(&format!("/api/cards/{}/move", card_ids[1])))
        .json(&serde_json::json!({
            "column_id": doing_id,
            "position": 0,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Refetch board, verify positions
    let detail2: serde_json::Value = app
        .client
        .get(app.url(&format!("/api/boards/{}", board_id)))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let cols2 = detail2["columns"].as_array().unwrap();
    let todo_cards = cols2[0]["cards"].as_array().unwrap();
    let doing_cards = cols2[1]["cards"].as_array().unwrap();
    assert_eq!(todo_cards.len(), 2, "todo should have A and C");
    assert_eq!(doing_cards.len(), 1, "doing should have B");
    assert_eq!(doing_cards[0]["title"], "B");

    // Positions should be contiguous starting at 0
    for (i, c) in todo_cards.iter().enumerate() {
        assert_eq!(c["position"].as_i64().unwrap(), i as i64);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_card_cascades_comments(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "gary@example.com", "garypassword11").await;

    let board: serde_json::Value = app
        .client
        .post(app.url("/api/boards"))
        .json(&serde_json::json!({ "title": "B" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let detail: serde_json::Value = app
        .client
        .get(app.url(&format!("/api/boards/{}", board["id"].as_str().unwrap())))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let col_id = detail["columns"][0]["id"].as_str().unwrap().to_string();

    let card: serde_json::Value = app
        .client
        .post(app.url(&format!("/api/columns/{}/cards", col_id)))
        .json(&serde_json::json!({ "title": "X" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let card_id = card["id"].as_str().unwrap().to_string();

    // Add comment
    let res = app
        .client
        .post(app.url(&format!("/api/cards/{}/comments", card_id)))
        .json(&serde_json::json!({ "body": "Looks good" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Delete card
    let res = app
        .client
        .delete(app.url(&format!("/api/cards/{}", card_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Verify directly in DB that no comment is left
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_label_and_attach_to_card(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "hank@example.com", "hankpassword11").await;

    let board: serde_json::Value = app
        .client
        .post(app.url("/api/boards"))
        .json(&serde_json::json!({ "title": "B" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let detail: serde_json::Value = app
        .client
        .get(app.url(&format!("/api/boards/{}", board["id"].as_str().unwrap())))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let col_id = detail["columns"][0]["id"].as_str().unwrap().to_string();
    let card: serde_json::Value = app
        .client
        .post(app.url(&format!("/api/columns/{}/cards", col_id)))
        .json(&serde_json::json!({ "title": "X" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let card_id = card["id"].as_str().unwrap();

    let label: serde_json::Value = app
        .client
        .post(app.url("/api/labels"))
        .json(&serde_json::json!({ "name": "urgent", "color": "#ff0000" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let label_id = label["id"].as_str().unwrap();

    let res = app
        .client
        .post(app.url(&format!("/api/cards/{}/labels", card_id)))
        .json(&serde_json::json!({ "label_id": label_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM card_labels")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}
