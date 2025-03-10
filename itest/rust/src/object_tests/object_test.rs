/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::bind::{godot_api, GodotClass};
use godot::builtin::meta::{FromGodot, ToGodot};
use godot::builtin::{GString, StringName, Variant, Vector3};
use godot::engine::{
    file_access, Area2D, Camera3D, FileAccess, IRefCounted, Node, Node3D, Object, RefCounted,
};
use godot::obj::{Base, Gd, Inherits, InstanceId, RawGd, UserClass};
use godot::prelude::meta::GodotType;
use godot::sys::{self, GodotFfi};

use crate::framework::{expect_panic, itest, TestContext};

// TODO:
// * make sure that ptrcalls are used when possible (ie. when type info available; maybe GDScript integration test)
// * Deref impl for user-defined types

#[itest]
fn object_construct_default() {
    let obj = Gd::<RefcPayload>::default();
    assert_eq!(obj.bind().value, 111);
}

#[itest]
fn object_construct_new_gd() {
    let obj = RefcPayload::new_gd();
    assert_eq!(obj.bind().value, 111);
}

#[itest]
fn object_construct_value() {
    let obj = Gd::from_object(RefcPayload { value: 222 });
    assert_eq!(obj.bind().value, 222);
}

// TODO(#23): DerefMut on Gd pointer may be used to break subtyping relations
#[itest(skip)]
fn object_subtype_swap() {
    let mut a: Gd<Node> = Node::new_alloc();
    let mut b: Gd<Node3D> = Node3D::new_alloc();

    /*
    let a_id = a.instance_id();
    let b_id = b.instance_id();
    let a_class = a.get_class();
    let b_class = b.get_class();

    dbg!(a_id);
    dbg!(b_id);
    dbg!(&a_class);
    dbg!(&b_class);
    println!("..swap..");
    */

    std::mem::swap(&mut *a, &mut *b);

    /*
    dbg!(a_id);
    dbg!(b_id);
    dbg!(&a_class);
    dbg!(&b_class);
    */

    // This should not panic
    a.free();
    b.free();
}

#[itest]
fn object_user_roundtrip_return() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    assert_eq!(obj.bind().value, value);

    let raw = obj.to_ffi();
    let ptr = raw.sys();
    std::mem::forget(obj);

    let raw2 = unsafe { RawGd::<RefcPayload>::from_sys(ptr) };
    let obj2 = Gd::from_ffi(raw2);
    assert_eq!(obj2.bind().value, value);
} // drop

#[itest]
fn object_user_roundtrip_write() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    assert_eq!(obj.bind().value, value);
    let raw = obj.to_ffi();

    let raw2 = unsafe {
        RawGd::<RefcPayload>::from_sys_init(|ptr| {
            raw.move_return_ptr(sys::AsUninit::force_init(ptr), sys::PtrcallType::Standard)
        })
    };
    let obj2 = Gd::from_ffi(raw2);
    assert_eq!(obj2.bind().value, value);
} // drop

#[itest]
fn object_engine_roundtrip() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    let raw = obj.to_ffi();
    let ptr = raw.sys();

    let raw2 = unsafe { RawGd::<Node3D>::from_sys(ptr) };
    let obj2 = Gd::from_ffi(raw2);
    assert_eq!(obj2.get_position(), pos);
    obj.free();
}

#[itest]
fn object_user_display() {
    let obj = Gd::from_object(RefcPayload { value: 774 });

    let actual = format!(".:{obj}:.");
    let expected = ".:value=774:.".to_string();

    assert_eq!(actual, expected);
}

#[itest]
fn object_engine_display() {
    let obj = Node3D::new_alloc();
    let id = obj.instance_id();

    let actual = format!(".:{obj}:.");
    let expected = format!(".:<Node3D#{id}>:.");

    assert_eq!(actual, expected);
    obj.free();
}

#[itest]
fn object_debug() {
    let obj = Node3D::new_alloc();
    let id = obj.instance_id();

    let actual = format!(".:{obj:?}:.");
    let expected = format!(".:Gd {{ id: {id}, class: Node3D }}:.");

    assert_eq!(actual, expected);
    obj.free();
}

