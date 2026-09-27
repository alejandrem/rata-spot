//! Capa JSON/red (experimental, con fallback al DOM).
//!
//! Idea: el DOM cambia cada semanas; los JSON cada meses/anos. Como Brave
//! ya esta logueado, el `fetch` se hace DENTRO de la pagina (lleva cookies
//! y login): se pide el token web del propio player y con el se llama a la
//! Web API documentada (`api.spotify.com/v1/search`).
//!
//! Garantia: si el endpoint interno cambia o no hay token, todo regresa
//! Err y el llamador usa el DOM como antes. Kill-switch: `RATA_SPOT_API=0`.

mod fetch;
mod probe;
mod search;
#[cfg(test)]
mod tests;
mod token;

pub use probe::api_diag;
pub use search::search_via_api;
#[allow(unused_imports)]
pub(crate) use fetch::{build_fetch_js, page_fetch};
#[allow(unused_imports)]
pub(crate) use probe::net_probe;
#[allow(unused_imports)]
pub(crate) use search::{parse_search_api, search_with_token};
#[allow(unused_imports)]
pub(crate) use token::{encode, web_token};
