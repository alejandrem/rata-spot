//! Page: diagnostico rapido titulo + player + login.

/// Diagnostico rapido de la pagina: titulo + hay player + hay login.
pub(crate) const PAGE_STATE_JS: &str = r#"(() => {
  const q = (s) => !!document.querySelector(s);
  return document.title + ' | play=' + q('[data-testid="control-button-playpause"]')
    + ' | login=' + (q('[data-testid="login-button"]') || q('a[href*="login"]'));
})()"#;
