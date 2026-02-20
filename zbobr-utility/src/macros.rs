#[macro_export]
macro_rules! config_struct {
    (
        $(#[$struct_attr:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_attr:meta])*
                $field_vis:vis $field:ident : $ty:ty,
            )*
        }
    ) => {
        ::paste::paste! {
            $(#[$struct_attr])*
            #[derive(Debug, Clone, ::serde::Deserialize, Default)]
            #[serde(default, deny_unknown_fields)]
            $vis struct [<$name Toml>] {
                $(
                    $field_vis $field: Option<$ty>,
                )*
            }

            $(#[$struct_attr])*
            #[derive(Debug, Clone, ::clap::Args, Default)]
            $vis struct [<$name Args>] {
                $(
                    $(#[$field_attr])*
                    $field_vis $field: Option<$ty>,
                )*
            }

            impl [<$name Toml>] {
                pub fn merge_with_args(self, args: [<$name Args>]) -> Self {
                    Self {
                        $(
                            $field: args.$field.or(self.$field),
                        )*
                    }
                }
            }
        }
    };
}
