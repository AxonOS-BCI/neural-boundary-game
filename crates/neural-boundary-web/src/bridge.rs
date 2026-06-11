//! WASM bootstrap: panic hook, boot flag, listeners, fixed-step RAF loop.

use crate::{app::App, input};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement, KeyboardEvent, MouseEvent};

type FrameHandle = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or("missing window")?;
    let document = window.document().ok_or("missing document")?;

    // Boot flag for the explicit-failure watchdog in index.html.
    if let Some(root) = document.document_element() {
        let _ = root.set_attribute("data-nbg-booted", "1");
    }

    let app = Rc::new(RefCell::new(App::new(&window)?));

    // Reduced motion: initial value + live updates.
    if let Ok(Some(query)) = window.match_media("(prefers-reduced-motion: reduce)") {
        app.borrow_mut().reduced_motion = query.matches();
        let app_for_media = Rc::clone(&app);
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            if let Some(list) = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::MediaQueryList>().ok())
            {
                app_for_media.borrow_mut().reduced_motion = list.matches();
            }
        });
        let _ = query.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
        closure.forget();
    }

    // Keyboard.
    {
        let app = Rc::clone(&app);
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            input::on_key(&mut app.borrow_mut(), &event);
        });
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Click delegation: data-cmd, data-act, data-mode, data-difficulty.
    {
        let app = Rc::clone(&app);
        let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let Some(target) = event
                .target()
                .and_then(|target| target.dyn_into::<Element>().ok())
            else {
                return;
            };
            let find = |selector: &str| target.closest(selector).ok().flatten();
            if let Some(found) = find("[data-cmd]") {
                if let Some(command) = found.get_attribute("data-cmd") {
                    input::on_command(&mut app.borrow_mut(), &command);
                    return;
                }
            }
            if let Some(found) = find("[data-act]") {
                if let Some(act) = found.get_attribute("data-act") {
                    input::on_action(&mut app.borrow_mut(), &act);
                    return;
                }
            }
            if let Some(found) = find("[data-mode]") {
                if let Some(mode) = found.get_attribute("data-mode") {
                    app.borrow_mut().set_mode(&mode);
                    return;
                }
            }
            if let Some(found) = find("[data-difficulty]") {
                if let Some(difficulty) = found.get_attribute("data-difficulty") {
                    app.borrow_mut().set_difficulty(&difficulty);
                }
            }
        });
        document.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Canvas lane selection (click covers touch via synthesized click; no
    // separate touch handler avoids double activation).
    {
        let app = Rc::clone(&app);
        let canvas = app.borrow().ui.canvas.clone();
        let canvas_for_handler = canvas.clone();
        let closure = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let rect = canvas_for_handler.get_bounding_client_rect();
            let x = event.client_x() as f64 - rect.left();
            let y = event.client_y() as f64 - rect.top();
            app.borrow_mut().canvas_click(x, y);
        });
        canvas.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Auto-pause when the document is hidden.
    {
        let app = Rc::clone(&app);
        let doc = document.clone();
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
            if doc.visibility_state() == web_sys::VisibilityState::Hidden {
                app.borrow_mut().auto_pause();
            }
        });
        document.add_event_listener_with_callback(
            "visibilitychange",
            closure.as_ref().unchecked_ref(),
        )?;
        closure.forget();
    }

    // Focus the primary CTA for immediate keyboard start.
    if let Some(cta) = document.get_element_by_id("cta-run") {
        if let Ok(button) = cta.dyn_into::<HtmlElement>() {
            let _ = button.focus();
        }
    }

    // requestAnimationFrame fixed-step loop.
    {
        let app = Rc::clone(&app);
        let handle: FrameHandle = Rc::new(RefCell::new(None));
        let handle_clone = Rc::clone(&handle);
        *handle.borrow_mut() = Some(Closure::new(move |ts: f64| {
            app.borrow_mut().frame(ts);
            if let Some(window) = web_sys::window() {
                if let Some(closure) = handle_clone.borrow().as_ref() {
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                }
            }
        }));
        {
            let borrowed = handle.borrow();
            if let Some(closure) = borrowed.as_ref() {
                window.request_animation_frame(closure.as_ref().unchecked_ref())?;
            }
        }
    }

    Ok(())
}
