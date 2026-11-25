# PlanoraCAD-Plugin-Hallo

Beispiel-Plug-in für [PlanoraCAD](https://github.com/KaelanTesseract/PlanoraCAD): Es öffnet einen Dialog und sagt Hallo.

Es dient als Vorlage für eigene Plug-ins und als Test für den Plug-in-Katalog.

## Installieren

In PlanoraCAD über den Plug-in-Katalog. Alternativ die `.pcplugin`-Datei aus dem neuesten Release herunterladen und im Programm installieren.

## Selbst bauen

Gebraucht wird Rust mit dem Ziel `wasm32-wasip2`:

```
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2
```

Zum Packen dient das Werkzeug `pack` aus dem PlanoraCAD-Repo (`crates/planora-plugin-host/examples/pack.rs`). Wie das genau geht, steht in `docs/plugins/entwickler.md` im PlanoraCAD-Repo.

`wit/` ist eine Kopie der Schnittstellenbeschreibung (API 0.2) aus PlanoraCAD.

## Lizenz

GPL-3.0-only, siehe `LICENSE`.
