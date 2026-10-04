---
name: writing-unsafe-rust
description: Use when writing, changing or reviewing unsafe Rust, whether an `unsafe` block, `unsafe fn`, `unsafe trait` or `unsafe impl Send` or `Sync`; a raw pointer, `NonNull`, a pointer cast or an address; `UnsafeCell`, `ManuallyDrop`, `MaybeUninit`, `mem::zeroed`, `transmute` or `Pin::new_unchecked`; FFI and `extern` blocks; atomics, memory orderings, fences, `static mut` or a lock-free structure; when a `// SAFETY:`, `// INVARIANT:` or `// ORDERING:` comment or a `# Safety` section must be written; when a loom model is due. Covers where unsafe code goes, what a safety proof must establish, pointer provenance, atomic orderings, and how unsafe code is tested.
---

# Writing Unsafe Rust

How unsafe code, raw pointers and atomics are written in this workspace: unsafe
only where no safe form serves, marked at the narrowest scope with its reason;
one operation a block, each with a `// SAFETY:` that proves its preconditions
from facts in scope; a caller's obligations written as `# Safety`; the
invariants a proof rests on written on their fields; pointers that keep their
provenance; and atomics whose every ordering names what it pairs with. The rules
below are the whole of it, each with its reason. The references hold each rule's
why, a bad and a good example that compile under the workspace's lints, and what
holds the rule. The rest of Rust, errors, names, layout and lints, is
`writing-rust`'s.

The examples leave most docs out to stay short, and compile with the lints that
ask for docs off; real code writes them, and a `# Safety` section on every
`unsafe fn` and `unsafe trait`, private ones included. Each example is small
enough that a safe form would serve it; it shows the shape a proof takes where
none does.

Under `strict`, real code also documents every item.

## Rules

### Where Unsafe Goes

1. **Reach for a safe form first**, `split_at_mut`, `get`, iterators,
   `array::from_fn`, `from_le_bytes`, since every unsafe block is a proof each
   later change must keep, and a bounds check is traded only once it is
   measured.
2. **Mark each unsafe site with `#[expect(unsafe_code, reason = "…")]` at the
   narrowest scope**: the statement, the item, the `impl` or the module, and the
   crate only where unsafe is its whole purpose, so a reader finds every site
   and why. The reason names the operation and why no safe form serves.
3. **A safe function is sound for every input**, so a raw pointer, index or
   length that decides soundness is checked, or the function is an `unsafe fn`;
   a comment that the caller passes a valid pointer proves nothing.
4. **An `unsafe fn` lists in `# Safety` each obligation its caller keeps, and
   its body still proves each operation in a block of its own**; beside a safe
   twin it is `_unchecked`, and the twin checks and calls it.
5. **Unsafe code trusts only what its module controls**: private fields and the
   module's own code, never a safe trait's impl, a caller's closure or a
   destructor, since each may be wrong without unsafe; a trait unsafe code must
   trust is an `unsafe trait`.

### Proofs

1. **One unsafe operation a block**, so each `// SAFETY:` proves one thing;
   `ptr.add(offset).read()` is two, and `add` lands in bounds even where nothing
   is read.
2. **A `// SAFETY:` proves each precondition the operation's `# Safety` lists,
   from a fact in scope**: a check above, a field's invariant, the caller's
   contract, the step before. Restating the operation proves nothing.
3. **A field a proof relies on is private, with an `// INVARIANT:` where it is
   declared, naming its writers**, since every writer keeps it, safe ones
   included; each `// SAFETY:` that relies on it cites the field INVARIANT.
4. **Every `unsafe impl` proves its trait, and `Send` and `Sync` carry the
   bounds its access needs**: `Send` needs `T: Send`, and `T: Sync` too where
   the `T` is shared, as in an `Arc`; `Sync` needs `T: Sync` where it lends `&T`
   to several threads at once, `T: Send` where it hands out `&mut T` or a `T`,
   as a lock does, and both where it does both. A type that owns `T` through a
   pointer holds `PhantomData<T>`.
