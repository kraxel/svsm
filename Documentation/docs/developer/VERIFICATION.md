# VERIFICATION

Formal verification is done via [Verus](https://github.com/verus-lang/verus). 
To execute verification, ensure you have set up the necessary tools to run
`cargo verify`.

## Setup

Run the following commands to install Verus tools.

```
cd svsm
./scripts/vinstall.sh
```

> Why am I using a different Rust toolchain?
>> Verus requires a specific version of Rust (e.g., 1.88.0) because it depends
  on the compiler’s internal libraries for verification. Thanks to Rust’s strong
  backward compatibility guarantees, Verus can analyze code written against
  older versions. However, some newer Verus features depend on recent rustc
  versions, so we may occasionally upgrade the toolchain to support those
  features, which does not guarantee it is the exact version defined by
  `rust-toolchain.toml`

## Build

### Build svsm with verification

```
cd svsm/kernel
cargo verify
```

By default, it only verifies the current crate (`cargo verify` is an alias of `cargo v --features verus`), while using spec/proof from external crates. To verify all external crates, run `cargo v --features verusall`



### Pass verus arguments for verification

For debugging purposes, it is helpful to pass additional Verus arguments.
You can specify extra arguments using the environmental variable
{crate}_{lib/bin}_VERUS_ARGS for a specific crate
{crate} or VERUS_ARGS for all crates.

**Examples**

* Compiles a crate without verifying svsm crate using verus compilation:

    ```
    svsm_lib_VERUS_ARGS="--no-verify" cargo verify
    ```

* Compiles a crate while only verifying address module in svsm crate:

    ```
    svsm_lib_VERUS_ARGS="--verify-module address --verify-function VirtAddr::new" cargo verify
    ```

### Build without verification

```
cd svsm/kernel
cargo build
```

All Verus-related annotations, specifications, and proofs are ignored.

## Verification Plan

- [x] Set up verification as an experimental development.
- [ ] Verify SVSM kernel protocol (similar to [VeriSMo](https://github.com/microsoft/verismo))
  - [ ] Allocator
  - [ ] Page table
  - [ ] Protocol
- [ ] Verifying other security-critical components.

## Verification Development

To enable verification, developers need to add annotations in executable Rust
and to write specification and proofs in ghost mode.


### Development Guidelines

* Code Collaboration: Unverified and verified code can co-exist. See [SVSM VeriSMo meeting](https://github.com/coconut-svsm/governance/blob/main/Meetings/Data/verismo-10-23-2024-talk.pdf) for more details.
* Minimize Annotations: Keep simple annotations in executable Rust, by defining
  complicated specifications in spec functions and group proofs.
* Specification and Proof Structure: For each module `x`, define code-related
  specification and proof in `x.verus.rs` .
* Code Formatting: Use `cargo fmt` for excutable Rust. Codes wrapped in verus!{}
  macro could be formatted via verusfmt. Run `./script/vfmt.sh` to format
  `*.verus.rs`
* Reusing spec/proof: Use external specification and proofs from
  [vstd](https://verus-lang.github.io/verus/verusdoc/vstd/) when possible.
* Specifications for dependencies (external crates): If functions, structs, or
  traits from external crates lack specifications from vstd, define their
  specifications in `verify_external/`.
* Performance: Store expensive and reusable proofs in `verify_proof/` if not
  provided by `vstd`. The `svsm/kernel/build.rs` sets `rlimit=1`, while
  `verify_proof/build.rs` sets `rlimit=4`, helping developers decide when they
  need more proof engineering to run verification within minutes. For some proofs that
  are tightly related to the project code, we may still put them under svsm/kernel with
  `#[verus_verify(rlimit=x)]`

### Annotation in Executable Rust
* #[verus_verify]: Indicates the item is Verus-aware.
* #[verus_verify(external_body)]: Indicates the item is Verus-aware, but marks the function body as uninterpreted by the verifier.
* #[verus_verify(external)]: Instructs Verus to ignore the item. By default, items are treated as #[verus_verify(external)].
* #[verus_spec($specificaton)]: Specifies pre/postconditions for executable codes.
  $specificaton is in format of
  ```
  => $ret
  requires precondition($inputs),
  ensures postcondition($inputs, $ret),
  returns ret_spec,
  decreases termination_condition
  ```
* proof!{...}: Inserts proofs to help solver to avoid false positives or improve
  performance. You can also add assert(..) inside proof macro to statically
  check assertions.
* More annotations usage can be found in https://github.com/verus-lang/verus/tree/main/source/rust_verify_test/tests/syntax_attr.rs


For example,

```rust

use vstd::prelude::*;
#[verus_verify]
trait A: Sized {
  #[verus_spec(ret => requires(m > 0), ensures 0 <= ret < m)]
  fn op(self, m: usize) -> usize;
}

#[verus_verify]
impl A for usize {
  // Failed postcondition.
  fn op(self, m: usize) -> usize {
    self
  }
}

#[verus_verify]
impl A for u64 {
  // Verified.
  fn op(self, m: usize) -> usize {
    (self % (m as u64)) as usize
  }
}
```

### Developing specification and proof (Verification developers)

While Verus allows you to write specifications and proofs in Rust, it's
beneficial to use the verus!{} macro for a more concise, mathematical syntax
similar to Dafny, F*, and Coq. To get started, be sure to read the [Verus
Tutorial](https://verus-lang.github.io/verus/guide/overview.html). You can use [Verus playground](https://play.verus-lang.org/) to play with verus without install it. To find
examples about recursive proof, quantifier, traits, pointers, type invariant, or
other advanced usage, please refer to [Verus
Tests](https://github.com/verus-lang/verus/tree/main/source/rust_verify_test/tests)
and [Verus
Examples](https://github.com/verus-lang/verus/tree/main/source/rust_verify/example).
