//! Accessibility helpers: polite announcements and modal focus management.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent};

/// Announce a meaningful state transition through the `#live` region.
/// Per-frame noise is never announced.
pub fn announce(document: &Document, text: &str) {
    if let Some(live) = document.get_element_by_id("live") {
        live.set_text_content(Some(text));
    }
}

fn focusables(container: &Element) -> Vec<HtmlElement> {
    let mut out = Vec::new();
    if let Ok(list) = container
        .query_selector_all("button, [href], input, select, [tabindex]:not([tabindex='-1'])")
    {
        for index in 0..list.length() {
            if let Some(node) = list.item(index) {
                if let Ok(element) = node.dyn_into::<HtmlElement>() {
                    out.push(element);
                }
            }
        }
    }
    out
}

pub fn focus_first(container: &Element) {
    if let Some(first) = focusables(container).into_iter().next() {
        let _ = first.focus();
    }
}

/// Trap Tab / Shift+Tab inside an open modal.
pub fn trap_focus(container: &Element, event: &KeyboardEvent) {
    if event.key() != "Tab" {
        return;
    }
    let items = focusables(container);
    let (Some(first), Some(last)) = (items.first(), items.last()) else {
        return;
    };
    let active = container
        .owner_document()
        .and_then(|document| document.active_element());
    let active_element: Option<&Element> = active.as_ref();
    if event.shift_key() {
        if active_element == Some(first.unchecked_ref()) {
            event.prevent_default();
            let _ = last.focus();
        }
    } else if active_element == Some(last.unchecked_ref()) {
        event.prevent_default();
        let _ = first.focus();
    }
}
