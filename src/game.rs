use crate::theory;
use crate::theory::{Accidental, FretCoord, Note, WhiteKey};
use sqlx::query::Query;
use sqlx::sqlite::{SqliteQueryResult, SqliteRow};
use sqlx::{Acquire, Error, Pool, Row, Sqlite};
use std::fmt;

#[derive(Debug)]
pub struct Game {
    pub id: Option<i64>,
    pub host_id: Option<i64>,
    pub status: Status,
    pub player_ids: Vec<i64>,
    pub opts: Opts,
    pub rounds: Vec<Round>,
}

const CREATE_GAMES_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    host_id INTEGER NOT NULL,
    status TEXT NOT NULL,
    FOREIGN KEY(host_id) REFERENCES users(id)
);";

impl Game {
    pub fn new(host_id: i64) -> Game {
        Game {
            id: None,
            host_id: Some(host_id),
            status: Status::Init,
            opts: Opts::new(),
            rounds: vec![],
            player_ids: vec![host_id],
        }
    }
}

#[derive(Debug)]
pub enum Status {
    Init,
    Playing,
    RoundOver,
    GameOver,
    NoPlayers,
}

impl Status {
    fn from(s: &str) -> Option<Status> {
        match s {
            "Init" => Some(Status::Init),
            "Playing" => Some(Status::Playing),
            "RoundOver" => Some(Status::RoundOver),
            "GameOver" => Some(Status::GameOver),
            "NoPlayers" => Some(Status::NoPlayers),
            _ => None,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Opts {
    id: Option<i64>,
    game_id: Option<i64>,
    num_rounds: i32,
    start_fret: i32,
    end_fret: i32,
}

const CREATE_SETTINGS_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL,
    num_rounds INTEGER NOT NULL,
    start_fret INTEGER NOT NULL,
    end_fret INTEGER NOT NULL,
    FOREIGN KEY(game_id) REFERENCES games(id)
);";

impl Opts {
    fn new() -> Opts {
        Opts {
            id: None,
            game_id: None,
            num_rounds: 4,
            start_fret: 0,
            end_fret: 4,
        }
    }
}

#[derive(Debug)]
pub struct Guess {
    id: Option<i64>,
    user_id: Option<i64>,
    round_id: i64,
    clicked_fret_coord: theory::FretCoord,
    is_correct: bool,
}

const CREATE_GUESSES_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS guesses (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    round_id INTEGER NOT NULL,
    clicked_fret INTEGER NOT NULL,
    clicked_string INTEGER NOT NULL,
    is_correct INTEGER NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id),
    FOREIGN KEY(round_id) REFERENCES rounds(id)
);";

#[derive(Debug)]
pub struct Round {
    id: Option<i64>,
    note_to_guess: Note,
    guesses: Vec<Guess>,
}

const CREATE_ROUNDS_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS rounds (
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL,
    note_white_key TEXT NOT NULL,
    note_accidental TEXT NOT NULL,
    note_octave INTEGER NOT NULL,
    FOREIGN KEY(game_id) REFERENCES games(id)
);";

impl Round {
    fn new() -> Round {
        Round {
            id: None,
            note_to_guess: Note::rand_in_range(40, 68),
            guesses: vec![],
        }
    }
}

const CREATE_PLAYERS_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS players (
    game_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    UNIQUE(game_id, user_id)
);";

pub async fn ensure_games_tables(pool: &Pool<Sqlite>) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(CREATE_GAMES_TABLE_SQL)
        .execute(&mut *tx)
        .await?;

    sqlx::query(CREATE_SETTINGS_TABLE_SQL)
        .execute(&mut *tx)
        .await?;

    sqlx::query(CREATE_ROUNDS_TABLE_SQL)
        .execute(&mut *tx)
        .await?;

    sqlx::query(CREATE_GUESSES_TABLE_SQL)
        .execute(&mut *tx)
        .await?;

    sqlx::query(CREATE_PLAYERS_TABLE_SQL)
        .execute(&mut *tx)
        .await?;

