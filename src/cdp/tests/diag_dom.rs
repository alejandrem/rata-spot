//! Diag_dom: biblioteca + selectores + API (solo imprimen, siempre pasan).

/// Diagnostico del DOM (biblioteca cruda).
#[tokio::test]
async fn diag_estado() {
    use crate::cdp::library::library_diag;
    match library_diag().await {
        Ok(s) => println!("DIAG: {s}"),
        Err(e) => println!("DIAG-ERR: {e}"),
    }
}

/// Health-check de selectores: dice que vive y que murio.
#[tokio::test]
async fn diag_selectores() {
    use crate::cdp::health_summary;
    println!("DIAG-SEL: {}", health_summary().await);
}

/// Diagnostico de la capa API/red: token web + search JSON.
#[tokio::test]
async fn diag_api() {
    use crate::cdp::api_diag;
    println!("DIAG-API: {}", api_diag().await);
}
