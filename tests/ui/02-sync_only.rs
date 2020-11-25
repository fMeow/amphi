use amphi::amphi;

#[amphi(blocking_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
    use self::blocking::my_fn;
    let res = my_fn();
    assert_eq!(res, true);
}
