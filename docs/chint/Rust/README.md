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

Changement souhaité

Dans la box Source I: Remplacer source I par Onduleur et le point bleu doit être Vert quand c est la source Onduleur qui est en service. de plus a cote du mot onduleur ajouter connecté si c est onduleur connecté si non déconnecté, cela reflète les informations de commutateurs 1/0

Dans la box Source II: Remplacer source II par Réseau et le point actuellement rouge doit refléter si Réseau est connectée ou déconnecté soit vert/rouge doit être Vert quand c est la source Réseau qui est en service ou rouge quand reconnecté. de plus a cote du mot Réseau a jouter connecte si c est réseau connecte si non déconnecté, cela reflète les informations de commutateurs 1/0
