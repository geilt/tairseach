//! Location Handler
//!
//! Handles location-related JSON-RPC methods using CoreLocation.
//! Returns lat/lng/altitude/accuracy from CLLocationManager.

use serde_json::Value;
use tracing::info;

use super::super::protocol::JsonRpcResponse;

/// Handle location-related methods
pub async fn handle(action: &str, params: &Value, id: Value) -> JsonRpcResponse {
    match action {
        "get" => handle_get(params, id).await,
        "watch" => handle_watch(params, id).await,
        _ => JsonRpcResponse::method_not_found(id, &format!("location.{}", action)),
    }
}

/// Get the current location
async fn handle_get(params: &Value, id: Value) -> JsonRpcResponse {
    let timeout_secs = params
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);

    match get_current_location(timeout_secs).await {
        Ok(loc) => JsonRpcResponse::success(id, loc),
        Err(e) => JsonRpcResponse::error(id, -32000, e, None),
    }
}

/// Watch for location updates (single-shot with subscription stub)
async fn handle_watch(_params: &Value, id: Value) -> JsonRpcResponse {
    JsonRpcResponse::error(
        id,
        -32001,
        "location.watch requires a streaming connection, not yet supported. Use location.get instead.",
        None,
    )
}

// ============================================================================
// Native CoreLocation integration — in-process via objc2
// ============================================================================

#[cfg(target_os = "macos")]
mod native {
    use std::cell::Cell;
    use std::time::{Duration, Instant};

    use objc2::rc::Retained;
    use objc2::runtime::{NSObject, ProtocolObject};
    use objc2::{define_class, msg_send, AllocAnyThread, ClassType, DefinedClass};
    use objc2_core_location::{CLLocation, CLLocationManager, CLLocationManagerDelegate};
    use objc2_foundation::{NSArray, NSError, NSObjectProtocol};

    /// Instance variables for our delegate
    struct LocationDelegateIvars {
        /// Latitude (0.0 means not yet received)
        latitude: Cell<f64>,
        /// Longitude
        longitude: Cell<f64>,
        /// Altitude
        altitude: Cell<f64>,
        /// Horizontal accuracy
        horizontal_accuracy: Cell<f64>,
        /// Vertical accuracy
        vertical_accuracy: Cell<f64>,
        /// Speed
        speed: Cell<f64>,
        /// Course
        course: Cell<f64>,
        /// Whether we have a location
        has_location: Cell<bool>,
        /// Error code (0 = no error)
        error_code: Cell<isize>,
        /// Whether a fatal error occurred
        has_fatal_error: Cell<bool>,
    }

    define_class!(
        // SAFETY: CLLocationManager delegate protocol is safe to implement.
        // We don't subclass anything special.
        #[unsafe(super(NSObject))]
        #[thread_kind = AllocAnyThread]
        #[ivars = LocationDelegateIvars]
        struct LocationDelegate;

        unsafe impl NSObjectProtocol for LocationDelegate {}

        unsafe impl CLLocationManagerDelegate for LocationDelegate {
            #[unsafe(method(locationManager:didUpdateLocations:))]
            fn location_manager_did_update_locations(
                &self,
                _manager: &CLLocationManager,
                locations: &NSArray<CLLocation>,
            ) {
                let count = locations.count();
                if count == 0 {
                    return;
                }
                // Get the last (most recent) location
                let loc: Retained<CLLocation> = unsafe {
                    msg_send![locations, objectAtIndex: count - 1]
                };
                let coord = unsafe { loc.coordinate() };
                self.ivars().latitude.set(coord.latitude);
                self.ivars().longitude.set(coord.longitude);
                self.ivars().altitude.set(unsafe { loc.altitude() });
                self.ivars().horizontal_accuracy.set(unsafe { loc.horizontalAccuracy() });
                self.ivars().vertical_accuracy.set(unsafe { loc.verticalAccuracy() });
                self.ivars().speed.set(unsafe { loc.speed() });
                self.ivars().course.set(unsafe { loc.course() });
                let _ = &loc; // prevent drop
                self.ivars().has_location.set(true);
            }

            #[unsafe(method(locationManager:didFailWithError:))]
            fn location_manager_did_fail_with_error(
                &self,
                _manager: &CLLocationManager,
                error: &NSError,
            ) {
                let code = error.code();
                // kCLErrorLocationUnknown = 0 is transient — keep waiting
                if code == 0 {
                    return;
                }
                // All other errors are fatal
                self.ivars().error_code.set(code);
                self.ivars().has_fatal_error.set(true);
            }
        }
    );

    impl LocationDelegate {
        fn new() -> Retained<Self> {
            let this = Self::alloc().set_ivars(LocationDelegateIvars {
                latitude: Cell::new(0.0),
                longitude: Cell::new(0.0),
                altitude: Cell::new(0.0),
                horizontal_accuracy: Cell::new(0.0),
                vertical_accuracy: Cell::new(0.0),
                speed: Cell::new(0.0),
                course: Cell::new(0.0),
                has_location: Cell::new(false),
                error_code: Cell::new(0isize),
                has_fatal_error: Cell::new(false),
            });
            unsafe { msg_send![super(this), init] }
        }
    }

