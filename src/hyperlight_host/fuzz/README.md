# Fuzzing Hyperlight

This directory contains the fuzzing infrastructure for Hyperlight. We use `cargo-fuzz` to run the fuzzers - i.e., small programs that run specific tests with semi-random inputs to find bugs. Because `cargo-fuzz` is not yet stable, we use the nightly toolchain. Also, because `cargo-fuzz` doesn't support Windows, we have to run this WSL or Linux (Mariner/Ubuntu).

You can run the fuzzers with:
```sh
cargo +nightly-2023-11-28-x86_64-unknown-linux-gnu fuzz run --release <fuzzer_name>
```

> Note: Because nightly toolchains are not stable, we pin the nightly version to `2023-11-28`. To install this toolchain, run:
> ```sh
> rustup toolchain install nightly-2023-11-28-x86_64-unknown-linux-gnu
> ```

As per Microsoft's Offensive Research & Security Engineering (MORSE) team, all host exposed functions that receive or interact with guest data must be continuously fuzzed for, at least, 500 million fuzz test cases without any crashes. Because `cargo-fuzz` doesn't support setting a maximum number of iterations; instead, we use the `--max_total_time` flag to set a maximum time to run the fuzzer. We have a GitHub action (acting like a CRON job) that runs the fuzzers for 24 hours every week.

Currently, we only fuzz the `PrintOutput` function. We plan to add more fuzzers in the future.

If you encounter a failure, you can run a single test case with:
```sh
cargo +nightly-2023-11-28-x86_64-unknown-linux-gnu fuzz run --release <fuzzer_name> -- -seed=<seed-number>
```

The seed number can be seed in a specific run, like:
![fuzz-seed](doc-assets/image.png)