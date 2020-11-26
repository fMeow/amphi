#![allow(dead_code)]
use amphi::amphi;

#[amphi(async_only, blocking_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
}