5. **A marker field keeps a type on one thread**: `PhantomData<*const ()>` takes
   both traits away and `PhantomData<Cell<()>>` takes `Sync` alone, since a doc
   binds no caller and `impl !Sync` needs nightly's `negative_impls`.
6. **`debug_assert!` checks an `unsafe fn`'s contract, with a message naming the
   violation, and proves nothing**: a build without debug assertions skips it,
   so a safe function never rests on one.
7. **A panic midway leaves nothing a drop would misread**: write, then count;
   allocate before freeing; a guard counts what a partial build wrote.
8. **A value moved out with `ptr::read` is not dropped again where it lay**:
   `ManuallyDrop::new(self)` first, or `mem::take` with no unsafe.

### Pointers

1. **An address is `addr()`, never `as usize`**, which exposes the provenance;
   `map_addr` changes an address and keeps it, and `without_provenance` makes a
   pointer that is never dereferenced.
2. **A pointer comes from one whose provenance covers the place it reaches**:
   one from `&squares[3]` may reach that square alone, which Rust has not
   settled, so a read past it counts as undefined; reach around a handle with
   `base.with_addr(handle.addr())`.
3. **Exposed provenance, `expose_provenance` and `with_exposed_provenance`, only
   for an address from outside the program**, a device register or a foreign
   interface's integer; inside it, a pointer stays a pointer.
4. **Cast with `cast`, `cast_mut`, `ptr::from_ref` and `&raw`, never `as`, and
   write only through a pointer from a `&mut` or an owner**, since `as` hides
   which of type, mutability and provenance it changed.
5. **A read is aligned for its type**: bytes read as a wider value go through
   `from_le_bytes` or `read_unaligned`, since a misaligned read is undefined
   wherever it runs.
6. **A reference made from a pointer holds for all of its lifetime**, aligned,
   initialized and unaliased as its kind demands, with a lifetime from a borrow
   the signature shows, or a `# Safety` that names it, and a `&mut` from a
   `&mut`, an owner, or an `UnsafeCell` whose exclusive access the module
   proves, as a lock's guard does.
7. **No `transmute`: name the conversion**, `from_le_bytes`, `from_bits`,
   `cast`, a `TryFrom`, since a transmute checks only sizes; one that remains
   names both types and proves the rest.
8. **A type read as another has a `repr` that says so**, `#[repr(transparent)]`
   or `#[repr(C)]`, since Rust's own layout is its choice.
9. **Uninitialized memory is `MaybeUninit`, read only once it is whole**, since
   unwritten memory is no value of any type, a `u8` included; `mem::zeroed` only
   where all-zero is a valid value.
10. **Pin with `pin!` or `Box::pin`**; `Pin::new_unchecked`, `get_unchecked_mut`
    or a projection proves the value never moves again, its `Drop` included, and
    names a structurally pinned field in an `// INVARIANT:`; `PhantomPinned`
    keeps a type that must not move from being `Unpin`.

### FFI

1. **An `extern` block is `unsafe extern`, an item `safe` only when no argument
   can make it unsound, and a wrapper passes pointers it holds for the call**, a
   `CString` bound to a name; an exported symbol is `#[unsafe(no_mangle)]`.
2. **A C enum arrives as its integer and becomes a Rust enum through
   `TryFrom`**, since an enum with no variant for its value is undefined.
3. **A foreign `(ptr, 0)` becomes `&[]` before `slice::from_raw_parts`**, which
   takes no null pointer, even for no elements.
4. **A callback handed to C catches its panics and returns a code**, since a
   panic out of `extern "C"` aborts; `"C-unwind"` only where both sides unwind,
   and a foreign exception through a `"C"` import is undefined.

### Atomics

1. **Shared state is a lock, a `OnceLock` or an atomic, never `static mut`**,
   and an atomic protocol only where a lock is measured too slow, since a lock's
   proof is the standard library's.
2. **A crate with an atomic protocol takes its atomics and cells from one module
   of its own**, which swaps in loom's under `--cfg loom`, so the code and its
   model are one body; a lone `Relaxed` flag or statistic needs none.
