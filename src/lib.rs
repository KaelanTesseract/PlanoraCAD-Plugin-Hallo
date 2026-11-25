//! Beispiel-Plug-in (Vertrag 0.2). Es zeigt die Bausteine der Schnittstelle an kleinen Befehlen:
//!
//! - „Hallo sagen“: ein Dialog mit Text und Haken, der Gruß kommt als Meldung.
//! - „Hallo zählen“: ein Zähler im Dokument (eine Transaktion = ein Rückgängig-Schritt); der
//!   Baumknoten „Hallo ({n})“ zeigt ihn und bietet den Befehl im Rechtsklick an.
//! - „Text importieren“: der Datei-Dialog des Programms; die Meldung nennt die Zahl der Zeilen.
//! - „Profil lesen“: ein Dialog mit „Im Modell wählen“ (Recht `model-read`); die Meldung nennt
//!   die Zahl der Segmente des gewählten Profils.

// Der von wit-bindgen erzeugte Code für `ui.choose-file` hat mehr als sieben Parameter.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({
    path: "wit",
    world: "plugin",
    generate_all,
});

use planora::plugin::types::{
    Command, Dialog, Field, FieldKind, FieldValue, FileFilter, FileMode, Level, PickKind, Picked,
    Right, Section,
};
use planora::plugin::{document, files, i18n, model, rights, ui};
use std::sync::atomic::{AtomicU64, Ordering};

/// Schema der Daten im Dokument (steht auch als `document-schema` im Manifest).
const SCHEMA: u32 = 1;

/// Kennung der offenen Datei-Anfrage von „Text importieren“ (0 = keine).
static DATEI_ANFRAGE: AtomicU64 = AtomicU64::new(0);

/// Kennung der offenen Rechte-Anfrage von „Profil lesen“ (0 = keine).
static RECHT_ANFRAGE: AtomicU64 = AtomicU64::new(0);

struct Hallo;

fn befehl(id: &str, label_key: &str) -> Command {
    Command {
        id: id.into(),
        label_key: label_key.into(),
        icon: None,
        shortcut: None,
    }
}

/// Der Zähler im Abschnitt des aktiven Dokuments (vier Bytes, Little Endian); ohne Abschnitt
/// oder mit einem anderen Schema (etwa nur lesbare Daten einer neueren Version) 0.
fn zaehler() -> u32 {
    document::get()
        .filter(|s| s.schema == SCHEMA)
        .and_then(|s| <[u8; 4]>::try_from(s.bytes.as_slice()).ok())
        .map_or(0, u32::from_le_bytes)
}

/// Erhöht den Zähler um eins: eine Transaktion, also genau ein Rückgängig-Schritt „Hallo zählen“.
fn zaehlen() -> Result<(), String> {
    let n = zaehler().saturating_add(1);
    document::begin("hallo-zaehlen-schritt")?;
    document::set(&Section {
        schema: SCHEMA,
        bytes: n.to_le_bytes().to_vec(),
    })?;
    document::commit()
}

fn gruss_dialog() -> Result<(), String> {
    ui::open_dialog(&Dialog {
        id: "gruss".into(),
        title_key: "hallo-titel".into(),
        fields: vec![
            Field {
                id: "name".into(),
                label_key: "hallo-name".into(),
                kind: FieldKind::Text(String::new()),
            },
            Field {
                id: "laut".into(),
                label_key: "hallo-laut".into(),
                kind: FieldKind::Toggle(false),
            },
        ],
    })
}

fn profil_dialog() -> Result<(), String> {
    ui::open_dialog(&Dialog {
        id: "profil".into(),
        title_key: "hallo-profil-titel".into(),
        fields: vec![Field {
            id: "profil".into(),
            label_key: "hallo-profil-feld".into(),
            kind: FieldKind::Pick(PickKind::Profile),
        }],
    })
}

/// Liest das Profil `index` der Skizze `sketch` und meldet die Zahl der Segmente.
fn profil_melden(sketch: u64, index: u32) {
    let text = match model::profiles(sketch) {
        Ok(alle) => match alle.get(index as usize) {
            Some(p) => {
                let innen: usize = p.holes.iter().map(Vec::len).sum();
                i18n::text(
                    "hallo-profil-segmente",
                    &[
                        ("aussen".to_string(), p.outer.len().to_string()),
                        ("loecher".to_string(), p.holes.len().to_string()),
                        ("innen".to_string(), innen.to_string()),
                    ],
                )
            }
            None => i18n::text("hallo-profil-fehlt", &[]),
        },
        Err(e) => e,
    };
    ui::message(Level::Info, &text);
}

impl Guest for Hallo {
    fn init(_info: HostInfo) -> Result<Registration, String> {
        Ok(Registration {
            commands: vec![
                befehl("hallo", "hallo-befehl"),
                befehl("zaehlen", "hallo-zaehlen-befehl"),
                befehl("text-importieren", "hallo-import-befehl"),
                befehl("profil-lesen", "hallo-profil-befehl"),
            ],
        })
    }

