use governor::{Quota, RateLimiter as GovernorRateLimiter, clock::DefaultClock, middleware::NoOpMiddleware, state::{InMemoryState, NotKeyed}};
use std::num::NonZeroU32;

fn main() {
    // Create a rate limiter with 10 requests per second
    let quota = Quota::per_second(NonZeroU32::new(10).unwrap());
    let limiter = GovernorRateLimiter::<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>::direct(quota);

    // Try to check different numbers of tokens
    for i in [1, 5, 10, 15, 20, 50, 100, 1000] {
        if let Ok(nz) = NonZeroU32::new(i) {
            let result = limiter.check_n(nz);
            println\!("check_n({}): {:?}", i, result.is_ok());
        }
    }

    // Check what the actual available capacity should be
    // by checking incremental values
    let mut capacity = 0;
    for i in 1..=100 {
        if let Ok(nz) = NonZeroU32::new(i) {
            if limiter.check_n(nz).is_ok() {
                capacity = i;
            } else {
                break;
            }
        }
    }
    println\!("Actual capacity: {}", capacity);
}
EOF < /dev/null
