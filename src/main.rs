use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use libsql::Database;
use serde::{Deserialize, Serialize};

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
    let conn = client.connect().unwrap();

    let mut rows = conn
        .query(
            "select signature_name, points, timestamp from score where scoreboard_id = ?1;",
            libsql::params![scoreboard_id.to_string()],
        )
        .await
        .unwrap();
    let mut scores = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        scores.push(CompleteScore {
            scoreboard_id: scoreboard_id.clone(),
            signature_name: row.get::<String>(0).unwrap(),
            points: row.get::<u32>(1).unwrap(),
            timestamp: row.get::<String>(2).unwrap(),
        });
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

async fn create_score_handler(
    State(client): State<Arc<Database>>,
    Json(score): Json<ScorePostPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let conn = client.connect().unwrap();
    conn.execute(
        "insert into score (scoreboard_id, signature_name, points) values (?1, '?2', ?3);",
        [
            format!("{}", score.scoreboard_id),
            score.signature_name.clone(),
            format!("{}", score.points),
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
    // let conn = client.connect().unwrap();

    // conn.execute(
    //     "create table if not exists example_users ( uid text primary key, email text );",
    //     (),
    // )
    // .await
    // .unwrap();
    // let mut rows = conn
    //     .query("SELECT * FROM sqlite_master WHERE type='table';", ())
    //     .await
    //     .unwrap();

    // let r1 = rows.next().await.unwrap();
    // println!("{:?}", r1);

    let router = Router::new()
        .route(
            "/scores/:scoreboard_id",
            get(get_scores_handler).post(create_score_handler),
        )
        .with_state(client);

    Ok(router.into())
}
