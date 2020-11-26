#![allow(dead_code)]
use amphi::amphi;

#[amphi(async_only, path="/home/user/project/lib.rs")]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
}
