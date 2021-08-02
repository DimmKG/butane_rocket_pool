# butane_rocket_pool
[Butane](https://github.com/Electron100/butane) database adapter for Rocket framework
# Usage

1. Configure your database in `Rocket.toml`. Parameters `url` and `backend_name` are required.
 ```toml
 [default.databases.test]
 backend_name = "sqlite" #Butane's backend name
 url = "test.db"
 ```

2. Add and init database in your application's code
```rust
#[database("test")]
struct DbConn(butane_rocket_pool::Connection); 

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![create])
    .attach(DbConn::fairing())
}
```
 3. To use the connection with Butane functions apply two dereference operators.

```rust
#[post("/", data = "<post>")]
async fn create(db: DbConn, post: Json<Post>) -> (Status, Value) {
    let result = db.run(move |db| -> Result<Post, butane::Error> {
        let mut result = post.0;
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
```