#[itest]
fn object_instance_id() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    let id = obj.instance_id();

    let obj2 = Gd::<RefcPayload>::from_instance_id(id);
    assert_eq!(obj2.bind().value, value);
}

#[itest]
fn object_instance_id_when_freed() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    assert!(node.is_instance_valid());

    node.clone().free(); // destroys object without moving out of reference
    assert!(!node.is_instance_valid());

    expect_panic("instance_id() on dead object", move || {
        node.instance_id();
    });
}

#[itest]
fn object_from_invalid_instance_id() {
    let id = InstanceId::try_from_i64(0xDEADBEEF).unwrap();

    Gd::<RefcPayload>::try_from_instance_id(id)
        .expect_err("invalid instance id should not return a valid object");
}

#[itest]
fn object_from_instance_id_inherits_type() {
    let descr = GString::from("some very long description");

    let mut node: Gd<Node3D> = Node3D::new_alloc();
    node.set_editor_description(descr.clone());

    let id = node.instance_id();

    let node_as_base = Gd::<Node>::from_instance_id(id);
    assert_eq!(node_as_base.instance_id(), id);
    assert_eq!(node_as_base.get_editor_description(), descr);

    node_as_base.free();
}

#[itest]
fn object_from_instance_id_unrelated_type() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    let id = node.instance_id();

    Gd::<RefCounted>::try_from_instance_id(id)
        .expect_err("try_from_instance_id() with bad type should fail");

    node.free();
}

#[itest]
fn object_new_has_instance_id() {
    let obj = ObjPayload::alloc_gd();
    let _id = obj.instance_id();
    obj.free();
}

#[itest]
fn object_dynamic_free() {
    let mut obj = ObjPayload::alloc_gd();
    let id = obj.instance_id();

    obj.call("free".into(), &[]);

    Gd::<ObjPayload>::try_from_instance_id(id)
        .expect_err("dynamic free() call must destroy object");
}

#[itest]
fn object_user_bind_after_free() {
    let obj = Gd::from_object(ObjPayload {});
    let copy = obj.clone();
    obj.free();

    expect_panic("bind() on dead user object", move || {
        let _ = copy.bind();
    });
}

#[itest]
fn object_user_free_during_bind() {
    let obj = Gd::from_object(ObjPayload {});
    let guard = obj.bind();

    let copy = obj.clone(); // TODO clone allowed while bound?

    expect_panic("direct free() on user while it's bound", move || {
        copy.free();
    });

    drop(guard);
    assert!(
        obj.is_instance_valid(),
        "object lives on after failed free()"
    );
    obj.free(); // now succeeds
}

#[itest(skip)] // This deliberately crashes the engine. Un-skip to manually test this.
fn object_user_dynamic_free_during_bind() {
    // Note: we could also test if GDScript can access free() when an object is bound, to check whether the panic is handled or crashes
    // the engine. However, that is only possible under the following scenarios:
    // 1. Multithreading -- needs to be outlawed on Gd<T> in general, anyway. If we allow a thread-safe Gd<T>, we however need to handle that.
    // 2. Re-entrant calls -- Rust binds a Gd<T>, calls GDScript, which frees the same Gd. This is the same as the test here.
    // 3. Holding a guard (GdRef/GdMut) across function calls -- not possible, guard's lifetime is coupled to a Gd and cannot be stored in
    //    fields or global variables due to that.

    let obj = Gd::from_object(ObjPayload {});
    let guard = obj.bind();

    let mut copy = obj.clone(); // TODO clone allowed while bound?

    // This technically triggers UB, but in practice no one accesses the references.
    // There is no alternative to test this, see destroy_storage() comments.
    copy.call("free".into(), &[]);

    drop(guard);
    assert!(
        !obj.is_instance_valid(),
        "dynamic free() destroys object even if it's bound"
    );
}

// TODO test if engine destroys it, eg. call()

#[itest]
fn object_user_call_after_free() {
    let obj = Gd::from_object(ObjPayload {});
    let mut copy = obj.clone();
    obj.free();

    expect_panic("call() on dead user object", move || {
        let _ = copy.call("get_instance_id".into(), &[]);
    });
}

#[itest]
fn object_engine_use_after_free() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    let copy = node.clone();
    node.free();

    expect_panic("call method on dead engine object", move || {
        copy.get_position();
    });
}

