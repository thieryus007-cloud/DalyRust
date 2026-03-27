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


warning: unused import: `std::io::Write`
  --> src\main.rs:10:5
   |
10 | use std::io::Write as IoWrite;
   |     ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `SerialPort`
 --> src\main.rs:6:24
  |
6 | use serialport::{self, SerialPort};
  |                        ^^^^^^^^^^

warning: unreachable pattern
   --> src\main.rs:136:9
    |
136 |         _ => None,
    |         ^ no value can reach this
    |
note: multiple earlier patterns match some of the same values
   --> src\main.rs:136:9
    |
128 |         Ok(n) => {
    |         ----- matches some of the same values
...
132 |         Err(e) => {
    |         ------ matches some of the same values
...
136 |         _ => None,
    |         ^ collectively making this unreachable
    = note: `#[warn(unreachable_patterns)]` (part of `#[warn(unused)]`) on by default

warning: `chint_ats` (bin "chint_ats") generated 3 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.36s

















