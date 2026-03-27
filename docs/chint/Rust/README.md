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
