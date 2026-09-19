//! A paste target may read the clipboard long after Ctrl+V is queued.
//! Leave the verified transcription available instead of restoring stale text.

const ATTEMPTS: usize = 5;

pub fn paste_verified(
    text: &str,
    mut write_and_read: impl FnMut(&str) -> Result<String, String>,
    paste: impl FnOnce(),
    mut wait: impl FnMut(),
) -> Result<(), String> {
    let mut last_error = String::from("clipboard verification failed");
    for attempt in 0..ATTEMPTS {
        match write_and_read(text) {
            Ok(actual) if actual == text => {
                paste();
                // Do not restore the previous clipboard on a timer. There is no
                // cross-application acknowledgement that the paste was consumed.
                return Ok(());
            }
            Ok(_) => last_error = String::from("clipboard changed before paste"),
            Err(error) => last_error = error,
        }
        if attempt + 1 < ATTEMPTS {
            wait();
        }
    }
    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn delayed_consumer_still_reads_the_transcription() {
        let clipboard = RefCell::new(String::from("old private clipboard"));
        let queued = Cell::new(false);
        paste_verified(
            "fresh speech",
            |text| {
                *clipboard.borrow_mut() = text.into();
                Ok(clipboard.borrow().clone())
            },
            || queued.set(true),
            || {},
        )
        .unwrap();
        // Model a target that consumes the queued paste only AFTER this helper
        // returns (the former timer restoration failed for precisely this case).
        assert!(queued.get());
        assert_eq!(*clipboard.borrow(), "fresh speech");
    }

    #[test]
    fn clipboard_contention_is_retried_before_pasting() {
        let attempts = Cell::new(0);
        let pastes = Cell::new(0);
        let waits = Cell::new(0);
        paste_verified(
            "speech",
            |text| {
                attempts.set(attempts.get() + 1);
                match attempts.get() {
                    1 => Err("busy".into()),
                    2 => Ok("different clipboard".into()),
                    _ => Ok(text.into()),
                }
            },
            || pastes.set(pastes.get() + 1),
            || waits.set(waits.get() + 1),
        )
        .unwrap();
        assert_eq!(attempts.get(), 3);
        assert_eq!(pastes.get(), 1);
        assert_eq!(waits.get(), 2);
    }

    #[test]
    fn persistent_mismatch_never_pastes_old_text() {
        let pasted = Cell::new(false);
        let result = paste_verified("speech", |_| Ok("stale".into()), || pasted.set(true), || {});
        assert!(result.is_err());
        assert!(!pasted.get());
    }

    #[test]
    fn persistent_clipboard_error_is_bounded_and_never_pastes() {
        let attempts = Cell::new(0);
        let result = paste_verified(
            "speech",
            |_| {
                attempts.set(attempts.get() + 1);
                Err("busy".into())
            },
            || panic!("must not paste"),
            || {},
        );
        assert_eq!(result.unwrap_err(), "busy");
        assert_eq!(attempts.get(), ATTEMPTS);
    }
}
