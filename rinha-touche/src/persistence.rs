use std::{error::Error, fmt::Display, str::FromStr, sync::Arc, thread};

use dashmap::{DashMap, DashSet};
use postgres::{
    error::SqlState, fallible_iterator::FallibleIterator, Config as PgConfig, Error as PgError,
    NoTls, Row,
};
use r2d2::{Error as PoolError, Pool};
use r2d2_postgres::PostgresConnectionManager;
use rinha_core::{NewPerson, Nick, Person, PersonName};
use time::Date;
use uuid::Uuid;

struct PersistedPerson {
    id: Uuid,
    name: PersonName,
    nick: Nick,
    birth_date: Date,
    stack: Option<Vec<String>>,
}

impl TryFrom<Row> for PersistedPerson {
    type Error = Box<dyn Error + Send + Sync>;

    fn try_from(value: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.try_get("id")?,
            name: value.try_get::<_, String>("name")?.try_into()?,
            nick: value.try_get::<_, String>("nick")?.try_into()?,
            birth_date: value.try_get("birth_date")?,
            stack: value.try_get("stack")?,
        })
    }
}

impl From<PersistedPerson> for Person {
    fn from(person: PersistedPerson) -> Self {
        Self {
            id: person.id,
            name: person.name,
            nick: person.nick,
            birth_date: person.birth_date,
            stack: person.stack,
        }
    }
}

pub type PersistenceResult<T> = Result<T, PersistenceError>;

#[derive(Debug)]
pub enum PersistenceError {
    UniqueViolation,
    DatabaseError(Box<dyn Error + Send + Sync>),
}

impl Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UniqueViolation => write!(f, "unique constraint violated"),
            Self::DatabaseError(err) => write!(f, "{}", err),
        }
    }
}

impl From<PgError> for PersistenceError {
    fn from(value: PgError) -> Self {
        match value.code() {
            Some(&SqlState::UNIQUE_VIOLATION) => PersistenceError::UniqueViolation,
            _ => PersistenceError::DatabaseError(Box::new(value)),
        }
    }
}

impl From<PoolError> for PersistenceError {
    fn from(value: PoolError) -> Self {
        PersistenceError::DatabaseError(Box::new(value))
    }
}

pub struct PostgresRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
    cache: Arc<DashMap<Uuid, Person>>,
    nicks: Arc<DashSet<String>>,
}

impl PostgresRepository {
    pub fn connect(url: &str, pool_size: usize) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = r2d2::Pool::builder()
            .max_size(pool_size.try_into()?)
            .build(PostgresConnectionManager::new(
                PgConfig::from_str(url).unwrap(),
                NoTls,
            ))
            .unwrap();

        let cache = Arc::new(DashMap::new());
        let nicks = Arc::new(DashSet::new());

        thread::spawn({
            let mut conn = pool.get()?;
            let cache = cache.clone();
            let nicks = nicks.clone();
            move || {
                conn.execute("LISTEN person_created", &[])?;
                let mut notifications = conn.notifications();
                notifications.blocking_iter().for_each(|msg| {
                    if let Ok(person) = serde_json::from_str::<Person>(msg.payload()) {
                        nicks.insert(person.nick.as_str().to_owned());
                        cache.insert(person.id, person);
                    }
                    Ok(())
                })?;
                Ok::<_, Box<dyn Error + Send + Sync>>(())
            }
        });

        Ok(Self { pool, cache, nicks })
    }

    pub fn create_person(&self, person: NewPerson) -> PersistenceResult<Uuid> {
        if self.nicks.contains(person.nick.as_str()) {
            return Err(PersistenceError::UniqueViolation);
        }

        let mut conn = self.pool.get()?;

        let stmt = conn.prepare(
            "
            INSERT INTO
            people (id, name, nick, birth_date, stack)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            ",
        )?;

        let result = conn.query_one(
            &stmt,
            &[
                &Uuid::now_v7(),
                &String::from(person.name),
                &String::from(person.nick),
                &person.birth_date,
                &person
                    .stack
                    .map(|stack| stack.into_iter().map(String::from).collect::<Vec<_>>()),
            ],
        )?;

        Ok(result.try_get(0)?)
    }

    pub fn find_person(&self, id: Uuid) -> PersistenceResult<Option<Person>> {
        if let Some(person) = self.cache.get(&id).map(|entry| entry.value().clone()) {
            return Ok(Some(person));
        }

        let mut conn = self.pool.get()?;

        let stmt = conn.prepare(
            "
            SELECT id, name, nick, birth_date, stack
            FROM people
            WHERE id = $1
            ",
        )?;

        match conn.query_opt(&stmt, &[&id])? {
            Some(row) => Ok(Some(
                PersistedPerson::try_from(row)
                    .map(Person::from)
                    .map_err(PersistenceError::DatabaseError)?,
            )),
            None => Ok(None),
        }
    }

    pub fn search_people(&self, query: &str) -> PersistenceResult<Vec<Person>> {
        let mut conn = self.pool.get()?;

        let stmt = conn.prepare(
            "
            SELECT id, name, nick, birth_date, stack
            FROM people
            WHERE search ILIKE $1
            LIMIT 50
            ",
        )?;

        conn.query(&stmt, &[&format!("%{query}%")])?
            .into_iter()
            .map(|person| {
                PersistedPerson::try_from(person)
                    .map(Person::from)
                    .map_err(PersistenceError::DatabaseError)
            })
            .collect()
    }

    pub fn count_people(&self) -> PersistenceResult<u64> {
        let mut conn = self.pool.get()?;
        let stmt = conn.prepare("SELECT COUNT(*) FROM people")?;
        let row = conn.query_one(&stmt, &[])?;
        let count: i64 = row.try_get(0)?;
        Ok(count.unsigned_abs())
    }
}
