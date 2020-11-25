#![allow(dead_code)]

use amphi::{amphi};

#[amphi]
mod amphi{

    #[async_trait::async_trait]
    trait Trait {
        fn sync_fn() {}

        async fn declare_async(&self);

        async fn async_fn(&self) {
            self.declare_async().await
        }
    }

    #[async_trait::async_trait]
    pub trait PubTrait {
        fn sync_fn() {}

        async fn declare_async(&self);

        async fn async_fn(&self) {
            self.declare_async().await
        }
    }

    async fn async_fn() {}

    pub async fn pub_async_fn() {
    }

    pub struct Struct;

    #[async_trait::async_trait]
    impl PubTrait for Struct {
        fn sync_fn() {}

        async fn declare_async(&self) {}

        async fn async_fn(&self) {
            async { self.declare_async().await }.await
        }
    }
}


#[async_std::main]
async fn main() {
    // sync
    {
        use self::sync::{PubTrait, Struct, pub_async_fn};

        let s = Struct;
        s.declare_async();
        s.async_fn();
        pub_async_fn();
    }

    // async
    {
        use self::asynchronous::{PubTrait, Struct, pub_async_fn};

        let s = Struct;
        s.declare_async().await;
        s.async_fn().await;
        pub_async_fn().await;
    }
}
