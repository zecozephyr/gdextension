/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::meta::{ConvertError, FromGodot, GodotConvert, ToGodot};
use godot::builtin::{
    dict, Array, Dictionary, GString, Variant, VariantArray, Vector2, Vector2Axis,
};
use godot::engine::{Node, Resource};
use godot::obj::Gd;

use crate::framework::itest;

/// Ensure conversions we define have an associated value, and no underlying rust cause.
#[itest]
fn error_has_value_and_no_cause() {
    let node = Node::new_alloc();
    let errors: Vec<(ConvertError, &'static str)> = vec![
        (
            Variant::nil().try_to::<i64>().unwrap_err(),
            "`nil` -> `i64`",
        ),
        (
            VariantArray::new()
                .to_variant()
                .try_to::<GString>()
                .unwrap_err(),
            "`VariantArray` -> `GString`",
        ),
        (
            VariantArray::new()
                .to_variant()
                .try_to::<Array<i64>>()
                .unwrap_err(),
            "`VariantArray` -> `Array<i64>`",
        ),
        (
            Array::<Gd<Node>>::new()
                .to_variant()
                .try_to::<Array<Gd<Resource>>>()
                .unwrap_err(),
            "`Array<Gd<Node>>` -> `Array<Gd<Resource>>`",
        ),
        (
            node.clone().to_variant().try_to::<f32>().unwrap_err(),
            "`Gd<Node>` -> `f32`",
        ),
        (
            node.clone()
                .to_variant()
                .try_to::<Gd<Resource>>()
                .unwrap_err(),
            "`Gd<Node>` -> `Gd<Resource>`",
        ),
    ];

    for (err, err_str) in errors.into_iter() {
        assert!(
            err.value().is_some(),
            "{err_str} conversion has no value: {err:?}"
        );
        assert!(
            err.cause().is_none(),
            "{err_str} conversion should have no rust cause: {err:?}"
        );
    }

    node.free();
}

/// Check that the value stored in an error is the same as the value we tried to convert.
#[itest]
fn error_maintains_value() {
    // Need to use Debug to check equality here since we get a `dyn Debug` and thus cannot check equality
    // directly.
    let value = i32::MAX;
    let err = Vector2Axis::try_from_godot(value).unwrap_err();
    assert_eq!(format!("{value:?}"), format!("{:?}", err.value().unwrap()));

    let value = i64::MAX;
    let err = value.to_variant().try_to::<i32>().unwrap_err();
    assert_eq!(format!("{value:?}"), format!("{:?}", err.value().unwrap()));

    let value = f64::MAX;
    let err = value.to_variant().try_to::<i32>().unwrap_err();
    assert_eq!(
        format!("{:?}", value.to_variant()),
        format!("{:?}", err.value().unwrap())
    );
}

// Manual implementation of `GodotConvert` and related traits to ensure conversion works.
#[derive(Debug, PartialEq)]
struct Foo {
    a: i32,
    b: f32,
}

impl Foo {
    const MISSING_KEY_A: &'static str = "missing `a` key";
    const MISSING_KEY_B: &'static str = "missing `b` key";
    const TOO_MANY_KEYS: &'static str = "too many keys provided";
}

impl GodotConvert for Foo {
    type Via = Dictionary;
}

impl ToGodot for Foo {
    fn to_godot(&self) -> Self::Via {
        dict! {
            "a": self.a,
            "b": self.b,
        }
    }
}

impl FromGodot for Foo {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        let a = match via.get("a") {
            Some(a) => a,
            None => return Err(ConvertError::with_cause_value(Self::MISSING_KEY_A, via)),
        };

        let b = match via.get("b") {
            Some(b) => b,
            None => return Err(ConvertError::with_cause_value(Self::MISSING_KEY_B, via)),
        };

        if via.len() > 2 {
            return Err(ConvertError::with_cause_value(Self::TOO_MANY_KEYS, via));
        }

        Ok(Self {
            a: a.try_to()?,
            b: b.try_to()?,
        })
    }
}

#[itest]
fn custom_convert_roundtrip() {
    let foo = Foo { a: 10, b: 12.34 };

    let as_dict = foo.to_godot();
    assert_eq!(as_dict.get("a"), Some(foo.a.to_variant()));
    assert_eq!(as_dict.get("b"), Some(foo.b.to_variant()));

    let foo2 = as_dict.to_variant().to::<Foo>();
    assert_eq!(foo, foo2, "from_variant");

    let foo3 = Foo::from_godot(as_dict);
    assert_eq!(foo, foo3, "from_godot");
}

// Ensure all failure states for the `FromGodot` conversion of `Foo` are propagated through the `try_to`
// method of `Variant` as they should be.
#[itest]
fn custom_convert_error_from_variant() {
    let missing_a = dict! {
        "b": -0.001
    };
    let err = missing_a
        .to_variant()
        .try_to::<Foo>()
        .expect_err("should be missing key `a`");
    assert_eq!(err.cause().unwrap().to_string(), Foo::MISSING_KEY_A);

    let missing_b = dict! {
        "a": 58,
    };
    let err = missing_b
        .to_variant()
        .try_to::<Foo>()
        .expect_err("should be missing key `b`");
    assert_eq!(err.cause().unwrap().to_string(), Foo::MISSING_KEY_B);

    let too_many_keys = dict! {
        "a": 12,
        "b": 777.777,
        "c": "bar"
    };
    let err = too_many_keys
        .to_variant()
        .try_to::<Foo>()
        .expect_err("should have too many keys");
    assert_eq!(err.cause().unwrap().to_string(), Foo::TOO_MANY_KEYS);

    let wrong_type_a = dict! {
        "a": "hello",
        "b": 28.41,
    };
    let err = wrong_type_a
        .to_variant()
        .try_to::<Foo>()
        .expect_err("should have wrongly typed key `a`");
    assert!(err.cause().is_none());
    // Need to use Debug to check equality here since we get a `dyn Debug` and thus cannot check equality
    // directly.
    assert_eq!(
        format!("{:?}", err.value().unwrap()),
        format!("{:?}", "hello".to_variant())
    );

    let wrong_type_b = dict! {
        "a": 29,
        "b": Vector2::new(1.0, 23.4),
    };
    let err = wrong_type_b
        .to_variant()
        .try_to::<Foo>()
        .expect_err("should have wrongly typed key `b`");
    assert!(err.cause().is_none());
    assert_eq!(
        format!("{:?}", err.value().unwrap()),
        format!("{:?}", Vector2::new(1.0, 23.4).to_variant())
    );

    let too_big_value = dict! {
        "a": i64::MAX,
        "b": f32::NAN
    };
    let err = too_big_value
        .to_variant()
        .try_to::<Foo>()
        .expect_err("should have too big value for field `a`");
    assert!(err.cause().is_none());
    assert_eq!(
        format!("{:?}", err.value().unwrap()),
        format!("{:?}", i64::MAX)
    );
}
