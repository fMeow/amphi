<!-- cargo-sync-readme start -->


# Amphisbaena
**Why bother writing similar code twice for blocking and async code?**

[![Build Status](https://github.com/fMeow/amphisbaena/workflows/CI%20%28Linux%29/badge.svg?branch=master)](https://github.com/fMeow/amphisbaena/actions)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Latest Version](https://img.shields.io/crates/v/amphisbaena.svg)](https://crates.io/crates/amphisbaena)
[![amphisbaena](https://docs.rs/amphisbaena/badge.svg)](https://docs.rs/amphisbaena)

When implementing both sync and async versions of API in a crate, most API
of the two version are almost the same except for some async/await keyword.

Write async code once and get blocking code for free with `amphisbaena`.

# How to use
1. place all your async code in a mod. By default, the mod should call `amphisbaena`,
but it can be customize.
2. apply `amphisbaena` attribute macro on the mod declaration code.

# LICENSE
MIT

<!-- cargo-sync-readme end -->

