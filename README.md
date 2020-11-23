<!-- cargo-sync-readme start -->


# amphi
**Why bother writing similar code twice for blocking and async code?**

amphi is an English prefix meaning `both`. This crate provides macro `amphi` to get
blocking code aside async implementation for free.

[![Build Status](https://github.com/fMeow/amphi/workflows/CI%20%28Linux%29/badge.svg?branch=master)](https://github.com/fMeow/amphi/actions)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Latest Version](https://img.shields.io/crates/v/amphi.svg)](https://crates.io/crates/amphi)
[![amphi](https://docs.rs/amphi/badge.svg)](https://docs.rs/amphi)

When implementing both sync and async versions of API in a crate, most API
of the two version are almost the same except for some async/await keyword.

Write async code once and get blocking code for free with `amphi`.

# How to use
1. place all your async code in a mod. By default, the mod should call `amphi`,
but it can be customize.
2. apply `amphi` attribute macro on the mod declaration code.

# LICENSE
MIT

<!-- cargo-sync-readme end -->

