//! Keyboard, pointer and touch input mapping. Inputs are queued and applied
//! by the deterministic core at explicit ticks; the DOM never owns state.

use crate::app::{App, Modal, Phase};
use neural_boundary_core::Action;
use web_sys::KeyboardEvent;

pub fn on_key(app: &mut App, event: &KeyboardEvent) {
    let key = event.key();

    // Modal layer first: trap focus, close on Escape.
    if let Some(modal) = app.modal {
        if key == "Tab" {
            if let Some(container) = app.modal_element(modal) {
                crate::accessibility::trap_focus(&container, event);
            }
            return;
        }
        match key.as_str() {
            "Escape" => {
                event.prevent_default();
                app.close_modal();
            }
            "h" | "H" if modal == Modal::Help => app.close_modal(),
            _ => {}
        }
        return;
    }

    match app.phase {
        Phase::Landing => match key.as_str() {
            "Enter" => {
                event.prevent_default();
                app.start_run(false);
            }
            "h" | "H" | "?" => app.open_modal(Modal::Help),
            _ => {}
        },
        Phase::Running => match key.as_str() {
            "ArrowUp" | "w" | "W" => {
                event.prevent_default();
                app.move_lane(-1);
            }
            "ArrowDown" | "s" | "S" => {
                event.prevent_default();
                app.move_lane(1);
            }
            "1" => app.queue_action(Action::Validate),
            "2" => app.queue_action(Action::Convert),
            "3" => app.queue_action(Action::Quarantine),
            "4" => app.queue_action(Action::ConsentGate),
            "5" => app.queue_action(Action::EvidenceGate),
            "Enter" => {
                event.prevent_default();
                app.queue_action(Action::Release);
            }
            "p" | "P" | " " => {
                event.prevent_default();
                app.pause();
            }
            "Escape" => app.pause(),
            "r" | "R" => app.start_run(false),
            "h" | "H" | "?" => app.open_modal(Modal::Help),
            _ => {}
        },
        Phase::Paused => match key.as_str() {
            "p" | "P" | " " | "Escape" => {
                event.prevent_default();
                app.resume();
            }
            "r" | "R" => app.start_run(false),
            "h" | "H" | "?" => app.open_modal(Modal::Help),
            _ => {}
        },
        Phase::Result => match key.as_str() {
            "Enter" | "r" | "R" => {
                event.prevent_default();
                app.start_run(true);
            }
            "n" | "N" => app.start_run(false),
            "h" | "H" | "?" => app.open_modal(Modal::Help),
            _ => {}
        },
    }
}

pub fn on_command(app: &mut App, command: &str) {
    match command {
        "start" => app.start_run(false),
        "restart" => app.start_run(false),
        "rerun-seed" => app.start_run(true),
        "new-run" => app.start_run(false),
        "pause" => app.toggle_pause(),
        "resume" => app.resume(),
        "to-menu" => app.return_to_menu(),
        "help" => app.open_modal(Modal::Help),
        "close-modal" => app.close_modal(),
        "lane-up" => app.move_lane(-1),
        "lane-down" => app.move_lane(1),
        "reset-data" => app.open_modal(Modal::Reset),
        "reset-confirm" => app.confirm_reset(),
        _ => {}
    }
}

pub fn on_action(app: &mut App, act: &str) {
    let action = match act {
        "1" => Action::Validate,
        "2" => Action::Convert,
        "3" => Action::Quarantine,
        "4" => Action::ConsentGate,
        "5" => Action::EvidenceGate,
        "release" => Action::Release,
        _ => return,
    };
    app.queue_action(action);
}
