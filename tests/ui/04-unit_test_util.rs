use amphi::amphi;

#[amphi]
mod amphi{
    pub async fn async_fn() -> bool {
        true
    }
}

#[amphi::test]
#[async_std::test]
async fn test_async_std() {
    use self::amphi::async_fn;
    let res = async_fn().await;
    assert_eq!(res, true);
}

#[amphi::test]
#[tokio::test]
async fn test_tokio() {
    use self::amphi::async_fn;
    let res = async_fn().await;
    assert_eq!(res, true);
}


fn main() {

}