#[itest]
fn object_engine_use_after_free_varcall() {
    let node: Gd<Node3D> = Node3D::new_alloc();
    let mut copy = node.clone();
    node.free();

    expect_panic("call method on dead engine object", move || {
        copy.call_deferred("get_position".into(), &[]);
    });
}

#[itest]
fn object_user_eq() {
    let value: i16 = 17943;
    let a = RefcPayload { value };
    let b = RefcPayload { value };

    let a1 = Gd::from_object(a);
    let a2 = a1.clone();
    let b1 = Gd::from_object(b);

    assert_eq!(a1, a2);
    assert_ne!(a1, b1);
    assert_ne!(a2, b1);
}

#[itest]
fn object_engine_eq() {
    let a1 = Node3D::new_alloc();
    let a2 = a1.clone();
    let b1 = Node3D::new_alloc();

    assert_eq!(a1, a2);
    assert_ne!(a1, b1);
    assert_ne!(a2, b1);

    a1.free();
    b1.free();
}

#[itest]
fn object_dead_eq() {
    let a = Node3D::new_alloc();
    let b = Node3D::new_alloc();
    let b2 = b.clone();

    // Destroy b1 without consuming it
    b.clone().free();

    {
        let lhs = a.clone();
        expect_panic("Gd::eq() panics when one operand is dead", move || {
            let _ = lhs == b;
        });
    }
    {
        let rhs = a.clone();
        expect_panic("Gd::ne() panics when one operand is dead", move || {
            let _ = b2 != rhs;
        });
    }

    a.free();
}

#[itest]
fn object_user_convert_variant() {
    let value: i16 = 17943;
    let user = RefcPayload { value };

    let obj: Gd<RefcPayload> = Gd::from_object(user);
    let variant = obj.to_variant();
    let obj2 = Gd::<RefcPayload>::from_variant(&variant);

    assert_eq!(obj2.bind().value, value);
}

#[itest]
fn object_engine_convert_variant() {
    let pos = Vector3::new(1.0, 2.0, 3.0);

    let mut obj: Gd<Node3D> = Node3D::new_alloc();
    obj.set_position(pos);

    let variant = obj.to_variant();
    let obj2 = Gd::<Node3D>::from_variant(&variant);

    assert_eq!(obj2.get_position(), pos);
    obj.free();
}

#[itest]
fn object_user_convert_variant_refcount() {
    let obj: Gd<RefcPayload> = Gd::from_object(RefcPayload { value: -22222 });
    let obj = obj.upcast::<RefCounted>();
    check_convert_variant_refcount(obj)
}

#[itest]
fn object_engine_convert_variant_refcount() {
    let obj = RefCounted::new();
    check_convert_variant_refcount(obj);
}

/// Converts between Object <-> Variant and verifies the reference counter at each stage.
fn check_convert_variant_refcount(obj: Gd<RefCounted>) {
    // Freshly created -> refcount 1
    assert_eq!(obj.get_reference_count(), 1);

    {
        // Variant created from object -> increment
        let variant = obj.to_variant();
        assert_eq!(obj.get_reference_count(), 2);

        {
            // Yet another object created *from* variant -> increment
            let another_object = variant.to::<Gd<RefCounted>>();
            assert_eq!(obj.get_reference_count(), 3);
            assert_eq!(another_object.get_reference_count(), 3);
        }

        // `another_object` destroyed -> decrement
        assert_eq!(obj.get_reference_count(), 2);
    }

    // `variant` destroyed -> decrement
    assert_eq!(obj.get_reference_count(), 1);
}

#[itest]
fn object_engine_convert_variant_nil() {
    let nil = Variant::nil();

    Gd::<Area2D>::try_from_variant(&nil).expect_err("`nil` should not convert to `Gd<Area2D>`");

    expect_panic("from_variant(&nil)", || {
        Gd::<Area2D>::from_variant(&nil);
    });
}

#[itest]
fn object_engine_returned_refcount() {
    let Some(file) = FileAccess::open(
        "res://itest.gdextension".into(),
        file_access::ModeFlags::READ,
    ) else {
        panic!("failed to open file used to test FileAccess")
    };
    assert!(file.is_open());

    // There was a bug which incremented ref-counts of just-returned objects, keep this as regression test.
    assert_eq!(file.get_reference_count(), 1);
}

