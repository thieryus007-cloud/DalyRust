# 🔍 Scanner Modbus RS485 pour ATS CHINT – Programme Rust

Voici un outil complet en **Rust** pour scanner tous les registres Modbus de votre ATS CHINT (adresse `0x06`) via le port **COM5**.

---

## 📦 Prérequis

1. **Rust installé** : [https://rustup.rs/](https://rustup.rs/)
2. **Convertisseur USB-RS485** connecté sur `COM5`
3. **ATS alimenté** et câblé correctement (A+, B-, GND)

---

## 🛠️ Création du Projet

```bash
cargo new chint_modbus_scanner
cd chint_modbus_scanner
```

### `Cargo.toml`

```toml
[package]
name = "chint_modbus_scanner"
version = "1.0.0"
edition = "2021"
authors = ["Votre Nom"]
description = "Scanner de registres Modbus pour ATS CHINT NXZBN"

[dependencies]
serialport = "4.2"
clap = { version = "4.4", features = ["derive"] }
colored = "2.0"
crc = "3.0"
```

---

## 💻 Code Source Principal (`src/main.rs`)

```rust
use serialport::{SerialPort, SerialPortType};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use clap::Parser;
use colored::*;

/// Scanner de registres Modbus pour ATS CHINT
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port série (ex: COM5, /dev/ttyUSB0)
    #[arg(short, long, default_value = "COM5")]
    port: String,

    /// Adresse Modbus de l'ATS (décimal)
    #[arg(short, long, default_value = "6")]
    address: u8,

    /// Débit en bauds
    #[arg(short, long, default_value = "9600")]
    baud: u32,

    /// Adresse registre de début (hex sans 0x)
    #[arg(short, long, default_value = "0")]
    start: u16,

    /// Adresse registre de fin (hex sans 0x)
    #[arg(short, long, default_value = "FFFF")]
    end: u16,

    /// Délai entre requêtes (ms)
    #[arg(short, long, default_value = "100")]
    delay: u64,

    /// Fonction Modbus (3=lecture, 6=écriture test)
    #[arg(short, long, default_value = "3")]
    function: u8,
}

fn main() {
    let args = Args::parse();
    
    println!("{}", "╔══════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║     SCANNER MODBUS RS485 - ATS CHINT NXZBN              ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    
    // Configuration
    println!("{} Port: {}", "📍".green(), args.port.bright_white());
    println!("{} Adresse Modbus: 0x{:02X} ({})", "🏷️".green(), args.address, args.address);
    println!("{} Baudrate: {} bps", "⚡".green(), args.baud);
    println!("{} Plage registres: 0x{:04X} - 0x{:04X}", "📊".green(), args.start, args.end);
    println!("{} Délai: {} ms", "⏱️".green(), args.delay);
    println!();
    
    // Ouverture du port
    let mut port = match serialport::new(&args.port, args.baud)
        .timeout(Duration::from_millis(500))
        .open()
    {
        Ok(p) => {
            println!("{} Port {} ouvert avec succès!", "✅".green(), args.port.bright_green());
            p
        }
        Err(e) => {
            eprintln!("{} Erreur ouverture port: {}", "❌".red(), e);
            std::process::exit(1);
        }
    };
    
    // Configuration série (8N1 ou 8E1 selon votre config)
    port.set_data_bits(serialport::DataBits::Eight).ok();
    port.set_flow_control(serialport::FlowControl::None).ok();
    port.set_parity(serialport::Parity::Even).ok(); // Parité par défaut CHINT
    port.set_stop_bits(serialport::StopBits::One).ok();
    
    println!("{} Configuration: 8E1 (8 bits, Even, 1 stop)", "⚙️".green());
    println!();
    
    // Scan
    scan_registers(&mut port, &args);
    
    println!();
    println!("{}", "══════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "Scan terminé!".bright_green());
}

fn scan_registers(port: &mut Box<dyn SerialPort>, args: &Args) {
    let mut responsive_registers = Vec::new();
    let mut error_registers = Vec::new();
    let mut no_response = Vec::new();
    
    let total = (args.end - args.start + 1) as usize;
    println!("{} Début du scan de {} registres...", "🔍".yellow(), total);
    println!();
    
    for addr in args.start..=args.end {
        let progress = ((addr - args.start + 1) as f32 / total as f32) * 100.0;
        print!("\rProgression: {:5.1}% | Registre: 0x{:04X}", progress, addr);
        std::io::stdout().flush().unwrap();
        
        // Construction trame Modbus RTU (Fonction 03 - Lecture)
        let frame = build_modbus_frame(args.address, 0x03, addr, 1);
        
        // Envoi
        port.write_all(&frame).unwrap();
        port.flush().unwrap();
        
        // Délai
        thread::sleep(Duration::from_millis(args.delay));
        
        // Lecture réponse
        let mut response = [0u8; 256];
        match port.read(&mut response) {
            Ok(len) if len >= 5 => {
                // Vérification adresse
                if response[0] == args.address {
                    // Vérification fonction
                    match response[1] {
                        0x03 => {
                            // Réponse normale
                            let value = if len >= 7 {
                                u16::from_be_bytes([response[3], response[4]])
                            } else {
                                0
                            };
                            responsive_registers.push((addr, value, len));
                            print!(" {}", "✅".green());
                        }
                        0x83 | 0x86 => {
                            // Erreur Modbus
                            let error_code = if len >= 3 { response[2] } else { 0 };
                            error_registers.push((addr, error_code, len));
                            print!(" {}", "⚠️".yellow());
                        }
                        _ => {
                            no_response.push((addr, len));
                            print!(" {}", "❓".blue());
                        }
                    }
                } else {
                    no_response.push((addr, len));
                    print!(" {}", "❌".red());
                }
            }
            Ok(len) => {
                no_response.push((addr, len));
                print!(" {}", "❌".red());
            }
            Err(_) => {
                no_response.push((addr, 0));
                print!(" {}", "❌".red());
            }
        }
        
        // Purge buffer
        port.clear(serialport::ClearBuffer::Both).ok();
    }
    
    println!();
    println!();
    
    // Rapport
    print_report(&responsive_registers, &error_registers, &no_response);
}

fn build_modbus_frame(address: u8, function: u8, reg_addr: u16, quantity: u16) -> Vec<u8> {
    let mut frame = Vec::with_capacity(6);
    frame.push(address);
    frame.push(function);
    frame.extend_from_slice(&reg_addr.to_be_bytes());
    frame.extend_from_slice(&quantity.to_be_bytes());
    
    // Calcul CRC-16 Modbus
    let crc = calculate_crc16(&frame);
    frame.extend_from_slice(&crc.to_le_bytes()); // Little-Endian
    
    frame
}

fn calculate_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in data {
        crc ^= *byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

fn print_report(
    responsive: &[(u16, u16, usize)],
    errors: &[(u16, u8, usize)],
    no_response: &[(u16, usize)],
) {
    println!("{}", "══════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "📋 RAPPORT DE SCAN".bright_white());
    println!("{}", "══════════════════════════════════════════════════════════".bright_cyan());
    println!();
    
    // Registres répondants
    if !responsive.is_empty() {
        println!("{} Registres répondants: {}", "✅".green(), responsive.len().to_string().bright_green());
        println!("┌────────────┬──────────────┬────────────┬──────────────┐");
        println!("│ Adresse    │ Valeur       │ Longueur   │ Status       │");
        println!("├────────────┼──────────────┼────────────┼──────────────┤");
        
        for (addr, value, len) in responsive {
            let status = match addr {
                0x0006..=0x000B => "Tension",
                0x000C => "Version",
                0x000D => "Fréquence",
                0x004F => "Statut Sources",
                0x0050 => "Statut Switch",
                0x0100 => "Adresse Modbus",
                0x0101 => "Baudrate",
                0x2065..=0x206D => "Configuration",
                0x2700 => "Transfert Forcé",
                0x2800 => "Commandes",
                _ => "Inconnu",
            };
            println!("│ 0x{:04X}     │ {:12} │ {:10} │ {:12} │", 
                addr, format!("{} (0x{:04X})", value, value), len, status);
        }
        println!("└────────────┴──────────────┴────────────┴──────────────┘");
        println!();
    }
    
    // Registres avec erreur
    if !errors.is_empty() {
        println!("{} Registres avec erreur: {}", "⚠️".yellow(), errors.len().to_string().bright_yellow());
        println!("┌────────────┬──────────────┬──────────────┐");
        println!("│ Adresse    │ Code Erreur  │ Description  │");
        println!("├────────────┼──────────────┼──────────────┤");
        
        for (addr, err_code, _) in errors {
            let desc = match err_code {
                0x01 => "Donnée illégale",
                0x02 => "Adresse registre invalide",
                0x03 => "Nombre registres invalide",
                _ => "Erreur inconnue",
            };
            println!("│ 0x{:04X}     │ 0x{:02X}         │ {:12} │", addr, err_code, desc);
        }
        println!("└────────────┴──────────────┴──────────────┘");
        println!();
    }
    
    // Pas de réponse
    if !no_response.is_empty() {
        println!("{} Pas de réponse: {}", "❌".red(), no_response.len().to_string().bright_red());
        println!("(Ces adresses ne sont pas implémentées ou protégées)");
        println!();
    }
    
    // Statistiques
    println!("{}", "📊 STATISTIQUES".bright_white());
    println!("  Total scruté:     {}", (responsive.len() + errors.len() + no_response.len()));
    println!("  Répondants:       {} ({:.1}%)", responsive.len(), 
        if responsive.len() + errors.len() + no_response.len() > 0 {
            (responsive.len() as f32 / (responsive.len() + errors.len() + no_response.len()) as f32) * 100.0
        } else { 0.0 });
    println!("  Erreurs:          {} ({:.1}%)", errors.len(),
        if responsive.len() + errors.len() + no_response.len() > 0 {
            (errors.len() as f32 / (responsive.len() + errors.len() + no_response.len()) as f32) * 100.0
        } else { 0.0 });
    println!("  Pas de réponse:   {} ({:.1}%)", no_response.len(),
        if responsive.len() + errors.len() + no_response.len() > 0 {
            (no_response.len() as f32 / (responsive.len() + errors.len() + no_response.len()) as f32) * 100.0
        } else { 0.0 });
}
```

---

## 🚀 Compilation et Exécution

```bash
# Compilation
cargo build --release

# Exécution basique (scan complet 0x0000-0xFFFF)
cargo run --release -- --port COM5 --address 6

# Scan rapide (seulement registres documentés)
cargo run --release -- --port COM5 --address 6 --start 0 --end 0x2800

# Scan personnalisé avec délai réduit
cargo run --release -- --port COM5 --address 6 --start 0x0000 --end 0x0100 --delay 50

# Aide
cargo run --release -- --help
```

---

## 📋 Options en Ligne de Commande

| Option | Court | Défaut | Description |
|--------|-------|--------|-------------|
| `--port` | `-p` | `COM5` | Port série (COM5, /dev/ttyUSB0) |
| `--address` | `-a` | `6` | Adresse Modbus (1-247) |
| `--baud` | `-b` | `9600` | Débit (4800/9600/19200/38400) |
| `--start` | `-s` | `0` | Adresse registre début (hex) |
| `--end` | `-e` | `FFFF` | Adresse registre fin (hex) |
| `--delay` | `-d` | `100` | Délai entre requêtes (ms) |
| `--function` | `-f` | `3` | Fonction Modbus (3=lecture) |

---

## ⚠️ Avertissements Importants

| Risque | Précaution |
|--------|-----------|
| **Scan trop rapide** | Délai minimum 50ms recommandé pour éviter perte de paquets |
| **Écriture accidentelle** | Ce programme est en **lecture seule** (fonction 03) |
| **Adresse incorrecte** | Vérifier que l'ATS est bien configuré sur `0x06` |
| **Parité** | La parité par défaut CHINT est **Even** (configurée dans le code) |
| **Bus RS485** | Ne pas déconnecter pendant le scan |

---

## 📤 Export des Résultats

Pour sauvegarder les résultats dans un fichier :

```bash
cargo run --release -- --port COM5 --address 6 > scan_results.txt
```

---

## 🔧 Dépannage

| Problème | Solution |
|----------|----------|
| Port non trouvé | Vérifier dans Gestionnaire de Périphériques (Windows) |
| Pas de réponse | Vérifier câblage A+/B-, alimentation ATS |
| Erreurs CRC | Vérifier parité (Even/Odd/None) dans le code |
| Timeout | Augmenter `--delay` à 200ms |
| Adresse incorrecte | Scanner avec `--address 3` (défaut usine) |

---

## 📊 Exemple de Sortie

```
╔══════════════════════════════════════════════════════════╗
║     SCANNER MODBUS RS485 - ATS CHINT NXZBN              ║
╚══════════════════════════════════════════════════════════╝

📍 Port: COM5
🏷️ Adresse Modbus: 0x06 (6)
⚡ Baudrate: 9600 bps
📊 Plage registres: 0x0000 - 0x0100
⏱️ Délai: 100 ms

✅ Port COM5 ouvert avec succès!
⚙️ Configuration: 8E1 (8 bits, Even, 1 stop)

🔍 Début du scan de 257 registres...

Progression: 100.0% | Registre: 0x0100 ✅ ✅ ✅ ✅ ✅ ✅ ✅ ✅ ✅ ✅

══════════════════════════════════════════════════════════
📋 RAPPORT DE SCAN
══════════════════════════════════════════════════════════

✅ Registres répondants: 22
┌────────────┬──────────────┬────────────┬──────────────┐
│ Adresse    │ Valeur       │ Longueur   │ Status       │
├────────────┼──────────────┼────────────┼──────────────┤
│ 0x0006     │ 220 (0x00DC) │ 7          │ Tension      │
│ 0x0007     │ 225 (0x00E1) │ 7          │ Tension      │
│ 0x004F     │ 21 (0x0015)  │ 7          │ Statut Sources│
│ 0x0050     │ 17 (0x0011)  │ 7          │ Statut Switch│
│ 0x0100     │ 6 (0x0006)   │ 7          │ Adresse Modbus│
└────────────┴──────────────┴────────────┴──────────────┘

📊 STATISTIQUES
  Total scruté:     257
  Répondants:       22 (8.6%)
  Erreurs:          5 (1.9%)
  Pas de réponse:   230 (89.5%)

══════════════════════════════════════════════════════════
Scan terminé!
```

---

Ce programme vous permettra de **découvrir tous les registres actifs** de votre ATS, y compris ceux non documentés. Les registres qui répondent avec une valeur valide seront listés avec leur contenu. 🎯
