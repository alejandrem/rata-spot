//! Tests: las consts y su JS no se pueden divorciar.

use super::*;

#[test]
fn js_menciona_sus_consts() {
    assert!(PLAY_CLICK_JS.contains(PLAY_TESTIDS[0]));
    assert!(PLAY_CLICK_JS.contains(PLAY_TESTIDS[1]));
    assert!(PLAY_CLICK_JS.contains("Reproducir"));
    assert!(PLAY_CLICK_JS.contains("\"Play\""), "PLAY_CLICK sin fallback EN (bug #30)");
    assert!(LIBRARY_JS.contains(LIBRARY_ROW_PREFIX));
    assert!(LIBRARY_JS.contains(SIDEBAR_ID));
    // Regla #30: ningún lector/click usa aria-label en un solo idioma.
    assert!(
        LIBRARY_JS.contains("biblioteca|library"),
        "LIBRARY_JS sin fallback ES/EN (bug #30)"
    );
    assert!(
        LIBRARY_DIAG_JS.contains("biblioteca|library"),
        "LIBRARY_DIAG_JS sin fallback ES/EN (bug #30)"
    );
    assert!(LIBRARY_DIAG_JS.contains(LIBRARY_ROW_PREFIX));
    assert!(SEARCH_JS.contains("/track/"));
    assert!(SEARCH_JS.contains(SIDEBAR_ID));
    assert!(TRACKS_JS.contains(TRACK_LINK_TESTID));
    assert!(TRACKS_JS.contains(TRACKLIST_TESTIDS[0]));
    assert!(TRACKS_JS.contains(TRACKLIST_TESTIDS[1]));
    assert!(TRACKLIST_EXISTS_JS.contains(TRACKLIST_TESTIDS[0]));
    assert!(TRACKLIST_EXISTS_JS.contains(TRACKLIST_TESTIDS[1]));
    assert!(TRACKLIST_SCROLL_TOP_JS.contains(TRACKLIST_TESTIDS[0]));
    assert!(TRACKLIST_SCROLL_TOP_JS.contains(TRACKLIST_TESTIDS[1]));
    assert!(TRACKLIST_SCROLL_JS.contains(TRACKLIST_TESTIDS[0]));
    assert!(TRACKLIST_SCROLL_JS.contains(TRACKLIST_TESTIDS[1]));
    assert!(PLAYER_ARIA_JS.contains(PLAY_TESTIDS[0]));
    assert!(DASHBOARD_JS.contains(TRACK_PAGE_TESTID));
    assert!(track_row_click_js("abc123").contains(TRACK_LINK_TESTID));
    assert!(track_row_click_js("abc123").contains("abc123"));
    assert!(
        track_row_click_js("abc123").contains("Reproducir"),
        "row_click sin ES (bug #30)"
    );
    assert!(
        track_row_click_js("abc123").contains("Play"),
        "row_click sin EN (bug #30)"
    );
    // Blindaje 800x600: los errores de grid llevan viewport para diagnosticar
    // colapso responsive sin adivinar (si Spotify cambia breakpoints).
    assert!(LIBRARY_JS.contains("innerWidth"), "LIBRARY_JS sin viewport en no-grid");
    assert!(TRACKS_JS.contains("innerWidth"), "TRACKS_JS sin viewport en errores");
    assert!(HEALTH_JS.contains("innerWidth"), "HEALTH_JS sin viewport");
}

#[test]
fn resumen_health() {
    let ok = r#"{"checks":[{"name":"sidebar","ok":true},{"name":"main","ok":true}]}"#;
    assert_eq!(summarize_health(ok), "DOM 2/2 ok");
    let fail = r#"{"checks":[{"name":"sidebar","ok":true},{"name":"library_grid","ok":false},{"name":"main","ok":false}]}"#;
    assert_eq!(
        summarize_health(fail),
        "DOM 1/3 ok, fallan: library_grid,main"
    );
    assert!(summarize_health("no-json").contains("ilegible"));
    assert!(summarize_health("{}").contains("sin checks"));
    // El health reporta viewport cuando viene (blindaje tamaño 800x600).
    let sized = r#"{"checks":[{"name":"sidebar","ok":true}],"vw":800,"vh":600}"#;
    assert_eq!(summarize_health(sized), "DOM 1/1 ok [800x600]");
}
