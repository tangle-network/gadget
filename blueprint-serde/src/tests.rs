use crate::from_field;
use crate::ser::new_bounded_string;
use crate::to_field;
use crate::Field;
use serde::{Deserialize, Serialize};
use serde_test::{assert_de_tokens, assert_ser_tokens, Token};
use tangle_subxt::subxt_core::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;

mod structs {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Person {
        name: String,
        age: u8,
    }

    impl Default for Person {
        fn default() -> Self {
            Person {
                name: String::from("John"),
                age: 40,
            }
        }
    }

    impl Person {
        fn as_field(&self) -> Field<AccountId32> {
            let struct_fields = vec![
                (
                    new_bounded_string("name"),
                    Field::String(new_bounded_string(&self.name)),
                ),
                (new_bounded_string("age"), Field::Uint8(self.age)),
            ];

            Field::Struct(
                new_bounded_string("Person"),
                Box::new(BoundedVec(struct_fields)),
            )
        }
    }

    #[test]
    fn test_ser_struct_valid() {
        let person = Person::default();

        assert_ser_tokens(
            &person,
            &[
                Token::Struct {
                    name: "Person",
                    len: 2,
                },
                Token::Str("name"),
                Token::Str("John"),
                Token::Str("age"),
                Token::U8(40),
                Token::StructEnd,
            ],
        );

        let field = to_field(&person).unwrap();
        assert_eq!(field, person.as_field());
    }

    #[test]
    fn test_de_struct_valid() {
        let person = Person::default();

        assert_de_tokens(
            &person,
            &[
                Token::Struct {
                    name: "Person",
                    len: 2,
                },
                Token::Str("name"),
                Token::Str("John"),
                Token::Str("age"),
                Token::U8(40),
                Token::StructEnd,
            ],
        );

        let person_de: Person = from_field(person.as_field()).unwrap();
        assert_eq!(person_de, person);
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct PersonWithFriend {
        name: String,
        age: u8,
        friend: Person,
    }

    impl Default for PersonWithFriend {
        fn default() -> Self {
            PersonWithFriend {
                name: String::from("Matthew"),
                age: 37,
                friend: Person::default(),
            }
        }
    }

    impl PersonWithFriend {
        fn as_field(&self) -> Field<AccountId32> {
            let friend_fields = vec![
                (
                    new_bounded_string("name"),
                    Field::String(new_bounded_string(Person::default().name)),
                ),
                (
                    new_bounded_string("age"),
                    Field::Uint8(Person::default().age),
                ),
            ];

            let person_fields = vec![
                (
                    new_bounded_string("name"),
                    Field::String(new_bounded_string(&self.name)),
                ),
                (new_bounded_string("age"), Field::Uint8(self.age)),
                (
                    new_bounded_string("friend"),
                    Field::Struct(
                        new_bounded_string("Person"),
                        Box::new(BoundedVec(friend_fields)),
                    ),
                ),
            ];

            Field::Struct(
                new_bounded_string("PersonWithFriend"),
                Box::new(BoundedVec(person_fields)),
            )
        }
    }

    #[test]
    fn test_ser_struct_nested() {
        let person_with_friend = PersonWithFriend::default();

        assert_ser_tokens(
            &person_with_friend,
            &[
                Token::Struct {
                    name: "PersonWithFriend",
                    len: 3,
                },
                Token::Str("name"),
                Token::Str("Matthew"),
                Token::Str("age"),
                Token::U8(37),
                Token::Str("friend"),
                Token::Struct {
                    name: "Person",
                    len: 2,
                },
                Token::Str("name"),
                Token::Str("John"),
                Token::Str("age"),
                Token::U8(40),
                Token::StructEnd,
                Token::StructEnd,
            ],
        );

        let field = to_field(&person_with_friend).unwrap();
        assert_eq!(field, person_with_friend.as_field());
    }

    #[test]
    fn test_de_struct_nested() {
        let person_with_friend = PersonWithFriend::default();

        assert_de_tokens(
            &person_with_friend,
            &[
                Token::Struct {
                    name: "PersonWithFriend",
                    len: 3,
                },
                Token::Str("name"),
                Token::Str("Matthew"),
                Token::Str("age"),
                Token::U8(37),
                Token::Str("friend"),
                Token::Struct {
                    name: "Person",
                    len: 2,
                },
                Token::Str("name"),
                Token::Str("John"),
                Token::Str("age"),
                Token::U8(40),
                Token::StructEnd,
                Token::StructEnd,
            ],
        );

        let person_with_friend_de: PersonWithFriend =
            from_field(person_with_friend.as_field()).unwrap();
        assert_eq!(person_with_friend_de, person_with_friend);
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct PersonTuple(String, u8);

    impl Default for PersonTuple {
        fn default() -> Self {
            let person = Person::default();
            PersonTuple(person.name, person.age)
        }
    }

    impl PersonTuple {
        fn as_field(&self) -> Field<AccountId32> {
            let fields = vec![
                (
                    new_bounded_string("field_0"),
                    Field::String(new_bounded_string(self.0.clone())),
                ),
                (new_bounded_string("field_1"), Field::Uint8(self.1)),
            ];

            Field::Struct(
                new_bounded_string("PersonTuple"),
                Box::new(BoundedVec(fields)),
            )
        }
    }

    #[test]
    fn test_ser_struct_tuple() {
        let person_tuple = PersonTuple::default();

        assert_ser_tokens(
            &person_tuple,
            &[
                Token::TupleStruct {
                    name: "PersonTuple",
                    len: 2,
                },
                Token::Str("John"),
                Token::U8(40),
                Token::TupleStructEnd,
            ],
        );

        let field = to_field(&person_tuple).unwrap();
        assert_eq!(field, person_tuple.as_field());
    }

    #[test]
    fn test_de_struct_tuple() {
        let person_tuple = PersonTuple::default();

        assert_de_tokens(
            &person_tuple,
            &[
                Token::TupleStruct {
                    name: "PersonTuple",
                    len: 2,
                },
                Token::Str("John"),
                Token::U8(40),
                Token::TupleStructEnd,
            ],
        );

        let person_tuple_de: PersonTuple = from_field(person_tuple.as_field()).unwrap();
        assert_eq!(person_tuple_de, person_tuple);
    }
}

mod primitives {
    use super::*;

    macro_rules! test_primitive {
        ($($t:ty => $val:literal, $token:path, $field:path);+ $(;)?) => {
            $(
            paste::paste! {
                #[test]
                fn [<test_ser_ $t>]() {
                    let val: $t = $val;

                    assert_ser_tokens(
                        &val,
                        &[
                            $token($val)
                        ],
                    );

                    let field = to_field(&val).unwrap();
                    assert_eq!(field, $field(val));
                }

                #[test]
                fn [<test_de_ $t>]() {
                    let val: $t = $val;

                    assert_de_tokens(
                        &val,
                        &[
                            $token($val)
                        ],
                    );

                    let val_de: $t = from_field($field(val)).unwrap();
                    assert_eq!(val_de, val);
                }
            }
            )+
        };
    }

    test_primitive!(
        bool => true, Token::Bool, Field::Bool;
        u8   =>    0, Token::U8,   Field::Uint8;
        i8   =>    0, Token::I8,   Field::Int8;
        u16  =>    0, Token::U16,  Field::Uint16;
        i16  =>    0, Token::I16,  Field::Int16;
        u32  =>    0, Token::U32,  Field::Uint32;
        i32  =>    0, Token::I32,  Field::Int32;
        u64  =>    0, Token::U64,  Field::Uint64;
        i64  =>    0, Token::I64,  Field::Int64;
    );
}