    tx.commit().await
}

pub async fn insert_game(pool: &Pool<Sqlite>, game: Game) -> Result<(i64), Error> {
    let mut tx = pool.begin().await?;

    let game_id = sqlx::query("INSERT INTO games (host_id, status) VALUES (?, ?);")
        .bind(game.host_id)
        .bind(game.status.to_string())
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

    sqlx::query(
        "INSERT INTO SETTINGS (game_id, num_rounds, start_fret, end_fret) VALUES (?, ?, ?, ?);",
    )
    .bind(game_id)
    .bind(game.opts.num_rounds)
    .bind(game.opts.start_fret)
    .bind(game.opts.end_fret)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO PLAYERS (game_id, user_id) VALUES (?, ?);")
        .bind(game_id)
        .bind(game.host_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(game_id)
}

pub async fn fetch_game(pool: &Pool<Sqlite>, game_id: i64) -> Result<Game, Error> {
    let mut conn = pool.acquire().await?;

    let opts = sqlx::query("SELECT * FROM settings WHERE game_id = ?")
        .bind(game_id)
        .map(|row: SqliteRow| Opts {
            id: Some(row.get::<i64, _>("id")),
            game_id: Some(game_id),
            num_rounds: row.get::<i32, _>("num_rounds"),
            start_fret: row.get::<i32, _>("start_fret"),
            end_fret: row.get::<i32, _>("end_fret"),
        })
        .fetch_one(&mut *conn)
        .await?;

    let player_ids = sqlx::query("SELECT * FROM players WHERE game_id = ?")
        .bind(game_id)
        .map(|row: SqliteRow| row.get::<i64, _>("user_id"))
        .fetch_all(&mut *conn)
        .await?;

    let mut rounds = sqlx::query("SELECT * from rounds WHERE game_id = ?")
        .bind(game_id)
        .map(|row: SqliteRow| {
            let accidental = row
                .get::<Option<&str>, _>("note_octave")
                .map(|s| Accidental::from(s))
                .flatten();

            let note_to_guess = Note {
                white_key: WhiteKey::from(row.get::<&str, _>("note_white_key")).unwrap(),
                octave: row.get::<i32, _>("note_octave"),
                accidental,
            };

            Round {
                id: Some(row.get::<i64, _>("round_id")),
                note_to_guess,
                guesses: vec![],
            }
        })
        .fetch_all(&mut *conn)
        .await?;

    for r in rounds.iter_mut() {
        let round_id = r.id.unwrap();

        let guesses = sqlx::query("SELECT * from guesses WHERE round_id = ?")
            .bind(round_id)
            .map(|row: SqliteRow| {
                let fret = row.get::<i32, _>("clicked_fret");
                let string = row.get::<i32, _>("clicked_string");
                let clicked_fret_coord = FretCoord { fret, string };

                Guess {
                    id: Some(row.get::<i64, _>("id")),
                    user_id: Some(row.get::<i64, _>("user_id")),
                    round_id,
                    clicked_fret_coord,
                    is_correct: row.get::<bool, _>("is_correct"),
                }
            })
            .fetch_all(&mut *conn)
            .await?;

        r.guesses = guesses;
    }

    sqlx::query("SELECT * FROM games where id = ?")
        .bind(game_id)
        .fetch_one(&mut *conn)
        .await
        .map(|row: SqliteRow| Game {
            id: Some(game_id),
            host_id: Some(row.get::<i64, _>("host_id")),
            status: Status::from(row.get::<&str, _>("status")).unwrap(),
            opts,
            player_ids,
            rounds,
        })
}

// "SELECT r.id as round_id, r.note_white_key, r.note_accidental, r.note_octave,
//  g.id as guess_id, g.user_id, g.clicked_fret, g.clicked_string, g.is_correct
//  FROM rounds r
//  JOIN guesses g on r.id = g.round_id
//  WHERE r.game_id = ?")
