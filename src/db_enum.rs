#[allow(unused_macros)]
macro_rules! define_enum {
    (
        $name:ident($existing:literal) {
            $( $item:ident = $alias:literal ,)+
        }
    ) => {
        #[derive(
            Copy,
            Clone,
            Debug,
            derive_more::Display,
            PartialEq,
            Eq,
            Hash,
            diesel_derive_enum::DbEnum,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[ExistingTypePath = $existing]
        pub enum $name {
            $(
                #[db_rename = $alias]
                #[display($alias)]
                #[serde(rename = $alias)]
                $item,
            )+
        }

        impl $name {
            pub fn names() -> &'static [ $name ] {
                static NAMES: &[ $name ] = &[
                    $(
                        $name :: $item,
                    )+
                ];
                return NAMES;
            }

            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        Self::$item => $alias,
                    )+
                }
            }
        }
    };
}

// define_enum! {
//     DbSomeEnumType("crate::schema::sql_types::SomeEnumType") {
//         OptionA = "option_name_in_db",
//     }
// }