    /// Location result
    pub struct LocationResult {
        pub latitude: f64,
        pub longitude: f64,
        pub altitude: f64,
        pub horizontal_accuracy: f64,
        pub vertical_accuracy: f64,
        pub speed: f64,
        pub course: f64,
    }

    /// Error from CoreLocation
    pub enum LocationError {
        TimedOut,
        Denied,
        CLError(isize),
    }

    impl std::fmt::Display for LocationError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LocationError::TimedOut => write!(f, "Location request timed out"),
                LocationError::Denied => write!(f, "Location access denied"),
                LocationError::CLError(code) => write!(f, "CoreLocation error (code {})", code),
            }
        }
    }

    /// Get current location using in-process CLLocationManager.
    ///
    /// This runs on a dedicated thread with its own run loop so that
    /// CLLocationManager delegate callbacks can fire. The calling async
    /// task awaits the result via a oneshot channel.
    pub fn get_location(timeout_secs: u64) -> Result<LocationResult, LocationError> {
        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn a thread that owns the CLLocationManager and runs a run loop
        std::thread::spawn(move || {
            let result = get_location_on_thread(timeout_secs);
            let _ = tx.send(result);
        });

        // Wait for the result
        rx.recv().unwrap_or(Err(LocationError::TimedOut))
    }

    /// Internal: run CLLocationManager on the current thread with run loop polling.
    fn get_location_on_thread(timeout_secs: u64) -> Result<LocationResult, LocationError> {
        let delegate = LocationDelegate::new();
        let manager = unsafe { CLLocationManager::new() };

        unsafe {
            let delegate_proto = ProtocolObject::from_ref(&*delegate);
            manager.setDelegate(Some(delegate_proto));
            manager.setDesiredAccuracy(objc2_core_location::kCLLocationAccuracyBest);
            manager.startUpdatingLocation();
        }

        // Poll the run loop until we get a location or error
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);

        while !delegate.ivars().has_location.get()
            && !delegate.ivars().has_fatal_error.get()
            && Instant::now() < deadline
        {
            // Run the run loop for a short interval to process callbacks
            unsafe {
                let interval = 0.1_f64;
                let date: Retained<objc2_foundation::NSDate> =
                    msg_send![objc2_foundation::NSDate::class(), dateWithTimeIntervalSinceNow: interval];
                let run_loop: *mut NSObject = msg_send![
                    objc2::class!(NSRunLoop),
                    currentRunLoop
                ];
                let mode = objc2_foundation::NSString::from_str("kCFRunLoopDefaultMode");
                let _: bool = msg_send![run_loop, runMode: &*mode, beforeDate: &*date];
            }
        }

        // Stop updates
        unsafe {
            manager.stopUpdatingLocation();
        }

        if delegate.ivars().has_location.get() {
            Ok(LocationResult {
                latitude: delegate.ivars().latitude.get(),
                longitude: delegate.ivars().longitude.get(),
                altitude: delegate.ivars().altitude.get(),
                horizontal_accuracy: delegate.ivars().horizontal_accuracy.get(),
                vertical_accuracy: delegate.ivars().vertical_accuracy.get(),
                speed: delegate.ivars().speed.get(),
                course: delegate.ivars().course.get(),
            })
        } else if delegate.ivars().has_fatal_error.get() {
            let code = delegate.ivars().error_code.get();
            if code == 1 {
                Err(LocationError::Denied)
            } else {
                Err(LocationError::CLError(code))
            }
        } else {
            Err(LocationError::TimedOut)
        }
    }
}

/// Get the current location using native CoreLocation (in-process).
///
/// Runs CLLocationManager on a dedicated thread with its own run loop,
/// so delegate callbacks fire correctly. The result is passed back via
/// a channel. This runs in the Tairseach process, inheriting its TCC
/// permissions — no subprocess needed.
#[cfg(target_os = "macos")]
async fn get_current_location(timeout_secs: u64) -> Result<Value, String> {
    info!(
        "Getting current location via native CoreLocation (timeout={}s)",
        timeout_secs
    );

    // Run the blocking location fetch on a thread pool
    let result = tokio::task::spawn_blocking(move || native::get_location(timeout_secs))
        .await
        .map_err(|e| format!("Location task panicked: {}", e))?;

    match result {
        Ok(loc) => {
            info!(
                "Got location: lat={}, lng={}",
                loc.latitude, loc.longitude
            );
            Ok(serde_json::json!({
                "latitude": loc.latitude,
                "longitude": loc.longitude,
                "altitude": loc.altitude,
                "horizontalAccuracy": loc.horizontal_accuracy,
                "verticalAccuracy": loc.vertical_accuracy,
                "speed": loc.speed,
                "course": loc.course,
            }))
        }
        Err(e) => Err(format!("CoreLocation error: {}", e)),
    }
}

#[cfg(not(target_os = "macos"))]
async fn get_current_location(_timeout_secs: u64) -> Result<Value, String> {
    Err("Location is only available on macOS".to_string())
}
