use amphi::amphi;

#[amphi(async_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

#[async_std::main]
async fn main() {
    use self::asynchronous::my_fn;
    let res = my_fn().await;
    assert_eq!(res, true);
}
