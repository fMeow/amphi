use amphi::amphi;

#[amphi(async_only)]
mod amphi{
    pub async fn my_fn() -> bool {
        true
    }
}

fn main() {
    use self::blocking::my_fn;
}