#[itest]
fn object_engine_up_deref() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    // Deref chain: Gd<Node3D> -> &Node3D -> &Node -> &Object
    assert_eq!(node3d.instance_id(), id);
    assert_eq!(node3d.get_class(), GString::from("Node3D"));

    node3d.free();
}

#[itest]
fn object_engine_up_deref_mut() {
    let mut node3d: Gd<Node3D> = Node3D::new_alloc();

    // DerefMut chain: Gd<Node3D> -> &mut Node3D -> &mut Node -> &mut Object
    node3d.set_message_translation(true);
    assert!(node3d.can_translate_messages());

    // DerefMut chain: &mut Node3D -> ...
    let node3d_ref = &mut *node3d;
    node3d_ref.set_message_translation(false);
    assert!(!node3d_ref.can_translate_messages());

    node3d.free();
}

#[itest]
fn object_engine_upcast() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GString::from("Node3D"));

    // Deliberate free on upcast object
    object.free();
}

#[itest]
fn object_engine_upcast_reflexive() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let object = node3d.upcast::<Node3D>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GString::from("Node3D"));

    object.free();
}

#[itest]
fn object_engine_downcast() {
    let pos = Vector3::new(1.0, 2.0, 3.0);
    let mut node3d: Gd<Node3D> = Node3D::new_alloc();
    node3d.set_position(pos);
    let id = node3d.instance_id();

    let object = node3d.upcast::<Object>();
    let node: Gd<Node> = object.cast::<Node>();
    let node3d: Gd<Node3D> = node.try_cast::<Node3D>().expect("try_cast");

    assert_eq!(node3d.instance_id(), id);
    assert_eq!(node3d.get_position(), pos);

    node3d.free();
}

#[derive(GodotClass)]
struct CustomClassA {}

#[derive(GodotClass)]
struct CustomClassB {}

#[itest]
fn object_reject_invalid_downcast() {
    let instance = Gd::from_object(CustomClassA {});
    let object = instance.upcast::<Object>();

    assert!(object.try_cast::<CustomClassB>().is_none());
}

#[itest]
fn variant_reject_invalid_downcast() {
    let variant = Gd::from_object(CustomClassA {}).to_variant();

    assert!(variant.try_to::<Gd<CustomClassB>>().is_err());
    assert!(variant.try_to::<Gd<CustomClassA>>().is_ok());
}

#[itest]
fn object_engine_downcast_reflexive() {
    let node3d: Gd<Node3D> = Node3D::new_alloc();
    let id = node3d.instance_id();

    let node3d: Gd<Node3D> = node3d.cast::<Node3D>();
    assert_eq!(node3d.instance_id(), id);

    node3d.free();
}

#[itest]
fn object_engine_bad_downcast() {
    let object: Gd<Object> = Object::new_alloc();
    let free_ref = object.clone();
    let node3d: Option<Gd<Node3D>> = object.try_cast::<Node3D>();

    assert!(node3d.is_none());
    free_ref.free();
}

#[itest]
fn object_engine_accept_polymorphic() {
    let mut node = Camera3D::new_alloc();
    let expected_name = StringName::from("Node name");
    let expected_class = GString::from("Camera3D");

    node.set_name(GString::from(&expected_name));

    let actual_name = accept_node(node.clone());
    assert_eq!(actual_name, expected_name);

    let actual_class = accept_object(node.clone());
    assert_eq!(actual_class, expected_class);

    node.free();
}

#[itest]
fn object_user_accept_polymorphic() {
    let obj = Gd::from_object(RefcPayload { value: 123 });
    let expected_class = GString::from("RefcPayload");

    let actual_class = accept_refcounted(obj.clone());
    assert_eq!(actual_class, expected_class);

    let actual_class = accept_object(obj);
    assert_eq!(actual_class, expected_class);
}

fn accept_node<T>(node: Gd<T>) -> StringName
where
    T: Inherits<Node>,
{
    let up = node.upcast();
    up.get_name()
}

fn accept_refcounted<T>(node: Gd<T>) -> GString
where
    T: Inherits<RefCounted>,
{
    let up = node.upcast();
    up.get_class()
}

