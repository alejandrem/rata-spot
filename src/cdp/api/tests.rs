//! Tests puros de la capa API (sin Brave vivo).

use super::fetch::build_fetch_js;
use super::search::parse_search_api;
use super::token::encode;

#[test]
fn encode_espacios_y_tildes() {
    assert_eq!(encode("pika pika"), "pika%20pika");
    assert!(encode("niña").contains("%C3%B1"));
    assert_eq!(encode("abc-_.~09AZ"), "abc-_.~09AZ");
}

#[test]
fn fetch_js_escapa_comillas() {
    let js = build_fetch_js("https://x/?q=\"hola\"\\", None);
    assert!(js.contains("\\\"hola\\\""));
    assert!(js.contains(", null)"));
    let js2 = build_fetch_js("https://x/", Some("tok"));
    assert!(js2.contains("\"tok\""));
}

#[test]
fn parsea_search_api() {
    let v: serde_json::Value = serde_json::from_str(
        r#"{
          "tracks": {"items": [
            {"uri": "spotify:track:AAA", "name": "Pika Pika",
             "artists": [{"name": "A"}, {"name": "B"}]},
            {"uri": "spotify:track:AAA", "name": "Duplicado"},
            {"uri": "", "name": "SinUri"}
          ]},
          "artists": {"items": [
            {"uri": "spotify:artist:BBB", "name": "Pikachu", "artists": []}
          ]},
          "albums": {"items": []},
          "playlists": {"items": [
            {"uri": "spotify:playlist:CCC", "name": "Mix",
             "owner": {"display_name": "rata"}}
          ]}
        }"#,
    )
    .unwrap();
    let items = parse_search_api(&v);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].kind, "track");
    assert_eq!(items[0].detail, "A, B");
    assert_eq!(items[1].uri, "spotify:artist:BBB");
    assert_eq!(items[2].detail, "rata");
}
