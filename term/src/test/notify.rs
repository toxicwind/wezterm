//! Tests for OSC 777 (rxvt `notify`) toast notifications.
//!
//! OSC 777 is already wired end-to-end (parser -> performer ->
//! MuxNotification::Alert -> GUI toast); these tests pin the terminal
//! side of that contract.

use super::*;
use k9::assert_equal;

struct Recorder {
    alerts: Arc<Mutex<Vec<Alert>>>,
}

impl AlertHandler for Recorder {
    fn alert(&mut self, alert: Alert) {
        self.alerts.lock().unwrap().push(alert);
    }
}

fn term_with_alert_recorder() -> (TestTerm, Arc<Mutex<Vec<Alert>>>) {
    let mut term = TestTerm::new(3, 10, 0);
    let alerts = Arc::new(Mutex::new(Vec::new()));
    term.set_notification_handler(Box::new(Recorder {
        alerts: Arc::clone(&alerts),
    }));
    (term, alerts)
}

/// `OSC 777 ; notify ; title ; body ST` fires a toast notification with
/// the given title and body.
#[test]
fn osc_777_notify_fires_toast_notification() {
    let (mut term, alerts) = term_with_alert_recorder();

    term.print(b"\x1b]777;notify;Tea Time;the tea is ready\x1b\\");

    let alerts = alerts.lock().unwrap();
    assert_equal!(
        *alerts,
        [Alert::ToastNotification {
            title: Some("Tea Time".to_string()),
            body: "the tea is ready".to_string(),
            focus: true,
        }]
    );
}

/// `OSC 777 ; notify ; body ST` (two params) maps the single param to the
/// notification body with no title.
#[test]
fn osc_777_notify_body_only() {
    let (mut term, alerts) = term_with_alert_recorder();

    term.print(b"\x1b]777;notify;the tea is ready\x1b\\");

    let alerts = alerts.lock().unwrap();
    assert_equal!(
        *alerts,
        [Alert::ToastNotification {
            title: None,
            body: "the tea is ready".to_string(),
            focus: true,
        }]
    );
}
