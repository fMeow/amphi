use amphi::amphi;

#[amphi]
mod amphi {
    #[amphi(blocking)]
    pub async fn async_fn() -> bool {
        false
    }
    #[amphi(asynchronous)]
    pub async fn async_fn() -> bool {
        true
    }
}

#[amphi::test]
#[async_std::test]
async fn test_async_std() {
    use self::amphi::async_fn;
    let res = async_fn().await;
    #[amphi(blocking)]
    {
        assert_eq!(res, false);
    }
    #[amphi(asynchronous)]
    {
        assert_eq!(res, true);
    }
}

#[amphi::test]
#[tokio::test]
async fn test_tokio() {
    use self::amphi::async_fn;
    let res = async_fn().await;
    #[amphi(blocking)]
    {
        assert_eq!(res, false);
    }
    #[amphi(asynchronous)]
    {
        assert_eq!(res, true);
    }
}
