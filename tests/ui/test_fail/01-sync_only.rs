use amphi::amphi;

#[amphi(sync_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
    use self::asynchronous::my_fn;
}
