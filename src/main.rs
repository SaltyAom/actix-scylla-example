use std::error::Error;
use serde::Deserialize;

use actix_web::{ HttpServer, App, Responder, web, get, put };
use scylla::{
    Session, 
    SessionBuilder, 
    IntoTypedRows, 
    prepared_statement::PreparedStatement
};

#[derive(Deserialize)]
struct User {
    username: String,
    password: String
}

struct AppUtils {
    scylla: Session,
    query: Query
}

struct Query {
    get_user: PreparedStatement,
    put_user: PreparedStatement
}

impl Query {
    async fn new(session: &Session) -> Query {
        Query {
            get_user: session.prepare("SELECT username, password FROM ks.salt1 WHERE username = ?").await.unwrap(),
            put_user: session.prepare("INSERT INTO ks.salt1 (username, password) VALUES (?, ?)").await.unwrap()
        }
    }
}

async fn setup(session: &Session) -> Result<Query, Box<dyn Error>> {
    session
        .query(
            "CREATE KEYSPACE IF NOT EXISTS ks WITH REPLICATION = \
            {'class' : 'SimpleStrategy', 'replication_factor' : 1}",
            &[],
        )
        .await?;

    session
        .query(
            "CREATE TABLE IF NOT EXISTS ks.salt1 (username ASCII primary key, password ASCII)",
            &[],
        )
        .await?;

    Ok(Query::new(&session).await)
}

#[get("/")]
async fn hi() -> impl Responder {
    "Hi"
}

#[get("/user/{user}")]
async fn post_user(
    data: web::Data<AppUtils>, 
    path: web::Path<(String,)>
) -> Result<impl Responder, Box<dyn Error>> {
    let (username,) = path.into_inner();

    match data.scylla.execute(
        &data.query.get_user,
        (username,)
    ).await?.rows {
        Some(rows) => {
            // ? We know that the query will return only one row
            match rows.into_typed::<(String, String)>().next() {
                Some(row) => {
                    let (username, password) = row?;
    
                    Ok(username + " " + &password)    
                },
                None => Ok("Can't find user".to_owned())
            }
        },
        None => Ok("Something went wrong".to_owned())
    }
}

#[put("/user")]
async fn put_user(
    data: web::Data<AppUtils>, 
    body: web::Json<User>
) -> Result<impl Responder, Box<dyn Error>> {
    match data.scylla.execute(
        &data.query.put_user,
        (body.username.to_owned(), body.password.to_owned())
    ).await {
        Ok(_) => Ok("User Added"),
        Err(_) => Ok("Something went wrong")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let uri = std::env::var("scylla_uri").unwrap_or("127.0.0.1:9042".to_owned());

    let scylla = SessionBuilder::new()
        .known_node(uri)
        .compression(Some(scylla::transport::Compression::Lz4))
        .build()
        .await
        .map_err(|_| {
            panic!("Unable to connect to Scylla");
        })
        .unwrap();

    let query = setup(&scylla)
        .await
        .map_err(|_| {
            panic!("Unable to setup Scylla DB");
        })
        .unwrap();

    let utils = web::Data::new(AppUtils {
        scylla,
        query
    });

    HttpServer::new(move || 
        App::new()
            .app_data(utils.clone())
            .service(hi)
            .service(post_user)
            .service(put_user)
    )
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
