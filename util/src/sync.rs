use std::sync::{Arc, Mutex};
use super::verify::*;

pub fn finalize_arc_mutex<T>(arcmutex: Arc<Mutex<T>>) -> T {
    unwrap!(
        unwrap!(Arc::into_inner(arcmutex)) // "Returns the inner value, if the Arc has exactly one strong reference"   
            .into_inner() // "If another user of this mutex panicked while holding the mutex, then this call will return an error instead"
    )
}
