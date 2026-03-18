Contexte — Migration Node-RED NanoPi → Pi5
Projet : Daly-BMS-Rust — stack domotique/énergie Rust sur Raspberry Pi 5 CM.

Infrastructure actuelle :

Pi5 (192.168.1.141) user: pi5compute pw: pi5compute : daly-bms-server (Rust, systemd), Docker stack (Mosquitto :1883, InfluxDB :8086, Grafana :3001). Node-RED http://192.168.1.141:1880/ sur le Pi5.

NanoPi Neo3 (192.168.1.120) user: root pw: 12345678 : Venus OS, BusyBox v1.36.1, Linux armv7l, santuario-venus-bridge (Rust, D-Bus), Node-RED :1881 avec les flows existants, Mosquitto local.
VRM instance: c0619ab9929a

https://github.com/thieryus007-cloud/Daly-BMS-Rust/blob/main/flux-nodered/

Objectif de la session : Migrer les flows Node-RED du NanoPi vers le Pi5.

Actions à réaliser :

Exporter les flows depuis le NanoPi (http://192.168.1.120:1881)
Vérifier que Node-RED est bien dans docker-compose.infra.yml sur le Pi5
Importer les flows sur le Pi5 (http://192.168.1.100:1880)
Adapter les références broker MQTT (127.0.0.1 → 192.168.1.141 ou nom container dalybms-mosquitto)
Valider les flows, puis arrêter Node-RED sur le NanoPi
Repo : branche claude/venus-dbus-rust-service-JIthK
MQTT topic prefix : santuario/bms/{n}/...

liste des flux par ordre de migration.

1 - meteo
2 - testing
3 - previsions
4 - waterheater
5 - setwaterheater
6 - deye
7 - shelly
8 - shelly2
9 - feedinpower    attention: flux Plus important 