3. **Every atomic operation has an `// ORDERING:` naming its ordering and what
   it pairs with**, a `Release` store with the `Acquire` loads that read it, or
   "Relaxed throughout" once where a function shares one.
4. **`Relaxed` where nothing pairs**, a flag or statistic that publishes only
   itself, since a stronger ordering claims a pairing that does not exist.
5. **`SeqCst` only as a fence between a store and a load a protocol rests on**,
   with an `// ORDERING:` paragraph saying why, since `Release` and `Acquire`
   never order a store before a later load; loom models the fence.
6. **A `compare_exchange` chooses both orderings**, the failure one a load's,
   `Relaxed` or `Acquire`; `compare_exchange_weak` in a loop.
7. **A pointer shared across threads is an `AtomicPtr`**, never an
   `AtomicUsize`, which drops the provenance.
8. **A node another thread may read is freed only through a reclamation scheme,
   epochs or hazard pointers, or once the structure drops**, since freeing it
   early is a use after free, and its reused address fools a `compare_exchange`,
   the ABA problem.

### Verifying

1. **A test for each edge a proof names**: the empty row, the last square, one
   past the end, since a checker finds nothing on a path no test runs.
2. **A loom model for each atomic protocol**, `#[cfg(test)] #[cfg(loom)] mod
   model`, run with `--config`, since threads in a test see one interleaving.
3. **A compile-fail test for each misuse the types refuse**, a trybuild fixture
   with its message committed; a `Send` or `Sync` that must stay is asserted at
   compile time.

Under `strict`, the lints hold more:

- **`unsafe_code` is denied**, so no unsafe compiles without its `#[expect]`.
- **Every unsafe block and `unsafe impl` has its `// SAFETY:`**, one operation a
  block, and none sits on safe code; a safe function the crate exports has no `#
  Safety`.
- **No `as`**, pointers included, and `mem::forget` only under an `#[expect]`
  with its reason.
- **A `Send` impl over a field that is not `Send` is refused**, a bare `T`
  included; a field behind a raw pointer, as `NonNull<T>`, is held by review.

Under `nightly`, `implicit_provenance_casts` refuses an `as` cast between a
pointer and an integer, either way.

## Steps

Read each reference a step names, whole, before writing the code.

1. **Changing existing unsafe code**: read the whole module, since it is the
   boundary; find each `// INVARIANT:` the change touches and each `// SAFETY:`
   that cites it, and keep them true or rewrite them with the change.
2. **A new unsafe block**: `references/safety-comments.md`. Try the safe form;
   mark the site; one operation a block; a `// SAFETY:` for each precondition; a
   test at each edge it names.
3. **A new `unsafe fn` or `unsafe trait`**: `references/safety-comments.md` and
   `references/verifying.md`: its `# Safety`, blocks in its body, a
   `debug_assert!` of what a check can see, a safe twin where one fits, and a
   test that pins it stays unsafe.
4. **A type over raw pointers or an `UnsafeCell`, or an `unsafe impl Send` or
   `Sync`**: `references/safety-comments.md`, `references/pointers.md`,
   `references/atomics.md` for a lock, and `references/verifying.md`: private
   fields with their `// INVARIANT:`, `PhantomData<T>`, impls bounded as their
   access needs, with proofs, a marker for a type bound to one thread, a
   compile-fail test.
5. **Pointer arithmetic, casts, alignment, layout, uninitialized memory,
   `transmute` or `Pin`**: `references/pointers.md`, whole.
6. **FFI**: `references/pointers.md`: an `unsafe extern` block, a safe wrapper
   that proves each call, C enums as integers, empty slices before null,
   callbacks that catch their panics, `core::ffi` types.
7. **Shared state, a lock, an atomic protocol or a lock-free structure**:
   `references/atomics.md`, whole, and `references/verifying.md` for its loom
   model.
8. **Before finishing**: `references/verifying.md`, then the checks below, until
   they pass.

How a `// SAFETY:`, `// INVARIANT:` or `// ORDERING:` comment and a `# Safety`
section are worded, and where each sits, is `writing-rustdoc`'s; this skill says
what each must prove.

