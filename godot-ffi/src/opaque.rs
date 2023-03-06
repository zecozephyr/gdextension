/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: transmute not supported for const generics; see
// https://users.rust-lang.org/t/transmute-in-the-context-of-constant-generics/56827

/// Stores an opaque object of a certain size, with very restricted operations
///
/// Note: due to `align(8)` and not `packed` repr, this type may be bigger than `N` bytes
/// (which should be OK since C++ just needs to read/write those `N` bytes reliably).
// XXX: I suspect this is only align(8) since Opaque pointers from the engine
// need to be pointer aligned? or something. At the very least it is being
// transmuted to a pointer at some *point*
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[derive(Copy, Clone)]
pub struct Opaque<const N: usize> {
    storage: [u8; N],
    marker: std::marker::PhantomData<*const u8>, // disable Send/Sync
}
