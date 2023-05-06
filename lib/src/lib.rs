//! Butane database adapter for Rocket framework
//! 
//! # Usage
//! 
//!   1. Configure your database in `Rocket.toml`. Parameters `url` and `backend_name` are required.
//! ```toml
//! [default.databases.test]
//! backend_name = "sqlite" #Butane's backend name
//! url = "test.db"
//! ```
//! 
//!   2. Add and init database in your application's code
//! ```
//! #[database("test")]
//! struct DbConn(butane_rocket_pool::Connection); 
//! 
//! #[launch]
//! fn rocket() -> _ {
//!     rocket::build().mount("/", routes![create])
//!     .attach(DbConn::fairing())
//! }
//! ```
//! 
//!   3. To use the connection with Butane functions apply two dereference operators.
//! 
//! ```
//! #[post("/", data = "<post>")]
//! async fn create(db: DbConn, post: Json<Post>) -> (Status, Value) {
//!     let result = db.run(move |db| -> Result<Post, butane::Error> {
//!         let mut result = post.0;
//!         result.save(&**db)?;
//!         Post::get(&**db, result.id)
//!     }).await;
//! 
//!     match result {
//!         Ok(res) => (Status::Created, json!({
//!             "message" : "Post is created!",
//!             "data" : res
//!         })),
//!         Err(err) => (Status::InternalServerError, json!({
//!             "message" : "Can't create post!",
//!             "error" : format!("{}", err)
//!         }))
//!     }
//! }
//! ```
//! 

use rocket_sync_db_pools::rocket::{Rocket, Build, Config};
use rocket_sync_db_pools::rocket::figment::{self,Figment, providers::Serialized};
use rocket_sync_db_pools::{Poolable, PoolResult};
use butane::db::ConnectionSpec;
use r2d2::ManageConnection;
use serde::Deserialize;
use std::ops::Deref;

/// R2D2 connection which supports Rocket.
/// 
/// To use it with Butane functions apply two dereference operators to the connection.
/// 
/// Example:
/// ```
/// #[database("test")]
/// struct Db(butane_rocket_pool::Connection); 
/// 
/// #[post("/", data = "<post>")]
/// async fn create(db: Db, post: Json<Post>) -> (Status, Value) {
///     let result = db.run(move |db| -> Result<Post, butane::Error> {
///         let mut result = post.0;
///         result.save(&**db)?;
///         Post::get(&**db, result.id)
///     }).await;
///
///     match result {
///         Ok(res) => (Status::Created, json!({
///             "message" : "Post is created!",
///             "data" : res
///         })),
///         Err(err) => (Status::InternalServerError, json!({
///             "message" : "Can't create post!",
///             "error" : format!("{}", err)
///         }))
///     }
/// }
/// ```
pub struct Connection(butane::db::Connection);

impl Deref for Connection {
    type Target = butane::db::Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// R2D2 connection manager which supports Rocket.
/// Not expected to be used directly by most users.
pub struct ConnectionManager(butane::db::ConnectionManager);

impl ConnectionManager {
    pub fn new(spec: ConnectionSpec) -> Self {
        ConnectionManager (butane::db::ConnectionManager::new(spec))
    }
}

#[derive(Deserialize)]
struct DBConfig {
    pub backend_name: String,
    pub url: String,
    pub pool_size: u32,
}

impl DBConfig {
    pub fn from(db_name: &str, rocket: &Rocket<Build>) -> Result<DBConfig, figment::Error> {
        DBConfig::figment(db_name, rocket).extract::<Self>()
    }

    fn figment(db_name: &str, rocket: &Rocket<Build>) -> Figment {
        let db_key = format!("databases.{}", db_name);
        let default_pool_size = rocket.figment()
            .extract_inner::<u32>(Config::WORKERS)
            .map(|workers| workers * 4)
            .ok();

        let figment = Figment::from(rocket.figment())
            .focus(&db_key)
            .join(Serialized::default("timeout", 5));

        match default_pool_size {
            Some(pool_size) => figment.join(Serialized::default("pool_size", pool_size)),
            None => figment
        }
    }
}


impl ManageConnection for ConnectionManager {
    type Connection = Connection;
    type Error = butane::Error;

    fn connect(&self) -> Result<Self::Connection, butane::Error> {
        match self.0.connect() {
            Ok(res) => Ok(Connection(res)),
            Err(err) => Err(err)
        }
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), butane::Error> {
        self.0.is_valid(&mut conn.0)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.0.has_broken(&mut conn.0)
    }

}


impl Poolable for Connection {
    type Manager = ConnectionManager;
    type Error = butane::Error;

    fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
        let config = DBConfig::from(db_name, rocket)?;
        let specs = ConnectionSpec{
            backend_name: config.backend_name,
            conn_str: config.url
            
        };
        let manager = ConnectionManager::new(specs);
        Ok(r2d2::Pool::builder().max_size(config.pool_size).build(manager)?)
    }
}
