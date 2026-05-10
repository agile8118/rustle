// Seeds the database with two demo users, a board with three columns,
// a few cards, comments, and labels. Creates the database first if it
// doesn't yet exist.
//
// Usage:
//     cargo run --bin seed                           # uses DATABASE_URL from .env
//     DATABASE_URL=postgres://...  cargo run --bin seed
//
// Demo accounts (password = `password123` for both):
//     ada@rustle.dev
//     turing@rustle.dev

use rustle::auth::password;
use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Postgres, Row};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set (in .env or env)"))?;

    if !Postgres::database_exists(&url).await.unwrap_or(false) {
        println!("• Database not found — creating…");
        Postgres::create_database(&url).await?;
        println!("  ✓ created");
    } else {
        println!("• Database already exists");
    }

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await?;

    println!("• Running migrations…");
    sqlx::migrate!("./migrations").run(&pool).await?;
    println!("  ✓ migrations applied");

    println!("• Wiping existing data (cards/columns/boards/users)…");
    sqlx::query("TRUNCATE users RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await?;

    println!("• Seeding users + board…");

    let ada_hash = password::hash("password123").map_err(to_any)?;
    let turing_hash = password::hash("password123").map_err(to_any)?;

    let ada_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("ada@rustle.dev")
    .bind(&ada_hash)
    .bind("Ada Lovelace")
    .fetch_one(&pool)
    .await?;

    let _turing_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("turing@rustle.dev")
    .bind(&turing_hash)
    .bind("Alan Turing")
    .fetch_one(&pool)
    .await?;

    let board_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO boards (owner_id, title) VALUES ($1, $2) RETURNING id",
    )
    .bind(ada_id)
    .bind("Launch checklist")
    .fetch_one(&pool)
    .await?;

    let mut col_ids = Vec::new();
    for (i, name) in ["To Do", "In Progress", "Done"].iter().enumerate() {
        let row = sqlx::query(
            "INSERT INTO board_columns (board_id, title, position) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(board_id)
        .bind(name)
        .bind(i as i32)
        .fetch_one(&pool)
        .await?;
        col_ids.push(row.get::<uuid::Uuid, _>("id"));
    }

    let cards = [
        (col_ids[0], "Write the press release", "Lead with the user benefit, not the tech."),
        (col_ids[0], "Audit accessibility", "Tab order, focus rings, contrast."),
        (col_ids[1], "Set up Postgres on staging", "Use the same major version as prod."),
        (col_ids[1], "Wire up monitoring", "Latency p95 + DB connection saturation."),
        (col_ids[2], "Pick a name", "Done — went with Rustle."),
    ];
    let mut card_ids = Vec::new();
    for (pos, (col, title, desc)) in cards.iter().enumerate() {
        let id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO cards (column_id, title, description, position)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(col)
        .bind(title)
        .bind(desc)
        .bind((pos % 2) as i32)
        .fetch_one(&pool)
        .await?;
        card_ids.push(id);
    }

    sqlx::query("INSERT INTO comments (card_id, author_id, body) VALUES ($1, $2, $3)")
        .bind(card_ids[0])
        .bind(ada_id)
        .bind("Draft is in the team doc — feedback welcome.")
        .execute(&pool)
        .await?;

    let label_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO labels (owner_id, name, color) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(ada_id)
    .bind("urgent")
    .bind("#dc2626")
    .fetch_one(&pool)
    .await?;
    sqlx::query("INSERT INTO card_labels (card_id, label_id) VALUES ($1, $2)")
        .bind(card_ids[0])
        .bind(label_id)
        .execute(&pool)
        .await?;

    println!();
    println!("✅ Seed complete.");
    println!();
    println!("   Sign in at http://127.0.0.1:7070/login with:");
    println!("     ada@rustle.dev      / password123");
    println!("     turing@rustle.dev   / password123");
    Ok(())
}

fn to_any(e: rustle::error::AppError) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}
