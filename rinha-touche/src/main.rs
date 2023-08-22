use std::{env, io, net::SocketAddr, sync::Arc};

use persistence::PersistenceError;
use rinha_core::NewPerson;
use serde::Deserialize;
use touche::{Body, HttpBody, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

use crate::persistence::PostgresRepository;

mod persistence;

#[derive(Deserialize)]
struct PersonSearchQuery {
    #[serde(rename = "t")]
    query: String,
}

fn main() -> io::Result<()> {
    let port = env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(9999);

    let database_url = env::var("DATABASE_URL")
        .unwrap_or(String::from("postgres://rinha:rinha@localhost:5432/rinha"));

    let database_pool_size = env::var("DATABASE_POOL")
        .ok()
        .and_then(|port| port.parse::<usize>().ok())
        .unwrap_or(50);

    let max_threads = env::var("MAX_THREADS")
        .ok()
        .and_then(|port| port.parse::<usize>().ok())
        .unwrap_or(400);

    let repo = PostgresRepository::connect(&database_url, database_pool_size).unwrap();
    let repo = Arc::new(repo);

    Server::builder()
        .max_threads(max_threads)
        .bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .serve(move |req: Request<Body>| {
            let repo = repo.clone();
            let segments = req.uri().path().split('/').skip(1).collect::<Vec<_>>();

            match (req.method(), segments.as_slice()) {
                (&Method::GET, ["pessoas"]) => {
                    let query = req.uri().query().unwrap_or_default();
                    match serde_urlencoded::from_str::<PersonSearchQuery>(query) {
                        Ok(PersonSearchQuery { query }) => match repo.search_people(&query) {
                            Ok(people) => {
                                let people = serde_json::to_vec(&people).unwrap();
                                Response::builder()
                                    .status(StatusCode::OK)
                                    .header("content-type", "application/json")
                                    .body(Body::from(people))
                            }
                            Err(_) => Response::builder()
                                .status(StatusCode::UNPROCESSABLE_ENTITY)
                                .body(Body::empty()),
                        },
                        Err(_) => Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::empty()),
                    }
                }

                (&Method::POST, ["pessoas"]) => {
                    let body = req.into_body();
                    match serde_json::from_reader::<_, NewPerson>(body.into_reader()) {
                        Ok(person) => match repo.create_person(person) {
                            Ok(id) => Response::builder()
                                .status(StatusCode::CREATED)
                                .header("location", format!("/pessoas/{id}"))
                                .body(Body::empty()),
                            Err(PersistenceError::UniqueViolation) => Response::builder()
                                .status(StatusCode::UNPROCESSABLE_ENTITY)
                                .body(Body::empty()),
                            Err(_) => Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::empty()),
                        },
                        Err(_) => Response::builder()
                            .status(StatusCode::UNPROCESSABLE_ENTITY)
                            .body(Body::empty()),
                    }
                }

                (&Method::GET, ["pessoas", id]) => match Uuid::parse_str(id) {
                    Ok(id) => match repo.find_person(id) {
                        Ok(Some(person)) => {
                            let person = serde_json::to_vec(&person).unwrap();
                            Response::builder()
                                .status(StatusCode::OK)
                                .header("content-type", "application/json")
                                .body(Body::from(person))
                        }
                        Ok(None) => Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::empty()),
                        Err(_) => Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty()),
                    },
                    Err(_) => Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::empty()),
                },

                (&Method::GET, ["contagem-pessoas"]) => match repo.count_people() {
                    Ok(count) => Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(count.to_string())),
                    Err(_) => Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty()),
                },

                _ => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty()),
            }
        })
}
