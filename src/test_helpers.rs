#![cfg(test)]

use std::time;

use crate::timing::Duration;

pub fn test_eq<A: std::fmt::Debug + PartialEq>(left: A, right: A) -> anyhow::Result<()> {
    if left == right {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Equality test failed.\n  left:  {:?}\n  right: {:?}\n",
            left,
            right
        ))
    }
}

pub fn eventually<A: std::fmt::Debug>(action: impl Fn() -> anyhow::Result<A>) -> anyhow::Result<A> {
    let start_time = time::Instant::now();
    loop {
        let result = action();
        match result {
            Ok(_) => {
                return result;
            }
            Err(_) => {
                // fail if we've taken too long, otherwise retry after a short delay
                if time::Instant::now() - start_time >= time::Duration::from_secs(3) {
                    return result;
                }
            }
        }
        Duration::QUANTUM.sleep();
    }
}
