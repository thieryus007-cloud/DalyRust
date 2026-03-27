# Install Rust

curl https://sh.rustup.rs -sSf | sh

cd C:\

cargo new chint_ats

cd chint_ats


C:\chint_ats\          ← Dossier du projet
│

├── Cargo.toml                      ← Fichier de configuration

│

├── src\

│   └── main.rs                     ← code Rust

│

└── index.html                      ← Fichier HTML

http://localhost:5000


[package]
name = "chint_ats"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4"
actix-files = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serialport = "4"

..............................................................................................................

C:\Users\thier\Downloads\Rust-ATS\chint_ats>cargo run
   Compiling chint_ats v0.1.0 (C:\Users\thier\Downloads\Rust-ATS\chint_ats)
error[E0425]: cannot find value `set_undervoltage2` in this scope
   --> src\main.rs:518:64
    |
427 | async fn set_undervoltage1(data: web::Data<Mutex<AppState>>, query: web::Query<RegValue>) -> impl Responder {
    | ----------------------------------------------------------------------------------------------------------- similarly named function `set_undervoltage1` defined here
...
518 |                 .route("/api/set_undervoltage2", web::get().to(set_undervoltage2))
    |                                                                ^^^^^^^^^^^^^^^^^
    |
help: a function with a similar name exists
    |
518 -                 .route("/api/set_undervoltage2", web::get().to(set_undervoltage2))
518 +                 .route("/api/set_undervoltage2", web::get().to(set_undervoltage1))
    |

error[E0425]: cannot find value `set_overvoltage1` in this scope
   --> src\main.rs:519:63
    |
427 | async fn set_undervoltage1(data: web::Data<Mutex<AppState>>, query: web::Query<RegValue>) -> impl Responder {
    | ----------------------------------------------------------------------------------------------------------- similarly named function `set_undervoltage1` defined here
...
519 |                 .route("/api/set_overvoltage1", web::get().to(set_overvoltage1))
    |                                                               ^^^^^^^^^^^^^^^^
    |
help: a function with a similar name exists
    |
519 -                 .route("/api/set_overvoltage1", web::get().to(set_overvoltage1))
519 +                 .route("/api/set_overvoltage1", web::get().to(set_undervoltage1))
    |

error[E0425]: cannot find value `set_overvoltage2` in this scope
   --> src\main.rs:520:63
    |
427 | async fn set_undervoltage1(data: web::Data<Mutex<AppState>>, query: web::Query<RegValue>) -> impl Responder {
    | ----------------------------------------------------------------------------------------------------------- similarly named function `set_undervoltage1` defined here
...
520 |                 .route("/api/set_overvoltage2", web::get().to(set_overvoltage2));
    |                                                               ^^^^^^^^^^^^^^^^
    |
help: a function with a similar name exists
    |
520 -                 .route("/api/set_overvoltage2", web::get().to(set_overvoltage2));
520 +                 .route("/api/set_overvoltage2", web::get().to(set_undervoltage1));
    |

warning: unused variable: `debug`
   --> src\main.rs:181:44
    |
181 | fn detect_model(port_name: &str, addr: u8, debug: bool) -> String {
    |                                            ^^^^^ help: if this is intentional, prefix it with an underscore: `_debug`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

For more information about this error, try `rustc --explain E0425`.
warning: `chint_ats` (bin "chint_ats") generated 1 warning
error: could not compile `chint_ats` (bin "chint_ats") due to 3 previous errors; 1 warning emitted

C:\Users\thier\Downloads\Rust-ATS\chint_ats>
