pub mod general;
pub mod minehut;

#[macro_export]
macro_rules! embed {
    ($embed_object:ident {
        $(
            author {
                $(name: ($($author_name_tokens:tt)+))?
                $(icon: ($($author_icon_tokens:tt)+))?
                $(url: ($($author_url_tokens:tt)+))?
            }
        )?
        $(description: ($($description_tokens:tt)+))?
        $(
            field {
                name: ($($field_name_tokens:tt)+)
                value: ($($field_value_tokens:tt)+)
                inline: $field_inline_value:literal;
            }
        )*
        $(color: ($($color_tokens:tt)+))?
    }) => {
        let mut $embed_object = serenity::builder::CreateEmbed::default();
        $(
            $embed_object.author(|__a| {
                $(__a.name($($author_name_tokens)+);)?
                $(__a.icon_url($($author_icon_tokens)+);)?
                $(__a.url($($author_url_tokens)+);)?
                __a
            });
        )?
        $($embed_object.description($($description_tokens)+);)?
        $($embed_object.field($($field_name_tokens)+, $($field_value_tokens)+, $field_inline_value);)*
        $($embed_object.color($($color_tokens)+);)?
    }
}