fn accept_object<T>(node: Gd<T>) -> GString
where
    T: Inherits<Object>,
{
    let up = node.upcast();
    up.get_class()
}

#[itest]
fn object_user_upcast() {
    let obj = user_refc_instance();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    assert_eq!(object.instance_id(), id);
    assert_eq!(object.get_class(), GString::from("RefcPayload"));
}

#[itest]
fn object_user_downcast() {
    let obj = user_refc_instance();
    let id = obj.instance_id();

    let object = obj.upcast::<Object>();
    let intermediate: Gd<RefCounted> = object.cast::<RefCounted>();
    assert_eq!(intermediate.instance_id(), id);

    let concrete: Gd<RefcPayload> = intermediate.try_cast::<RefcPayload>().expect("try_cast");
    assert_eq!(concrete.instance_id(), id);
    assert_eq!(concrete.bind().value, 17943);
}

#[itest]
fn object_user_bad_downcast() {
    let obj = user_refc_instance();
    let object = obj.upcast::<Object>();
    let node3d: Option<Gd<Node>> = object.try_cast::<Node>();

    assert!(node3d.is_none());
}

#[itest]
fn object_engine_manual_free() {
    // Tests if no panic or memory leak
    {
        let node = Node3D::new_alloc();
        let node2 = node.clone();
        node2.free();
    } // drop(node)
}

/// Tests the [`DynamicRefCount`] destructor when the underlying [`Object`] is already freed.
#[itest]
fn object_engine_shared_free() {
    {
        let node = Node::new_alloc();
        let _object = node.clone().upcast::<Object>();
        node.free();
    } // drop(_object)
}

#[itest]
fn object_engine_manual_double_free() {
    let node = Node3D::new_alloc();
    let node2 = node.clone();
    node.free();

    expect_panic("double free()", move || {
        node2.free();
    });
}

#[itest]
fn object_engine_refcounted_free() {
    let node = RefCounted::new();
    let node2 = node.clone().upcast::<Object>();

    expect_panic("calling free() on RefCounted object", || node2.free())
}

#[itest]
fn object_user_double_free() {
    let mut obj = ObjPayload::alloc_gd();
    let obj2 = obj.clone();
    obj.call("free".into(), &[]);

    expect_panic("double free()", move || {
        obj2.free();
    });
}

#[itest]
fn object_user_share_drop() {
    let drop_count = Rc::new(RefCell::new(0));

    let object: Gd<Tracker> = Gd::from_object(Tracker {
        drop_count: Rc::clone(&drop_count),
    });
    assert_eq!(*drop_count.borrow(), 0);

    let shared = object.clone();
    assert_eq!(*drop_count.borrow(), 0);

    drop(shared);
    assert_eq!(*drop_count.borrow(), 0);

    drop(object);
    assert_eq!(*drop_count.borrow(), 1);
}

#[itest]
fn object_call_no_args() {
    let mut node = Node3D::new_alloc().upcast::<Object>();

    let static_id = node.instance_id();
    let reflect_id_variant = node.call(StringName::from("get_instance_id"), &[]);

    let reflect_id = InstanceId::from_variant(&reflect_id_variant);

    assert_eq!(static_id, reflect_id);
    node.free();
}

#[itest]
fn object_call_with_args() {
    let mut node = Node3D::new_alloc();

    let expected_pos = Vector3::new(2.5, 6.42, -1.11);

    let none = node.call(
        StringName::from("set_position"),
        &[expected_pos.to_variant()],
    );
    let actual_pos = node.call(StringName::from("get_position"), &[]);

    assert_eq!(none, Variant::nil());
    assert_eq!(actual_pos, expected_pos.to_variant());
    node.free();
}

#[itest]
fn object_get_scene_tree(ctx: &TestContext) {
    let node = Node3D::new_alloc();

    let mut tree = ctx.scene_tree.clone();
    tree.add_child(node.upcast());

    let count = tree.get_child_count();
    assert_eq!(count, 1);
} // implicitly tested: node does not leak

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Object)]
struct ObjPayload {}

