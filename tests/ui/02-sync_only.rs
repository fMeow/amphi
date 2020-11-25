use amphi::amphi;

#[amphi(sync_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
    use self::sync::my_fn;
    let res = my_fn();
    assert_eq!(res, true);
}