    fn run_command(id: String) -> Result<(), String> {
        match id.as_str() {
            "hallo" => gruss_dialog(),
            "zaehlen" => zaehlen(),
            "text-importieren" => {
                let filter = FileFilter {
                    label_key: "hallo-textdateien".into(),
                    extensions: vec!["txt".into()],
                };
                let r = ui::choose_file(FileMode::Open, &[filter], "");
                DATEI_ANFRAGE.store(r, Ordering::Relaxed);
                Ok(())
            }
            "profil-lesen" => {
                // „Im Modell wählen“ braucht das Recht; ohne Recht zuerst nachfragen.
                if rights::granted(Right::ModelRead) {
                    profil_dialog()
                } else {
                    let r = rights::request(Right::ModelRead);
                    RECHT_ANFRAGE.store(r, Ordering::Relaxed);
                    Ok(())
                }
            }
            _ => Err(format!("unbekannter Befehl: {id}")),
        }
    }

    fn handle_dialog(id: String, event: DialogEvent) -> Result<(), String> {
        let DialogEvent::Ok(values) = event else {
            return Ok(());
        };
        match id.as_str() {
            "gruss" => {
                let mut name = String::new();
                let mut laut = false;
                for (feld, wert) in values {
                    match (feld.as_str(), wert) {
                        ("name", FieldValue::Text(t)) => name = t,
                        ("laut", FieldValue::Toggle(b)) => laut = b,
                        _ => {}
                    }
                }
                let mut text = i18n::text("hallo-gruss", &[("name".to_string(), name)]);
                if laut {
                    text = text.to_uppercase();
                }
                ui::message(Level::Info, &text);
            }
            "profil" => {
                for (feld, wert) in values {
                    if feld != "profil" {
                        continue;
                    }
                    match wert {
                        FieldValue::Picked(Picked::Profile((sketch, index))) => {
                            profil_melden(sketch, index);
                        }
                        _ => ui::message(Level::Info, &i18n::text("hallo-profil-keins", &[])),
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_result(id: u64, result: RequestResult) {
        if id == DATEI_ANFRAGE.load(Ordering::Relaxed) {
            DATEI_ANFRAGE.store(0, Ordering::Relaxed);
            let griff = match result {
                RequestResult::File(griff) => griff,
                // Der Dialog ließ sich nicht zeigen: den Grund melden.
                RequestResult::Failed(t) => {
                    ui::message(Level::Warning, &t);
                    return;
                }
                // Abbruch im Datei-Dialog: nichts zu tun.
                _ => return,
            };
            let text = match files::read(griff) {
                Ok(bytes) => {
                    let zeilen = String::from_utf8_lossy(&bytes).lines().count();
                    i18n::text(
                        "hallo-import-zeilen",
                        &[("zeilen".to_string(), zeilen.to_string())],
                    )
                }
                Err(e) => e,
            };
            ui::message(Level::Info, &text);
        } else if id == RECHT_ANFRAGE.load(Ordering::Relaxed) {
            RECHT_ANFRAGE.store(0, Ordering::Relaxed);
            match result {
                RequestResult::Granted => {
                    if let Err(e) = profil_dialog() {
                        ui::message(Level::Error, &e);
                    }
                }
                _ => ui::message(Level::Warning, &i18n::text("hallo-profil-recht", &[])),
            }
        }
    }

    fn shutdown() {}

    // Das Programm fragt den Baum nach jedem Commit, Rückgängig und Tab-Wechsel selbst neu ab;
    // hier ist nichts zu merken.
    fn on_document(_e: DocumentEvent) {}

    // Es gibt nur Schema 1; ein älteres Schema kann es nicht geben.
    fn section_upgrade(old_schema: u32, _bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        Err(format!("unbekanntes Schema {old_schema}"))
    }

    // Ohne Daten im Dokument kein Knoten (und damit auch keine Wurzel „Beispiel-Hallo“).
    fn tree() -> Vec<TreeNode> {
        if document::get().is_none() {
            return Vec::new();
        }
        let n = zaehler();
        vec![TreeNode {
            id: "zaehler".into(),
            parent: None,
            label: i18n::text("hallo-knoten", &[("n".to_string(), n.to_string())]),
            icon: None,
            visible: None,
            commands: vec!["zaehlen".into()],
        }]
    }

    fn on_tree(node: String, e: TreeEvent) {
        // Rechtsklick „Hallo zählen“ am Knoten: derselbe Befehl wie im Menü.
        if node == "zaehler"
            && let TreeEvent::Command(c) = e
            && c == "zaehlen"
            && let Err(e) = zaehlen()
        {
            ui::message(Level::Error, &e);
        }
    }

    fn on_model(_changed: Vec<u64>) {}
}

export!(Hallo);
