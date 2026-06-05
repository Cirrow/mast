use rusqlite::{Connection, params};

pub fn init_db(path: &str) -> Connection {
    let conn = Connection::open(path).expect("failed to open auth db");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS users (
            username    INTEGER PRIMARY KEY AUTOINCREMENT,
            github_id   INTEGER UNIQUE NOT NULL,
            login       TEXT NOT NULL,
            email       TEXT,
            role        TEXT,
            created_at  TEXT DEFAULT (datetime('now'))
        );"
    ).expect("failed to create users table");
    conn
}


pub fn upsert_user(conn: &Connection, github_id: i64, login: &str, email: Option<&str>) -> i64 {
    conn.execute(
        "INSERT INTO users (github_id, login, email)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(github_id) DO UPDATE SET
            login = excluded.login,
            email = excluded.email",
        params![github_id, login, email],
    ).expect("failed to upsert user");

    conn.query_row(
        "SELECT id FROM users WHERE github_id = ?1",
        params![github_id],
        |row| row.get(0),
    ).expect("failed to get user id")
}

pub fn get_user(conn: &Connection, user_id: i64) -> Option<(i64, i64, String, Option<String>, Option<String>)> {
    conn.query_row(
        "SELECT id, github_id, login, email FROM users WHERE id = ?1",
        params![user_id],
        |row| Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        )),
    ).ok()
}