#[godot_api]
impl ObjPayload {
    #[signal]
    fn do_use();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[inline(never)] // force to move "out of scope", can trigger potential dangling pointer errors
fn user_refc_instance() -> Gd<RefcPayload> {
    let value: i16 = 17943;
    let user = RefcPayload { value };
    Gd::from_object(user)
}

#[derive(GodotClass, Eq, PartialEq, Debug)]
pub struct RefcPayload {
    value: i16,
}

#[godot_api]
impl IRefCounted for RefcPayload {
    fn init(_base: Base<Self::Base>) -> Self {
        Self { value: 111 }
    }

    fn to_string(&self) -> GString {
        format!("value={}", self.value).into()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Eq, PartialEq, Debug)]
pub struct Tracker {
    drop_count: Rc<RefCell<i32>>,
}

impl Drop for Tracker {
    fn drop(&mut self) {
        //println!("      Tracker::drop");
        *self.drop_count.borrow_mut() += 1;
    }
}

pub mod object_test_gd {
    use godot::prelude::*;

    #[derive(GodotClass)]
    #[class(init, base=Object)]
    struct MockObjRust {
        #[var]
        i: i64,
    }

    #[godot_api]
    impl MockObjRust {}

    #[derive(GodotClass)]
    #[class(init, base=RefCounted)]
    struct MockRefCountedRust {
        #[var]
        i: i64,
    }

    #[godot_api]
    impl MockRefCountedRust {}

    #[derive(GodotClass, Debug)]
    #[class(init, base=RefCounted)]
    struct ObjectTest;

    #[godot_api]
    impl ObjectTest {
        #[func]
        fn pass_object(&self, object: Gd<Object>) -> i64 {
            let i = object.get("i".into()).to();
            object.free();
            i
        }

        #[func]
        fn return_object(&self) -> Gd<Object> {
            Gd::from_object(MockObjRust { i: 42 }).upcast()
        }

        #[func]
        fn pass_refcounted(&self, object: Gd<RefCounted>) -> i64 {
            object.get("i".into()).to()
        }

        #[func]
        fn pass_refcounted_as_object(&self, object: Gd<Object>) -> i64 {
            object.get("i".into()).to()
        }

        #[func]
        fn return_refcounted(&self) -> Gd<RefCounted> {
            Gd::from_object(MockRefCountedRust { i: 42 }).upcast()
        }

        #[func]
        fn return_refcounted_as_object(&self) -> Gd<Object> {
            Gd::from_object(MockRefCountedRust { i: 42 }).upcast()
        }
    }

    // ----------------------------------------------------------------------------------------------------------------------------------------------

    #[derive(GodotClass)]
    #[class(base=Object)]
    pub struct CustomConstructor {
        #[var]
        pub val: i64,
    }

    #[godot_api]
    impl CustomConstructor {
        #[func]
        pub fn construct_object(val: i64) -> Gd<CustomConstructor> {
            Gd::from_init_fn(|_base| Self { val })
        }
    }
}

#[itest]
fn custom_constructor_works() {
    let obj = object_test_gd::CustomConstructor::construct_object(42);
    assert_eq!(obj.bind().val, 42);
    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Object)]
struct DoubleUse {
    used: Cell<bool>,
}

#[godot_api]
impl DoubleUse {
    #[func]
    fn use_1(&self) {
        self.used.set(true);
    }
}

/// Test that Godot can call a method that takes `&self`, while there already exists an immutable reference
/// to that type acquired through `bind`.
///
/// This test is not signal-specific, the original bug would happen whenever Godot would call a method that takes `&self`.
#[itest]
fn double_use_reference() {
    let double_use: Gd<DoubleUse> = DoubleUse::alloc_gd();
    let emitter: Gd<ObjPayload> = ObjPayload::alloc_gd();

    emitter
        .clone()
        .upcast::<Object>()
        .connect("do_use".into(), double_use.callable("use_1"));

    let guard = double_use.bind();

    assert!(!guard.used.get());

    emitter
        .clone()
        .upcast::<Object>()
        .emit_signal("do_use".into(), &[]);

    assert!(guard.used.get(), "use_1 was not called");

    drop(guard);

    double_use.free();
    emitter.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

// There isn't a good way to test editor plugins, but we can at least declare one to ensure that the macro
// compiles.
#[cfg(since_api = "4.1")]
#[derive(GodotClass)]
#[class(base = EditorPlugin, editor_plugin)]
struct CustomEditorPlugin;
