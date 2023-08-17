use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::{NewPerson, Person};

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub async fn connect(url: &str, pool_size: u32) -> Result<Self, sqlx::Error> {
        Ok(PostgresRepository {
            pool: PgPoolOptions::new()
                .max_connections(pool_size)
                .connect(url)
                .await?,
        })
    }

    pub async fn find_person(&self, id: Uuid) -> Result<Option<Person>, sqlx::Error> {
        sqlx::query_as!(
            Person,
            "
            SELECT id, name, nick, birth_date, stack
            FROM people
            WHERE id = $1
            ",
            id,
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_person(&self, new_person: NewPerson) -> Result<Uuid, sqlx::Error> {
        let stack = new_person
            .stack
            .map(|stack| stack.into_iter().map(String::from).collect::<Vec<String>>());

        sqlx::query!(
            "
            INSERT INTO people (id, name, nick, birth_date, stack)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            ",
            Uuid::now_v7(),
            new_person.name.0,
            new_person.nick.0,
            new_person.birth_date,
            stack.as_ref().map(|stack| stack.as_slice()),
        )
        .fetch_one(&self.pool)
        .await
        .map(|row| row.id)
    }

    pub async fn search_people(&self, query: String) -> Result<Vec<Person>, sqlx::Error> {
        sqlx::query_as!(
            Person,
            "
            SELECT id, name, nick, birth_date, stack
            FROM people
            WHERE search ILIKE $1
            LIMIT 50
            ",
            format!("%{query}%"),
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn count_people(&self) -> Result<u64, sqlx::Error> {
        sqlx::query!("SELECT COUNT(*) AS count FROM people")
            .fetch_one(&self.pool)
            .await
            .map(|row| row.count.unwrap_or_default())
            .map(|count| count.unsigned_abs())
    }
}
