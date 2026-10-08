# `atomix-derive-impl`

The logic of the derives of [atomix][project]: it reads a type's definition, and
writes its impls or the errors that refuse it. It is a library of its own so
that its tests run without a compiler's proc-macro context; `atomix-derive`
calls it. To derive `Atom`, depend on `atomix-rs` with its `derive` feature, and
not on this crate:

```sh
cargo add --git https://github.com/atomix-labs/atomix atomix-rs --features derive
```

## License

Either [the MIT License][mit] or [the Apache License, Version 2.0][apache], at
your option.

[project]: https://github.com/atomix-labs/atomix
[mit]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-MIT
[apache]: https://github.com/atomix-labs/atomix/blob/main/LICENSE-APACHE
