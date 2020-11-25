use amphi::amphi;

#[amphi(blocking_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
    use self::asynchronous::my_fn;
}
