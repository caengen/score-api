use std::sync::Arc;

use anyhow::format_err;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use libsql::Database;
use serde::{Deserialize, Serialize};
use shuttle_runtime::SecretStore;
use tracing::info;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CompleteScore {
    scoreboard_id: u32,
    signature_name: String,
    points: u32,
    timestamp: String,
}
async fn get_scores_handler(
    Path(scoreboard_id): Path<u32>,
    State(client): State<Arc<Database>>,
) -> Json<Vec<CompleteScore>> {
    info!("Establishing connection...");
    let conn = client.connect().unwrap();
    info!("Connected to database.");
    info!("selecting rows for scoreboard_id {}", scoreboard_id);
    // let mut rows = conn
    //     .query(
    //         "select signature_name, points, timestamp from score where scoreboard_id = ?0;",
    //         libsql::params![scoreboard_id],
    //     )
    //     .await
    //     .unwrap();
    let res = conn
        .query(
            "select signature_name, points, timestamp from score where scoreboard_id = ?1 order by points DESC;",
            libsql::params![scoreboard_id],
        )
        .await;

    let mut scores = vec![];
    match res {
        Ok(mut rows) => {
            info!("Query completed.");
            while let Some(row) = rows.next().await.unwrap() {
                scores.push(CompleteScore {
                    scoreboard_id: scoreboard_id.clone(),
                    signature_name: row.get::<String>(0).unwrap(),
                    points: row.get::<u32>(1).unwrap(),
                    timestamp: row.get::<String>(2).unwrap(),
                });
                info!("Pushed result row");
            }
        }
        Err(e) => {
            info!("{}", e.to_string());
        }
    }

    Json(scores)
}

#[derive(Debug, Deserialize, Serialize)]
enum Game {
    TetrisRS = 1,
}

// #[derive(Debug, Deserialize, Serialize, Clone)]
// enum Scoreboard {
//     TetrisRSScoreboard = 1,
// }
// impl Display for Scoreboard {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", Into::<i32>::into(self.clone()))
//     }
// }

// impl Into<i32> for Scoreboard {
//     fn into(self) -> i32 {
//         self as i32
//     }
// }

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ScorePostPayload {
    scoreboard_id: u32,
    signature_name: String,
    points: u32,
}
async fn root(State(client): State<Arc<Database>>) -> Json<String> {
    Json("Hello, world!".to_string())
}

async fn create_score_handler(
    State(client): State<Arc<Database>>,
    Json(score): Json<ScorePostPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = client.connect().unwrap();
    conn.execute(
        "insert into score (scoreboard_id, signature_name, points, timestamp) values (?1, ?2, ?3, datetime());",
        libsql::params![
            score.scoreboard_id,
            score.signature_name.clone(),
            score.points,
        ],
    )
    .await
    .unwrap();

    Ok((StatusCode::CREATED, Json(score)))
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_turso::Turso(addr = "{secrets.TURSO_DB_URL}", token = "{secrets.TURSO_DB_TOKEN}")]
    client: Database,
) -> shuttle_axum::ShuttleAxum {
    let client = Arc::new(client);

    let router = Router::new()
        .route("/", get(root))
        .route(
            "/scores/:scoreboard_id",
            get(get_scores_handler).post(create_score_handler),
        )
        .with_state(client);

    Ok(router.into())
}
