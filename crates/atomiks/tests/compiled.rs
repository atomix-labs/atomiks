//! The tests that run the compiler: what a crate that derives sees.

// Neither loom's model nor Miri's interpreter can run the compiler.
#![cfg(not(any(loom, miri)))]

// Every fixture derives.
#[cfg(feature = "derive")]
#[cfg(test)]
mod trybuild {
    //! What a crate that derives sees: what compiles under strict lints, with no feature gate for
    //! a type without parameters, and each refusal with its message.

    #[test]
    fn user_crates_compile_or_are_refused_with_the_message() {
        let cases = trybuild::TestCases::new();
        // With a pass case, trybuild builds every fixture rather than checking it, so the
        // post-monomorphization errors show.
        cases.pass("tests/compile_pass/*.rs");
        cases.compile_fail("tests/compile_fail/*.rs");
    }
}
