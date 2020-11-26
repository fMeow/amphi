#![allow(redundant_semicolons)]

use amphi::amphi;

#[amphi]
mod amphi {
    // sync struct
    #[amphi(blocking)]
    pub struct Foo(pub ());

    // async struct
    #[amphi(asynchronous)]
    pub struct Foo(pub String);

    // sync trait
    #[amphi(blocking)]
    pub trait Bar {
        fn sync_method(&self) -> &'static str;
    }

    #[amphi(blocking)]
    impl Bar for Foo {
        fn sync_method(&self) -> &'static str {
            "sync_method for Foo"
        }
    }

    // async trait
    #[amphi(asynchronous)]
    pub trait Bar {
        fn async_method(&self) -> &'static str;
    }

    #[amphi(asynchronous)]
    impl Bar for Foo {
        fn async_method(&self) -> &'static str {
            "async_method for Foo"
        }
    }

    // sync fn
    #[amphi(blocking)]
    pub fn sync_impl() -> Foo {
        Foo(())
    }

    //async fn
    #[amphi(asynchronous)]
    pub fn async_impl() -> Foo {
        Foo(String::new())
    }


    // sync and async division on expressions
    pub fn use_foo() {
        #[amphi(blocking)]
            Foo(());
        #[amphi(blocking)]
            let _ = Foo(());
        #[amphi(blocking)]
            { Foo(()); }


        #[amphi(asynchronous)]
            Foo(String::new());
        #[amphi(asynchronous)]
            let _ = Foo(String::new());
        #[amphi(asynchronous)]
            { Foo(String::new()); }
    }
}

fn main() {
    // sync
    use self::blocking::{Foo as BlockingFoo, Bar as BlockingBar, sync_impl};
    let _ = sync_impl();
    let foo = BlockingFoo(());
    assert_eq!(foo.sync_method(), "sync_method for Foo");
    // async
    use self::asynchronous::{Foo, Bar, async_impl};
    let _ = async_impl();
    let foo = Foo(String::new());
    assert_eq!(foo.async_method(), "async_method for Foo");
}