## Checks

- `just check`: every check, as CI runs them, after `just fix`.
- `just check-rust-clippy`: clippy on every crate, target and feature, with
  warnings denied, which holds the pointer, transmute and uninitialized-memory
  lints.
- `just check-rust-lints`: nightly's lints, `implicit_provenance_casts` among
  them.
- A crate's loom models, which no recipe runs:

  ```sh
  cargo test -p <crate> --lib --release --config 'target."cfg(all())".rustflags=["--cfg","loom"]'
  ```

## What Not to Do

| Thought                                                     | Instead                                                                           |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------- |
| "One `unsafe` block around the loop is tidier"              | One operation a block, each with its proof.                                       |
| "`// SAFETY: the pointer is valid`"                         | Each precondition the operation lists, and the fact that meets it.                |
| "The caller will pass a valid pointer"                      | An `unsafe fn` with `# Safety`, or a reference and a check.                       |
| "The `debug_assert!` checks the index"                      | A real check in a safe function, or an `unsafe fn`.                               |
| "`len()` said the tiles fit"                                | A bound the module holds; a safe trait's impl may lie.                            |
| "`unsafe impl<T> Send`, the pointer is ours"                | `unsafe impl<T: Send> Send`, with the proof above it.                             |
| "`T: Sync` is enough for the lock to be `Sync`"             | `T: Send`: a lock lends `&mut T`, through which a `T` moves.                      |
| "`#![expect(unsafe_code)]` on the crate"                    | On the statement or item that holds the unsafe.                                   |
| "`ptr as usize`, then back"                                 | `addr()`, and `with_addr` from a pointer that has the provenance.                 |
| "`as *mut u8` is clearer than `cast_mut`"                   | `cast_mut`, from a pointer that may be written.                                   |
| "The address is right, so the read is fine"                 | The provenance must cover it: rebuild from the owner's pointer.                   |
| "Read the `u32` straight from the bytes"                    | `from_le_bytes` on a copied chunk, or `read_unaligned`.                           |
| "`ptr::read` the field out of `self`"                       | `ManuallyDrop::new(self)` first, or `mem::take`.                                  |
| "`transmute` is shortest"                                   | `from_le_bytes`, `from_bits`, `cast`, `TryFrom`.                                  |
| "A `u8` has no invalid bit pattern, so `assume_init` early" | Write every slot first: unwritten memory is no `u8`.                              |
| "`static mut` for the global"                               | `OnceLock`, a `Mutex`, or an atomic.                                              |
| "`SeqCst`, to be safe"                                      | The ordering that pairs; `SeqCst` only as a store-load fence, with its paragraph. |
| "Free the node once it is unlinked"                         | A reclamation scheme, or not before the structure drops.                          |
| "The threaded test passes a thousand times"                 | A loom model, which runs every interleaving.                                      |

Under `strict`, also:

| Thought                                 | Instead                                            |
| --------------------------------------- | -------------------------------------------------- |
| "`// SAFETY:` above this safe call too" | Only above unsafe code; the lint refuses the rest. |

## References

Read every reference a task touches before writing code, and read them again
after compaction: this body is the summary, and the examples are there.

- `references/safety-comments.md`: before an `unsafe` block, `unsafe fn`,
  `unsafe impl` or `unsafe trait`, a field unsafe code relies on, a guard, a
  `ptr::read`, or a change to a module that holds unsafe code.
- `references/pointers.md`: before a raw pointer is made, cast, offset or read,
  an address, alignment or layout, `transmute`, `MaybeUninit`, `mem::zeroed`,
  `Pin`, an `extern` block, a C enum, a foreign slice or a callback.
- `references/atomics.md`: before an atomic, a fence, a shared `static`, a lock
  over an `UnsafeCell`, a lock-free structure, or state shared between threads
  outside a lock.
- `references/verifying.md`: before a test of unsafe code, a loom model or a
  compile-fail test, and before calling unsafe code done.
- `references/sources.md`: before citing a source for a rule, or adapting one.
