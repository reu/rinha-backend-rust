use std::{error::Error, sync::Arc};

use dashmap::{DashMap, DashSet};
use rinha_core::{NewPerson, Person};
use sqlx::{
    postgres::{PgListener, PgPoolOptions},
    PgPool,
};
use uuid::Uuid;

pub enum PersistenceError {
    UniqueViolation,
    DatabaseError(Box<dyn Error + Send + Sync>),
}

impl From<sqlx::Error> for PersistenceError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(err) if err.is_unique_violation() => {
                PersistenceError::UniqueViolation
            }
            _ => PersistenceError::DatabaseError(Box::new(error)),
        }
    }
}

type PersistenceResult<T> = Result<T, PersistenceError>;

pub struct PostgresRepository {
    pool: PgPool,
    cache: Arc<DashMap<Uuid, Person>>,
    nicks: Arc<DashSet<String>>,
}

impl PostgresRepository {
    pub async fn connect(url: &str, pool_size: u32) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(pool_size)
            .connect(url)
            .await?;

        let cache = Arc::new(DashMap::with_capacity(30_000));
        let nicks = Arc::new(DashSet::new());

        tokio::spawn({
            let pool = pool.clone();
            let cache = cache.clone();
            let nicks = nicks.clone();
            async move {
                if let Ok(mut listener) = PgListener::connect_with(&pool).await {
                    listener.listen("person_created").await.ok();
                    while let Ok(msg) = listener.recv().await {
                        if let Ok(person) = serde_json::from_str::<Person>(msg.payload()) {
                            nicks.insert(person.nick.as_str().to_owned());
                            cache.insert(person.id, person);
                        }
                    }
                }
            }
        });

        Ok(PostgresRepository { pool, cache, nicks })
    }

    pub async fn find_person(&self, id: Uuid) -> PersistenceResult<Option<Person>> {
        if let Some(person) = self.cache.get(&id).map(|entry| entry.value().clone()) {
            return Ok(Some(person));
        }

        sqlx::query_as(
            "
            SELECT id, name, nick, birth_date, stack
            FROM people
            WHERE id = $1
            ",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(PersistenceError::from)
    }

    pub async fn create_person(&self, new_person: NewPerson) -> PersistenceResult<Uuid> {
        if self.nicks.contains(new_person.nick.as_str()) {
            return Err(PersistenceError::UniqueViolation);
        }

        let stack = new_person
            .stack
            .map(|stack| stack.into_iter().map(String::from).collect::<Vec<_>>());

        sqlx::query!(
            "
            INSERT INTO people (id, name, nick, birth_date, stack)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            ",
            Uuid::now_v7(),
            new_person.name.as_str(),
            new_person.nick.as_str(),
            new_person.birth_date,
            stack.as_ref().map(|stack| stack.as_slice()),
        )
        .fetch_one(&self.pool)
        .await
        .map(|row| row.id)
        .map_err(PersistenceError::from)
    }

    pub async fn search_people(&self, query: &str) -> PersistenceResult<Vec<Person>> {
        sqlx::query_as(
            "
            SELECT id, name, nick, birth_date, stack
            FROM people
            WHERE search ILIKE $1
            LIMIT 50
            ",
        )
        .bind(format!("%{query}%"))
        .fetch_all(&self.pool)
        .await
        .map_err(PersistenceError::from)
    }

    pub async fn count_people(&self) -> PersistenceResult<u64> {
        sqlx::query!("SELECT COUNT(*) AS count FROM people")
            .fetch_one(&self.pool)
            .await
            .map(|row| row.count.unwrap_or_default())
            .map(|count| count.unsigned_abs())
            .map_err(PersistenceError::from)
    }
}
