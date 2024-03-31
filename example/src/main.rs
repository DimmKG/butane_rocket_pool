#[macro_use] extern crate rocket;
use butane::ObjectState;
use butane::model;
use butane::prelude::*;
use rocket::http::Status;
use rocket::serde::json::{Value, json};
use rocket_sync_db_pools::database;
use rocket::serde::{json::Json};
use serde::Deserialize;
use serde::Serialize;
use butane_rocket_pool::Connection;

#[model]
#[derive(Default, Serialize, Deserialize)]
struct Post {
    #[auto]
    id: i64,
    title: String,
    body: String,
    published: bool,
    likes: i32,
}

impl Post {
    pub fn new(data: CreatePost) -> Self {
        Post{
            id: -1,
            title: data.title,
            body: data.body,
            published: data.published,
            likes: 0,
            state: ObjectState::default(),
        }
    }
}

#[derive(Deserialize)]
struct CreatePost {
    title: String,
    body: String,
    #[serde(default)]
    published: bool,
}


#[database("test")]
struct DbConn(Connection); 

#[post("/", data = "<post>")]
async fn create(db: DbConn, post: Json<CreatePost>) -> (Status, Value) {
    let result = db.run(move |db| -> Result<Post, butane::Error> {
        let mut result = Post::new(post.0);
        match result.save(&**db) {
            Ok(_) => (),
            Err(err) => return Err(err)
        };
        Post::get(&**db, result.id)
    }).await;

    match result {
        Ok(res) => (Status::Created, json!({
            "message" : "Post is created!",
            "data" : res
        })),
        Err(err) => (Status::InternalServerError, json!({
            "message" : "Can't create post!",
            "error" : format!("{}", err)
        }))
    }
}

#[get("/")]
async fn index(db: DbConn) -> (Status, Value) {
    let result = db.run(move |db| -> Result<Vec<Post>, butane::Error> {
        let result = Post::query().load(&**db);
        match result {
            Ok(posts) => Ok(posts),
            Err(err) => return Err(err)
        }
    }).await;

    match result {
        Ok(res) => (Status::Ok, json!({
            "data" : res
        })),
        Err(err) => (Status::InternalServerError, json!({
            "message" : "Can't get posts!",
            "error" : format!("{}", err)
        }))
    }
}


#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, create])
    .attach(DbConn::fairing())
}
